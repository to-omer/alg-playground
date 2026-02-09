use crate::policy::{LazyMapMonoid, VertexSumAdd};
use crate::traits::{ComponentOps, DynamicForest, PathOps, SubtreeOps, VertexOps};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NodeId(u32);

impl NodeId {
    const NIL: Self = Self(u32::MAX);

    #[inline(always)]
    fn is_nil(self) -> bool {
        self.0 == u32::MAX
    }

    #[inline(always)]
    fn idx(self) -> usize {
        self.0 as usize
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VertexId(u32);

impl VertexId {
    #[inline(always)]
    fn idx(self) -> usize {
        self.0 as usize
    }
}

#[inline(always)]
fn v_id(v: usize) -> VertexId {
    debug_assert!(v < u32::MAX as usize);
    VertexId(v as u32)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeType {
    Compress,
    Rake,
    Edge,
}

struct Fold<P: LazyMapMonoid> {
    path_fwd: P::Agg,
    path_rev: P::Agg,
    all: P::Agg,
    path_v_cnt: u32, // number of real vertices in path aggregate (excluding endpoints)
    all_v_cnt: u32,  // number of real vertices in all aggregate (excluding endpoints)
}

impl<P: LazyMapMonoid> Copy for Fold<P> {}

impl<P: LazyMapMonoid> Clone for Fold<P> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<P: LazyMapMonoid> Fold<P> {
    #[inline(always)]
    fn identity() -> Self {
        Self {
            path_fwd: P::agg_unit(),
            path_rev: P::agg_unit(),
            all: P::agg_unit(),
            path_v_cnt: 0,
            all_v_cnt: 0,
        }
    }

    #[inline(always)]
    fn from_edge_key(edge_key: P::Key) -> Self {
        let a = P::agg_from_key(&edge_key);
        Self {
            path_fwd: a,
            path_rev: a,
            all: a,
            path_v_cnt: 0,
            all_v_cnt: 0,
        }
    }

    #[inline(always)]
    fn compress(left: Self, right: Self, cv_key: P::Key, cv_cnt: u32) -> Self {
        let path_fwd = P::agg_merge(&left.path_fwd, &cv_key, &right.path_fwd);
        let path_rev = if P::REVERSAL_INVARIANT {
            path_fwd
        } else {
            P::agg_merge(&right.path_rev, &cv_key, &left.path_rev)
        };
        Self {
            path_fwd,
            path_rev,
            all: P::agg_merge(&left.all, &cv_key, &right.all),
            path_v_cnt: left
                .path_v_cnt
                .wrapping_add(right.path_v_cnt)
                .wrapping_add(cv_cnt),
            all_v_cnt: left
                .all_v_cnt
                .wrapping_add(right.all_v_cnt)
                .wrapping_add(cv_cnt),
        }
    }

    #[inline(always)]
    fn rake(a: Self, b: Self, bv_key: P::Key, bv_cnt: u32) -> Self {
        Self {
            path_fwd: a.path_fwd,
            path_rev: a.path_rev,
            all: P::agg_merge(&a.all, &bv_key, &b.all),
            path_v_cnt: a.path_v_cnt,
            all_v_cnt: a.all_v_cnt.wrapping_add(b.all_v_cnt).wrapping_add(bv_cnt),
        }
    }

    #[inline(always)]
    fn reverse(mut self) -> Self {
        if !P::REVERSAL_INVARIANT {
            std::mem::swap(&mut self.path_fwd, &mut self.path_rev);
        }
        self
    }
}

#[derive(Clone, Copy)]
struct Vertex<P: LazyMapMonoid> {
    key: P::Key,
    handle: NodeId,
}

#[derive(Clone, Copy)]
struct Node<P: LazyMapMonoid> {
    ch: [NodeId; 2],
    par: NodeId,
    rake: NodeId,
    endpoint: [VertexId; 2],

    fold: Fold<P>,

    // lazy
    lazy_path: P::Act,
    lazy_path_pending: bool,
    lazy_all: P::Act,
    lazy_all_pending: bool,

    // Edge-only payload.
    edge_key: P::Key,

    // For `Compress` nodes, this is the shared (middle) vertex `cv` between the two children.
    // (Used to avoid pushing the left child just to read `endpoint[1]` when propagating lazies.)
    mid: VertexId,

    rev: bool,
    guard: bool,
    ty: NodeType,
}

impl<P: LazyMapMonoid> Node<P> {
    fn new_edge(v: VertexId, u: VertexId, edge_key: P::Key) -> Self {
        Self {
            endpoint: [v, u],
            ch: [NodeId::NIL, NodeId::NIL],
            par: NodeId::NIL,
            rake: NodeId::NIL,
            fold: Fold::<P>::from_edge_key(edge_key),
            lazy_path: P::act_unit(),
            lazy_path_pending: false,
            lazy_all: P::act_unit(),
            lazy_all_pending: false,
            edge_key,
            mid: v_id(0),
            rev: false,
            guard: false,
            ty: NodeType::Edge,
        }
    }

    fn new_compress(left: NodeId, right: NodeId) -> Self {
        Self {
            ch: [left, right],
            par: NodeId::NIL,
            rake: NodeId::NIL,
            endpoint: [v_id(0), v_id(0)],
            fold: Fold::<P>::identity(),
            lazy_path: P::act_unit(),
            lazy_path_pending: false,
            lazy_all: P::act_unit(),
            lazy_all_pending: false,
            edge_key: P::key_unit(),
            mid: v_id(0),
            rev: false,
            guard: false,
            ty: NodeType::Compress,
        }
    }

    fn new_rake(left: NodeId, right: NodeId) -> Self {
        Self {
            ch: [left, right],
            par: NodeId::NIL,
            rake: NodeId::NIL,
            endpoint: [v_id(0), v_id(0)],
            fold: Fold::<P>::identity(),
            lazy_path: P::act_unit(),
            lazy_path_pending: false,
            lazy_all: P::act_unit(),
            lazy_all_pending: false,
            edge_key: P::key_unit(),
            mid: v_id(0),
            rev: false,
            guard: false,
            ty: NodeType::Rake,
        }
    }
}

/// Self-adjusting Top Tree (rake/compress + splay).
///
/// Public API uses vertices `[0, n)`; internally it creates dummy vertices `[n, 2n)`
/// linked to each real vertex to avoid isolated-vertex corner cases.
///
/// Note: `path_apply`/`component_apply` are intended for additive-style actions (e.g. `VertexSumAdd`).
pub struct TopTree<P: LazyMapMonoid = VertexSumAdd> {
    real_n: usize,
    vertices: Vec<Vertex<P>>, // length = 2*real_n (real + dummy)
    nodes: Vec<Node<P>>,
    edges: Vec<Vec<(u32, NodeId)>>, // edges[u] contains (v, edge node id), only for real vertices
}

impl<P: LazyMapMonoid> TopTree<P> {
    pub fn new(values: &[P::Key]) -> Self {
        let n = values.len();
        let mut vertices = Vec::with_capacity(2 * n);
        for &v in values {
            vertices.push(Vertex {
                key: v,
                handle: NodeId::NIL,
            });
        }
        for _ in 0..n {
            vertices.push(Vertex {
                key: P::key_unit(),
                handle: NodeId::NIL,
            });
        }

        let mut this = Self {
            real_n: n,
            vertices,
            nodes: Vec::new(),
            edges: vec![Vec::new(); n],
        };

        // Attach a dummy leaf to each vertex to avoid isolated-vertex edge cases.
        for i in 0..n {
            let d = n + i;
            this.link_internal(v_id(i), v_id(d), P::key_unit(), false);
        }

        this
    }

    /// Reserve additional capacity for the internal node arena.
    ///
    /// This is a performance hint for workloads with many `link` operations.
    pub fn reserve_nodes(&mut self, additional: usize) {
        self.nodes.reserve(additional);
    }

    pub fn len(&self) -> usize {
        self.real_n
    }

    pub fn is_empty(&self) -> bool {
        self.real_n == 0
    }

    #[inline(always)]
    fn is_real_vertex(&self, v: VertexId) -> bool {
        v.idx() < self.real_n
    }

    #[inline(always)]
    fn v_weight(&self, v: VertexId) -> u32 {
        u32::from(self.is_real_vertex(v))
    }

    #[inline(always)]
    fn node(&self, x: NodeId) -> &Node<P> {
        debug_assert!(!x.is_nil());
        if cfg!(debug_assertions) {
            &self.nodes[x.idx()]
        } else {
            // SAFETY: `NodeId` values are only created from valid indices and `NIL` is checked.
            unsafe { self.nodes.get_unchecked(x.idx()) }
        }
    }

    #[inline(always)]
    fn node_mut(&mut self, x: NodeId) -> &mut Node<P> {
        debug_assert!(!x.is_nil());
        if cfg!(debug_assertions) {
            &mut self.nodes[x.idx()]
        } else {
            // SAFETY: `NodeId` values are only created from valid indices and `NIL` is checked.
            unsafe { self.nodes.get_unchecked_mut(x.idx()) }
        }
    }

    #[inline(always)]
    fn vertex(&self, v: VertexId) -> &Vertex<P> {
        debug_assert!(v.idx() < self.vertices.len());
        if cfg!(debug_assertions) {
            &self.vertices[v.idx()]
        } else {
            // SAFETY: `VertexId` values are only created from valid indices.
            unsafe { self.vertices.get_unchecked(v.idx()) }
        }
    }

    #[inline(always)]
    fn vertex_mut(&mut self, v: VertexId) -> &mut Vertex<P> {
        debug_assert!(v.idx() < self.vertices.len());
        if cfg!(debug_assertions) {
            &mut self.vertices[v.idx()]
        } else {
            // SAFETY: `VertexId` values are only created from valid indices.
            unsafe { self.vertices.get_unchecked_mut(v.idx()) }
        }
    }

    #[inline(always)]
    fn value(&self, v: VertexId) -> P::Key {
        self.vertex(v).key
    }

    #[inline(always)]
    fn set_value(&mut self, v: VertexId, key: P::Key) {
        self.vertex_mut(v).key = key;
    }

    fn set_handle(&mut self, v: VertexId, h: NodeId) {
        self.vertex_mut(v).handle = h;
    }

    fn parent_dir(&self, child: NodeId) -> i32 {
        let par = self.node(child).par;
        if par.is_nil() {
            return -1;
        }
        if self.node(par).guard {
            return -1;
        }
        if self.node(par).ch[0] == child {
            0
        } else if self.node(par).ch[1] == child {
            1
        } else {
            -1
        }
    }

    fn parent_dir_guard(&self, child: NodeId) -> i32 {
        let par = self.node(child).par;
        if par.is_nil() {
            return -1;
        }
        if self.node(par).ch[0] == child {
            0
        } else if self.node(par).ch[1] == child {
            1
        } else {
            -1
        }
    }

    fn reverse_node(&mut self, x: NodeId) {
        if x.is_nil() {
            return;
        }
        let ty = self.node(x).ty;
        match ty {
            NodeType::Edge => {
                let node = self.node_mut(x);
                node.endpoint.swap(0, 1);
                node.fold = node.fold.reverse();
            }
            NodeType::Compress => {
                let node = self.node_mut(x);
                node.endpoint.swap(0, 1);
                node.fold = node.fold.reverse();
                node.rev ^= true;
            }
            NodeType::Rake => {}
        }
    }

    fn apply_all(&mut self, x: NodeId, act: P::Act) {
        if x.is_nil() {
            return;
        }
        let (path_len, all_len) = {
            let f = &self.node(x).fold;
            (f.path_v_cnt as usize, f.all_v_cnt as usize)
        };
        let nx = self.node_mut(x);
        let new_fwd = P::act_apply_agg(&nx.fold.path_fwd, &act, path_len);
        nx.fold.path_fwd = new_fwd;
        if P::REVERSAL_INVARIANT {
            nx.fold.path_rev = new_fwd;
        } else {
            nx.fold.path_rev = P::act_apply_agg(&nx.fold.path_rev, &act, path_len);
        }
        nx.fold.all = P::act_apply_agg(&nx.fold.all, &act, all_len);
        if nx.lazy_all_pending {
            nx.lazy_all = P::act_compose(&act, &nx.lazy_all);
        } else {
            nx.lazy_all = act;
            nx.lazy_all_pending = true;
        }
    }

    fn apply_path(&mut self, x: NodeId, act: P::Act) {
        if x.is_nil() {
            return;
        }
        let path_len = self.node(x).fold.path_v_cnt as usize;
        let nx = self.node_mut(x);
        let new_fwd = P::act_apply_agg(&nx.fold.path_fwd, &act, path_len);
        nx.fold.path_fwd = new_fwd;
        if P::REVERSAL_INVARIANT {
            nx.fold.path_rev = new_fwd;
        } else {
            nx.fold.path_rev = P::act_apply_agg(&nx.fold.path_rev, &act, path_len);
        }
        nx.fold.all = P::act_apply_agg(&nx.fold.all, &act, path_len);
        if nx.lazy_path_pending {
            nx.lazy_path = P::act_compose(&act, &nx.lazy_path);
        } else {
            nx.lazy_path = act;
            nx.lazy_path_pending = true;
        }
    }

    fn apply_act_to_vertex_if_real(&mut self, v: VertexId, act: P::Act) {
        if !self.is_real_vertex(v) {
            return;
        }
        let key = self.value(v);
        self.set_value(v, P::act_apply_key(&key, &act));
    }

    fn push(&mut self, x: NodeId) {
        if x.is_nil() {
            return;
        }

        // rev handling (compress only)
        let (ty, rev, l, r) = {
            let node = self.node(x);
            (node.ty, node.rev, node.ch[0], node.ch[1])
        };
        if ty == NodeType::Compress && rev {
            self.node_mut(x).ch.swap(0, 1);
            self.reverse_node(l);
            self.reverse_node(r);
            self.node_mut(x).rev = false;
        }

        // lazy_all first
        let (all_pending, all_act) = {
            let nx = self.node(x);
            (nx.lazy_all_pending, nx.lazy_all)
        };
        if all_pending {
            let (l, r, rake) = {
                let nx = self.node(x);
                (nx.ch[0], nx.ch[1], nx.rake)
            };
            self.apply_all(l, all_act);
            self.apply_all(r, all_act);
            self.apply_all(rake, all_act);

            match self.node(x).ty {
                NodeType::Compress => {
                    let cv = self.node(x).mid;
                    self.apply_act_to_vertex_if_real(cv, all_act);
                }
                NodeType::Rake => {
                    let b = self.node(x).ch[1];
                    debug_assert!(!b.is_nil());
                    let bv = self.node(b).endpoint[0];
                    self.apply_act_to_vertex_if_real(bv, all_act);
                }
                NodeType::Edge => {}
            }

            let nx = self.node_mut(x);
            nx.lazy_all = P::act_unit();
            nx.lazy_all_pending = false;
        }

        // lazy_path
        let (path_pending, path_act) = {
            let nx = self.node(x);
            (nx.lazy_path_pending, nx.lazy_path)
        };
        if path_pending {
            let (l, r) = {
                let nx = self.node(x);
                (nx.ch[0], nx.ch[1])
            };
            self.apply_path(l, path_act);
            self.apply_path(r, path_act);

            if self.node(x).ty == NodeType::Compress {
                let cv = self.node(x).mid;
                self.apply_act_to_vertex_if_real(cv, path_act);
            }

            let nx = self.node_mut(x);
            nx.lazy_path = P::act_unit();
            nx.lazy_path_pending = false;
        }
    }

    fn fix(&mut self, x: NodeId) {
        if x.is_nil() {
            return;
        }
        let ty = self.node(x).ty;
        match ty {
            NodeType::Edge => {
                let par = self.node(x).par;
                if par.is_nil() {
                    let [a, b] = self.node(x).endpoint;
                    self.set_handle(a, x);
                    self.set_handle(b, x);
                } else if self.node(par).ty == NodeType::Compress {
                    if self.parent_dir(x) == -1 {
                        let a = self.node(x).endpoint[0];
                        self.set_handle(a, x);
                    }
                } else if self.node(par).ty == NodeType::Rake {
                    let a = self.node(x).endpoint[0];
                    self.set_handle(a, x);
                }
            }
            NodeType::Compress => {
                self.push(x);
                let l = self.node(x).ch[0];
                let r = self.node(x).ch[1];
                debug_assert!(!l.is_nil() && !r.is_nil());

                let l0 = self.node(l).endpoint[0];
                let l1 = self.node(l).endpoint[1];
                let r0 = self.node(r).endpoint[0];
                let r1 = self.node(r).endpoint[1];
                debug_assert_eq!(l1, r0);

                self.node_mut(x).endpoint[0] = l0;
                self.node_mut(x).endpoint[1] = r1;
                self.node_mut(x).mid = l1;

                let mut left_fold = self.node(l).fold;
                let rake = self.node(x).rake;
                if !rake.is_nil() {
                    left_fold = Fold::<P>::rake(
                        self.node(l).fold,
                        self.node(rake).fold,
                        self.value(self.node(rake).endpoint[0]),
                        self.v_weight(self.node(rake).endpoint[0]),
                    );
                }

                let cv_key = self.value(l1);
                let cv_cnt = self.v_weight(l1);
                self.node_mut(x).fold =
                    Fold::<P>::compress(left_fold, self.node(r).fold, cv_key, cv_cnt);

                // Middle vertex becomes a boundary for this cluster.
                self.set_handle(l1, x);

                let par = self.node(x).par;
                if par.is_nil() {
                    let [a, b] = self.node(x).endpoint;
                    self.set_handle(a, x);
                    self.set_handle(b, x);
                } else if self.node(par).ty == NodeType::Compress {
                    if self.parent_dir(x) == -1 {
                        let a = self.node(x).endpoint[0];
                        self.set_handle(a, x);
                    }
                } else if self.node(par).ty == NodeType::Rake {
                    let a = self.node(x).endpoint[0];
                    self.set_handle(a, x);
                }
            }
            NodeType::Rake => {
                self.push(x);
                let a = self.node(x).ch[0];
                let b = self.node(x).ch[1];
                debug_assert!(!a.is_nil() && !b.is_nil());
                self.node_mut(x).endpoint[0] = self.node(a).endpoint[0];
                self.node_mut(x).endpoint[1] = self.node(a).endpoint[1];
                let bv = self.node(b).endpoint[0];
                self.node_mut(x).fold = Fold::<P>::rake(
                    self.node(a).fold,
                    self.node(b).fold,
                    self.value(bv),
                    self.v_weight(bv),
                );
            }
        }
    }

    fn rotate(&mut self, t: NodeId, x: NodeId, dir: usize) {
        let y = self.node(x).par;
        let par_dir = self.parent_dir_guard(x);

        let td = self.node(t).ch[dir];
        self.push(td);

        self.node_mut(x).ch[dir ^ 1] = td;
        if !td.is_nil() {
            self.node_mut(td).par = x;
        }

        self.node_mut(t).ch[dir] = x;
        self.node_mut(x).par = t;

        self.node_mut(t).par = y;
        if par_dir != -1 {
            let pdir = par_dir as usize;
            self.node_mut(y).ch[pdir] = t;
        } else if !y.is_nil() && self.node(y).ty == NodeType::Compress {
            self.node_mut(y).rake = t;
        }

        self.fix(x);
        self.fix(t);
        if !y.is_nil() && !self.node(y).guard {
            self.fix(y);
        }
    }

    fn splay(&mut self, t: NodeId) {
        debug_assert!(self.node(t).ty != NodeType::Edge);
        self.push(t);

        while self.parent_dir(t) != -1 {
            let q = self.node(t).par;
            if self.node(q).ty != self.node(t).ty {
                break;
            }
            if self.parent_dir(q) != -1 {
                let r = self.node(q).par;
                if !r.is_nil() && self.node(r).ty == self.node(q).ty {
                    let rp = self.node(r).par;
                    if !rp.is_nil() {
                        self.push(rp);
                    }
                    self.push(r);
                    self.push(q);
                    self.push(t);
                    let qt_dir = self.parent_dir(t);
                    let rq_dir = self.parent_dir(q);
                    if rq_dir == qt_dir {
                        self.rotate(q, r, (rq_dir as usize) ^ 1);
                        self.rotate(t, q, (qt_dir as usize) ^ 1);
                    } else {
                        self.rotate(t, q, (qt_dir as usize) ^ 1);
                        self.rotate(t, r, (rq_dir as usize) ^ 1);
                    }
                    continue;
                }
            }

            let qp = self.node(q).par;
            if !qp.is_nil() {
                self.push(qp);
            }
            self.push(q);
            self.push(t);
            let qt_dir = self.parent_dir(t);
            self.rotate(t, q, (qt_dir as usize) ^ 1);
        }
    }

    fn expose_raw(&mut self, mut t: NodeId) -> NodeId {
        loop {
            debug_assert!(self.node(t).ty != NodeType::Rake);
            if self.node(t).ty == NodeType::Compress {
                self.splay(t);
            }

            let par = self.node(t).par;
            let n = if par.is_nil() {
                break;
            } else if self.node(par).ty == NodeType::Rake {
                self.push(par);
                self.splay(par);
                self.node(par).par
            } else if self.node(par).ty == NodeType::Compress {
                self.push(par);
                if self.node(par).guard && self.parent_dir_guard(t) != -1 {
                    break;
                }
                par
            } else {
                unreachable!("invalid parent type");
            };

            self.splay(n);

            let mut dir = self.parent_dir_guard(n);
            if dir == -1 {
                dir = 0;
            } else {
                let np = self.node(n).par;
                if !np.is_nil() && self.node(np).ty == NodeType::Rake {
                    dir = 0;
                }
            }
            let dir = dir as usize;

            if dir == 1 {
                let child = self.node(n).ch[dir];
                self.reverse_node(child);
                self.push(child);
                self.reverse_node(t);
                self.push(t);
            }

            let n_dir = self.parent_dir(t);
            if n_dir != -1 {
                let nch = self.node(n).ch[dir];
                self.push(nch);
                let rake = self.node(t).par;
                self.push(rake);

                self.node_mut(rake).ch[n_dir as usize] = nch;
                if !nch.is_nil() {
                    self.node_mut(nch).par = rake;
                }
                self.node_mut(n).ch[dir] = t;
                self.node_mut(t).par = n;

                self.fix(nch);
                self.fix(rake);
                self.fix(t);
                self.fix(n);

                self.splay(rake);
            } else {
                let nch = self.node(n).ch[dir];
                self.push(nch);

                self.node_mut(n).rake = nch;
                if !nch.is_nil() {
                    self.node_mut(nch).par = n;
                }
                self.node_mut(n).ch[dir] = t;
                self.node_mut(t).par = n;

                self.fix(nch);
                self.fix(t);
                self.fix(n);
            }

            if self.node(t).ty == NodeType::Edge {
                t = n;
            }
        }

        t
    }

    fn expose(&mut self, v: VertexId) -> NodeId {
        let h = self.vertex(v).handle;
        debug_assert!(!h.is_nil());
        self.expose_raw(h)
    }

    fn soft_expose(&mut self, v: VertexId, u: VertexId) {
        let root = self.expose(v);
        if self.vertex(v).handle == self.vertex(u).handle {
            if self.node(root).endpoint[1] == v || self.node(root).endpoint[0] == u {
                self.reverse_node(root);
                self.push(root);
            }
            return;
        }
        self.node_mut(root).guard = true;
        let soot = self.expose(u);
        self.node_mut(root).guard = false;
        self.fix(root);
        if self.parent_dir(soot) == 0 {
            self.reverse_node(root);
            self.push(root);
        }
    }

    fn new_edge_node(&mut self, v: VertexId, u: VertexId, edge_key: P::Key) -> NodeId {
        debug_assert!(self.nodes.len() < u32::MAX as usize);
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(Node::<P>::new_edge(v, u, edge_key));
        self.fix(id);
        id
    }

    fn new_compress_node(&mut self, left: NodeId, right: NodeId) -> NodeId {
        debug_assert!(self.nodes.len() < u32::MAX as usize);
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(Node::<P>::new_compress(left, right));
        self.fix(id);
        id
    }

    fn new_rake_node(&mut self, left: NodeId, right: NodeId) -> NodeId {
        debug_assert!(self.nodes.len() < u32::MAX as usize);
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(Node::<P>::new_rake(left, right));
        self.fix(id);
        id
    }

    fn link_internal(
        &mut self,
        v: VertexId,
        u: VertexId,
        edge_key: P::Key,
        record_edge: bool,
    ) -> NodeId {
        let nnu = self.vertex(u).handle;
        let nnv = self.vertex(v).handle;

        let e = self.new_edge_node(v, u, edge_key);

        if record_edge && self.is_real_vertex(v) && self.is_real_vertex(u) {
            let vv = v.idx() as u32;
            let uu = u.idx() as u32;
            self.edges[v.idx()].push((uu, e));
            self.edges[u.idx()].push((vv, e));
        }

        if nnu.is_nil() && nnv.is_nil() {
            return e;
        }

        let left = if nnu.is_nil() {
            e
        } else {
            let uu = self.expose_raw(nnu);
            self.push(uu);
            if self.node(uu).endpoint[1] == u {
                self.reverse_node(uu);
                self.push(uu);
            }
            if self.node(uu).endpoint[0] == u {
                let nu = self.new_compress_node(e, uu);
                self.node_mut(e).par = nu;
                self.fix(e);
                self.node_mut(uu).par = nu;
                self.fix(uu);
                self.fix(nu);
                nu
            } else {
                let nu = uu;
                let left_ch = self.node(nu).ch[0];
                self.push(left_ch);

                self.node_mut(nu).ch[0] = e;
                self.node_mut(e).par = nu;
                self.fix(e);

                let beta = self.node(nu).rake;
                let rake = if !beta.is_nil() {
                    self.push(beta);
                    let r = self.new_rake_node(beta, left_ch);
                    self.node_mut(beta).par = r;
                    if !left_ch.is_nil() {
                        self.node_mut(left_ch).par = r;
                    }
                    self.fix(beta);
                    self.fix(left_ch);
                    r
                } else {
                    left_ch
                };
                self.node_mut(nu).rake = rake;
                if !rake.is_nil() {
                    self.node_mut(rake).par = nu;
                    self.fix(rake);
                }
                self.fix(nu);
                nu
            }
        };

        if nnv.is_nil() {
            // v was isolated.
        } else {
            let vv = self.expose_raw(nnv);
            self.push(vv);
            if self.node(vv).endpoint[0] == v {
                self.reverse_node(vv);
                self.push(vv);
            }
            if self.node(vv).endpoint[1] == v {
                let top = self.new_compress_node(vv, left);
                self.node_mut(vv).par = top;
                if !left.is_nil() {
                    self.node_mut(left).par = top;
                }
                self.fix(vv);
                self.fix(left);
                self.fix(top);
            } else {
                let nv = vv;
                let right_ch = self.node(nv).ch[1];
                self.reverse_node(right_ch);
                self.push(right_ch);

                self.node_mut(nv).ch[1] = left;
                if !left.is_nil() {
                    self.node_mut(left).par = nv;
                }
                self.fix(left);

                let alpha = self.node(nv).rake;
                let rake = if !alpha.is_nil() {
                    self.push(alpha);
                    let r = self.new_rake_node(alpha, right_ch);
                    self.node_mut(alpha).par = r;
                    self.fix(alpha);
                    if !right_ch.is_nil() {
                        self.node_mut(right_ch).par = r;
                    }
                    self.fix(right_ch);
                    r
                } else {
                    right_ch
                };
                self.node_mut(nv).rake = rake;
                if !rake.is_nil() {
                    self.node_mut(rake).par = nv;
                    self.fix(rake);
                }
                self.fix(nv);
            }
        }

        e
    }

    fn bring(&mut self, root: NodeId) {
        if root.is_nil() {
            return;
        }
        let rake = self.node(root).rake;
        if rake.is_nil() {
            let left = self.node(root).ch[0];
            if !left.is_nil() {
                self.node_mut(left).par = NodeId::NIL;
                self.fix(left);
            }
            return;
        }

        match self.node(rake).ty {
            NodeType::Compress | NodeType::Edge => {
                self.push(rake);
                let new_right = rake;
                self.reverse_node(new_right);
                self.push(new_right);

                self.node_mut(root).ch[1] = new_right;
                self.node_mut(new_right).par = root;
                self.node_mut(root).rake = NodeId::NIL;

                self.fix(new_right);
                self.fix(root);
            }
            NodeType::Rake => {
                let mut r = rake;
                self.push(r);
                loop {
                    let next = self.node(r).ch[1];
                    if next.is_nil() || self.node(next).ty != NodeType::Rake {
                        break;
                    }
                    self.push(next);
                    r = next;
                }

                self.node_mut(root).guard = true;
                self.splay(r);
                self.node_mut(root).guard = false;

                let new_rake = self.node(r).ch[0];
                let new_right = self.node(r).ch[1];

                self.reverse_node(new_right);
                self.push(new_right);

                self.node_mut(root).ch[1] = new_right;
                if !new_right.is_nil() {
                    self.node_mut(new_right).par = root;
                }

                self.node_mut(root).rake = new_rake;
                if !new_rake.is_nil() {
                    self.node_mut(new_rake).par = root;
                }

                self.fix(new_rake);
                self.fix(new_right);
                self.fix(root);
            }
        }
    }

    fn cut_internal(&mut self, v: VertexId, u: VertexId) {
        self.soft_expose(v, u);
        let root = self.vertex(v).handle;
        self.push(root);
        let right = self.node(root).ch[1];
        if !right.is_nil() {
            self.node_mut(right).par = NodeId::NIL;
            self.reverse_node(right);
            self.push(right);
        }
        self.bring(right);
        self.bring(root);
    }

    fn path_query_node(&mut self, v: VertexId, u: VertexId) -> NodeId {
        self.soft_expose(v, u);
        let root = self.vertex(v).handle;
        self.push(root);
        if self.node(root).endpoint[0] == v && self.node(root).endpoint[1] == u {
            return root;
        }
        if self.node(root).endpoint[0] == v {
            return self.node(root).ch[0];
        }
        if self.node(root).endpoint[1] == u {
            return self.node(root).ch[1];
        }
        let right = self.node(root).ch[1];
        self.push(right);
        self.node(right).ch[0]
    }

    fn connected_internal(&self, u: VertexId, v: VertexId) -> bool {
        if u == v {
            return true;
        }
        let hu = self.vertex(u).handle;
        let hv = self.vertex(v).handle;
        if hu.is_nil() || hv.is_nil() {
            return false;
        }
        let mut ru = hu;
        while !self.node(ru).par.is_nil() {
            ru = self.node(ru).par;
        }
        let mut rv = hv;
        while !self.node(rv).par.is_nil() {
            rv = self.node(rv).par;
        }
        ru == rv
    }

    fn remove_edge_entry(&mut self, u: usize, v: usize) -> Option<NodeId> {
        let list = &mut self.edges[u];
        let vv = v as u32;
        let i = list.iter().position(|&(to, _)| to == vv)?;
        let (_to, id) = list.swap_remove(i);
        Some(id)
    }

    fn remove_edge_both(&mut self, u: usize, v: usize) -> Option<NodeId> {
        let e1 = self.remove_edge_entry(u, v)?;
        let e2 = self.remove_edge_entry(v, u)?;
        debug_assert_eq!(e1, e2);
        Some(e1)
    }

    fn find_edge_node(&self, u: usize, v: usize) -> Option<NodeId> {
        let list = &self.edges[u];
        let vv = v as u32;
        list.iter()
            .find_map(|&(to, id)| if to == vv { Some(id) } else { None })
    }

    fn fix_upwards(&mut self, mut x: NodeId) {
        while !x.is_nil() {
            self.fix(x);
            x = self.node(x).par;
        }
    }

    fn cut_with_edge_key(&mut self, u: usize, v: usize) -> Option<P::Key> {
        let e = self.remove_edge_both(u, v)?;
        let w = self.node(e).edge_key;
        self.cut_internal(v_id(u), v_id(v));
        Some(w)
    }

    fn kth_on_path_internal(&mut self, mut x: NodeId, mut k: u32) -> VertexId {
        // k is 0-indexed among internal vertices (excluding endpoints) in the *stored direction*.
        loop {
            self.push(x);
            match self.node(x).ty {
                NodeType::Edge => unreachable!("no internal vertices in an edge"),
                NodeType::Rake => {
                    x = self.node(x).ch[0];
                }
                NodeType::Compress => {
                    let l = self.node(x).ch[0];
                    let r = self.node(x).ch[1];
                    self.push(l);
                    let lcnt = self.node(l).fold.path_v_cnt;
                    if k < lcnt {
                        x = l;
                        continue;
                    }
                    if k == lcnt {
                        let m = self.node(l).endpoint[1];
                        debug_assert!(self.is_real_vertex(m));
                        return m;
                    }
                    k = k.wrapping_sub(lcnt.wrapping_add(1));
                    x = r;
                }
            }
        }
    }

    pub fn connected(&mut self, u: usize, v: usize) -> bool {
        debug_assert!(u < self.real_n && v < self.real_n);
        self.connected_internal(v_id(u), v_id(v))
    }

    pub fn link(&mut self, u: usize, v: usize) -> bool {
        self.link_with_edge(u, v, P::key_unit())
    }

    pub fn link_with_edge(&mut self, u: usize, v: usize, w: P::Key) -> bool {
        debug_assert!(u < self.real_n && v < self.real_n);
        if u == v {
            return false;
        }
        let uid = v_id(u);
        let vid = v_id(v);
        if self.connected_internal(uid, vid) {
            return false;
        }
        self.link_internal(uid, vid, w, true);
        true
    }

    pub fn cut(&mut self, u: usize, v: usize) -> bool {
        debug_assert!(u < self.real_n && v < self.real_n);
        if u == v {
            return false;
        }
        if self.remove_edge_both(u, v).is_none() {
            return false;
        }
        self.cut_internal(v_id(u), v_id(v));
        true
    }

    pub fn edge_get(&mut self, u: usize, v: usize) -> Option<P::Key> {
        debug_assert!(u < self.real_n && v < self.real_n);
        let e = self.find_edge_node(u, v)?;
        Some(self.node(e).edge_key)
    }

    pub fn edge_set(&mut self, u: usize, v: usize, w: P::Key) -> bool {
        debug_assert!(u < self.real_n && v < self.real_n);
        let Some(e) = self.find_edge_node(u, v) else {
            return false;
        };
        let nx = self.node_mut(e);
        nx.edge_key = w;
        nx.fold = Fold::<P>::from_edge_key(w);
        self.fix_upwards(e);
        true
    }

    pub fn edge_apply(&mut self, u: usize, v: usize, act: P::Act) -> bool {
        debug_assert!(u < self.real_n && v < self.real_n);
        let Some(e) = self.find_edge_node(u, v) else {
            return false;
        };
        let w = self.node(e).edge_key;
        let w2 = P::act_apply_key(&w, &act);
        let nx = self.node_mut(e);
        nx.edge_key = w2;
        nx.fold = Fold::<P>::from_edge_key(w2);
        self.fix_upwards(e);
        true
    }

    pub fn makeroot(&mut self, v: usize) {
        debug_assert!(v < self.real_n);
        let vid = v_id(v);
        let root = self.expose(vid);
        self.push(root);
        if self.node(root).endpoint[1] == vid {
            self.reverse_node(root);
            self.push(root);
        }
    }

    pub fn find_root(&mut self, v: usize) -> usize {
        debug_assert!(v < self.real_n);
        let vid = v_id(v);
        let root = self.expose(vid);
        self.push(root);
        let a = self.node(root).endpoint[0];
        let b = self.node(root).endpoint[1];
        if self.is_real_vertex(a) {
            a.idx()
        } else {
            debug_assert!(self.is_real_vertex(b));
            b.idx()
        }
    }

    pub fn vertex_get(&mut self, v: usize) -> P::Key {
        debug_assert!(v < self.real_n);
        let vid = v_id(v);
        let root = self.expose(vid);
        self.push(root);
        self.value(vid)
    }

    pub fn vertex_set(&mut self, v: usize, key: P::Key) {
        debug_assert!(v < self.real_n);
        let vid = v_id(v);
        let root = self.expose(vid);
        self.push(root);
        self.set_value(vid, key);
        self.fix(root);
    }

    pub fn vertex_apply(&mut self, v: usize, act: P::Act) {
        debug_assert!(v < self.real_n);
        let vid = v_id(v);
        let root = self.expose(vid);
        self.push(root);
        self.apply_act_to_vertex_if_real(vid, act);
        self.fix(root);
    }

    pub fn component_fold(&mut self, v: usize) -> P::Agg {
        debug_assert!(v < self.real_n);
        let vid = v_id(v);
        let root = self.expose(vid);
        self.push(root);
        let a = self.node(root).endpoint[0];
        let b = self.node(root).endpoint[1];
        let mut agg = P::agg_merge(&P::agg_unit(), &self.value(a), &self.node(root).fold.all);
        agg = P::agg_merge(&agg, &self.value(b), &P::agg_unit());
        agg
    }

    pub fn component_apply(&mut self, v: usize, act: P::Act) {
        debug_assert!(v < self.real_n);
        let vid = v_id(v);
        let root = self.expose(vid);
        self.push(root);
        let a = self.node(root).endpoint[0];
        let b = self.node(root).endpoint[1];
        self.apply_all(root, act);
        self.apply_act_to_vertex_if_real(a, act);
        self.apply_act_to_vertex_if_real(b, act);
        self.fix(root);
    }

    pub fn component_size(&mut self, v: usize) -> usize {
        debug_assert!(v < self.real_n);
        let vid = v_id(v);
        let root = self.expose(vid);
        self.push(root);
        let a = self.node(root).endpoint[0];
        let b = self.node(root).endpoint[1];
        (self.node(root).fold.all_v_cnt + self.v_weight(a) + self.v_weight(b)) as usize
    }

    pub fn path_fold(&mut self, u: usize, v: usize) -> Option<P::Agg> {
        debug_assert!(u < self.real_n && v < self.real_n);
        let uid = v_id(u);
        let vid = v_id(v);
        if !self.connected_internal(uid, vid) {
            return None;
        }
        if u == v {
            let key = self.vertex_get(u);
            return Some(P::agg_from_key(&key));
        }

        let p = self.path_query_node(uid, vid);
        self.push(p);
        let internal = if self.node(p).endpoint[0] == uid {
            self.node(p).fold.path_fwd
        } else {
            self.node(p).fold.path_rev
        };
        let mut agg = P::agg_merge(&P::agg_unit(), &self.value(uid), &internal);
        agg = P::agg_merge(&agg, &self.value(vid), &P::agg_unit());
        Some(agg)
    }

    pub fn path_apply(&mut self, u: usize, v: usize, act: P::Act) -> bool {
        debug_assert!(u < self.real_n && v < self.real_n);
        let uid = v_id(u);
        let vid = v_id(v);
        if !self.connected_internal(uid, vid) {
            return false;
        }
        if u == v {
            self.vertex_apply(u, act);
            return true;
        }
        let p = self.path_query_node(uid, vid);
        self.push(p);
        self.apply_path(p, act);
        self.apply_act_to_vertex_if_real(uid, act);
        self.apply_act_to_vertex_if_real(vid, act);
        self.fix_upwards(p);
        true
    }

    pub fn path_len(&mut self, u: usize, v: usize) -> Option<usize> {
        debug_assert!(u < self.real_n && v < self.real_n);
        let uid = v_id(u);
        let vid = v_id(v);
        if !self.connected_internal(uid, vid) {
            return None;
        }
        if u == v {
            return Some(1);
        }
        let p = self.path_query_node(uid, vid);
        self.push(p);
        Some((self.node(p).fold.path_v_cnt + 2) as usize)
    }

    pub fn path_kth(&mut self, u: usize, v: usize, k: usize) -> Option<usize> {
        debug_assert!(u < self.real_n && v < self.real_n);
        let uid = v_id(u);
        let vid = v_id(v);
        if !self.connected_internal(uid, vid) {
            return None;
        }
        if u == v {
            return if k == 0 { Some(u) } else { None };
        }

        let p = self.path_query_node(uid, vid);
        self.push(p);
        let internal_cnt = self.node(p).fold.path_v_cnt;
        let len = internal_cnt as usize + 2;
        if k >= len {
            return None;
        }
        if k == 0 {
            return Some(u);
        }
        if k + 1 == len {
            return Some(v);
        }

        // internal vertex index (excluding endpoints)
        let mut ik = (k - 1) as u32;
        if self.node(p).endpoint[0] != uid {
            // stored direction is v->u; flip
            ik = internal_cnt - 1 - ik;
        }
        let vv = self.kth_on_path_internal(p, ik);
        debug_assert!(self.is_real_vertex(vv));
        Some(vv.idx())
    }

    pub fn subtree_fold(&mut self, child: usize, parent: usize) -> P::Agg {
        debug_assert!(child < self.real_n && parent < self.real_n);
        let w = self
            .cut_with_edge_key(child, parent)
            .expect("subtree_fold requires an existing edge");
        let res = self.component_fold(child);
        let ok = self.link_with_edge(child, parent, w);
        debug_assert!(ok);
        res
    }

    pub fn subtree_apply(&mut self, child: usize, parent: usize, act: P::Act) {
        debug_assert!(child < self.real_n && parent < self.real_n);
        let w = self
            .cut_with_edge_key(child, parent)
            .expect("subtree_apply requires an existing edge");
        self.component_apply(child, act);
        let ok = self.link_with_edge(child, parent, w);
        debug_assert!(ok);
    }
}

impl TopTree<VertexSumAdd> {
    pub fn vertex_add(&mut self, v: usize, delta: i64) {
        self.vertex_apply(v, delta);
    }

    pub fn path_sum(&mut self, u: usize, v: usize) -> Option<i64> {
        self.path_fold(u, v)
    }

    pub fn component_sum(&mut self, v: usize) -> i64 {
        self.component_fold(v)
    }
}

impl<P: LazyMapMonoid> DynamicForest for TopTree<P> {
    type Key = P::Key;

    fn new(values: &[Self::Key]) -> Self {
        Self::new(values)
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn link(&mut self, u: usize, v: usize) -> bool {
        self.link(u, v)
    }

    fn cut(&mut self, u: usize, v: usize) -> bool {
        self.cut(u, v)
    }

    fn connected(&mut self, u: usize, v: usize) -> bool {
        self.connected(u, v)
    }
}

impl<P: LazyMapMonoid> VertexOps for TopTree<P> {
    type Act = P::Act;

    fn vertex_get(&mut self, v: usize) -> Self::Key {
        self.vertex_get(v)
    }

    fn vertex_set(&mut self, v: usize, key: Self::Key) {
        self.vertex_set(v, key)
    }

    fn vertex_apply(&mut self, v: usize, act: Self::Act) {
        self.vertex_apply(v, act)
    }
}

impl<P: LazyMapMonoid> PathOps for TopTree<P> {
    type Agg = P::Agg;
    type Act = P::Act;

    fn makeroot(&mut self, v: usize) {
        self.makeroot(v)
    }

    fn find_root(&mut self, v: usize) -> usize {
        self.find_root(v)
    }

    fn path_fold(&mut self, u: usize, v: usize) -> Option<Self::Agg> {
        self.path_fold(u, v)
    }

    fn path_apply(&mut self, u: usize, v: usize, act: Self::Act) -> bool {
        self.path_apply(u, v, act)
    }

    fn path_len(&mut self, u: usize, v: usize) -> Option<usize> {
        self.path_len(u, v)
    }

    fn path_kth(&mut self, u: usize, v: usize, k: usize) -> Option<usize> {
        self.path_kth(u, v, k)
    }
}

impl<P: LazyMapMonoid> ComponentOps for TopTree<P> {
    type Agg = P::Agg;
    type Act = P::Act;

    fn component_fold(&mut self, v: usize) -> Self::Agg {
        self.component_fold(v)
    }

    fn component_apply(&mut self, v: usize, act: Self::Act) {
        self.component_apply(v, act)
    }

    fn component_size(&mut self, v: usize) -> usize {
        self.component_size(v)
    }
}

impl<P: LazyMapMonoid> SubtreeOps for TopTree<P> {
    type Agg = P::Agg;
    type Act = P::Act;

    fn subtree_fold(&mut self, child: usize, parent: usize) -> Self::Agg {
        self.subtree_fold(child, parent)
    }

    fn subtree_apply(&mut self, child: usize, parent: usize, act: Self::Act) {
        self.subtree_apply(child, parent, act)
    }
}
