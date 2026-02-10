use std::marker::PhantomData;

use crate::policy::{LazyMapMonoid, VertexSumAdd};
use crate::traits::{ComponentOps, DynamicForest, PathOps, SubtreeOps, VertexOps};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Id(u32);

impl Id {
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

#[inline(always)]
fn id(v: usize) -> Id {
    debug_assert!(v < u32::MAX as usize);
    Id(v as u32)
}

#[derive(Clone, Copy, Debug)]
struct Node<P: LazyMapMonoid<Key = i64, Agg = i64, Act = i64>> {
    ch: [Id; 2],
    p: Id,
    sz: u32,
    vir_sz: u32,
    all_sz: u32,

    // Per-vertex value.
    key: P::Key,

    // Path aggregate over the auxiliary (splay) tree: sum of `key` in the splay subtree.
    agg: P::Agg,

    // Virtual (light-edge) aggregate: sum/size of subtrees attached as virtual children.
    //
    // Invariant: `vir_sum` already reflects any pending `vir_lazy` (see below) for the current
    // virtual children set. Individual virtual child roots may still be stale.
    vir_sum: P::Agg,

    // Component aggregate over the auxiliary (splay) subtree: sum/size of all vertices represented
    // by the splay subtree plus its virtual children (recursively).
    all_sum: P::Agg,

    // Lazy "path-only" add over the splay subtree (used by `path_apply`).
    lazy_path: P::Act,

    // Lazy "all" add over the splay subtree (used by `component_apply` and for catching up
    // virtual children).
    //
    // This updates:
    // - `key`/`agg` for splay vertices,
    // - `vir_sum` and `vir_lazy` for virtual subtrees,
    // - `all_sum` for all vertices represented by the splay subtree.
    lazy_all: P::Act,

    // Pending add that should be applied to each current virtual child subtree root when it is
    // promoted (virtual -> preferred) by `access`.
    vir_lazy: P::Act,

    // Snapshot of the parent's `vir_lazy` at the moment this node became a *virtual child*.
    // Used to avoid double-applying "virtual" lazies when edges move between preferred/virtual.
    vir_from_parent: P::Act,

    rev: bool,
    lazy_path_pending: bool,
    lazy_all_pending: bool,
}

impl<P: LazyMapMonoid<Key = i64, Agg = i64, Act = i64>> Node<P> {
    fn new(key: P::Key) -> Self {
        let agg = P::agg_from_key(&key);
        Self {
            ch: [Id::NIL, Id::NIL],
            p: Id::NIL,
            rev: false,
            key,
            agg,
            sz: 1,
            vir_sum: P::agg_unit(),
            vir_sz: 0,
            all_sum: agg,
            all_sz: 1,
            lazy_path: P::act_unit(),
            lazy_path_pending: false,
            lazy_all: P::act_unit(),
            lazy_all_pending: false,
            vir_lazy: P::act_unit(),
            vir_from_parent: P::act_unit(),
        }
    }
}

/// Link-Cut Tree (splay-based) that additionally supports component/subtree aggregates.
///
/// This is a specialized variant that tracks "virtual" (light-edge) contributions and supports
/// `component_fold/component_apply/component_size` and `subtree_*` (via cut+component+link).
///
/// The current subtree/component implementation requires numeric add semantics, so we constrain
/// policies to `Key/Agg/Act = i64`.
pub struct LinkCutTreeSubtree<P: LazyMapMonoid<Key = i64, Agg = i64, Act = i64> = VertexSumAdd> {
    nodes: Vec<Node<P>>,
    stack: Vec<Id>,
    _marker: PhantomData<fn() -> P>,
}

impl<P: LazyMapMonoid<Key = i64, Agg = i64, Act = i64>> LinkCutTreeSubtree<P> {
    pub fn new(values: &[P::Key]) -> Self {
        let mut nodes = Vec::with_capacity(values.len());
        for &v in values {
            debug_assert!(nodes.len() < u32::MAX as usize);
            nodes.push(Node::new(v));
        }
        Self {
            nodes,
            stack: Vec::with_capacity(values.len()),
            _marker: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[inline(always)]
    fn node(&self, x: Id) -> &Node<P> {
        debug_assert!(!x.is_nil());
        debug_assert!(x.idx() < self.nodes.len());
        if cfg!(debug_assertions) {
            &self.nodes[x.idx()]
        } else {
            // SAFETY: `Id` values are only created from valid indices and `NIL` is checked.
            unsafe { self.nodes.get_unchecked(x.idx()) }
        }
    }

    #[inline(always)]
    fn node_mut(&mut self, x: Id) -> &mut Node<P> {
        debug_assert!(!x.is_nil());
        debug_assert!(x.idx() < self.nodes.len());
        if cfg!(debug_assertions) {
            &mut self.nodes[x.idx()]
        } else {
            // SAFETY: `Id` values are only created from valid indices and `NIL` is checked.
            unsafe { self.nodes.get_unchecked_mut(x.idx()) }
        }
    }

    #[inline(always)]
    fn sz(&self, x: Id) -> u32 {
        if x.is_nil() { 0 } else { self.node(x).sz }
    }

    #[inline(always)]
    fn is_aux_root(&self, x: Id) -> bool {
        let p = self.node(x).p;
        if p.is_nil() {
            return true;
        }
        self.node(p).ch[0] != x && self.node(p).ch[1] != x
    }

    #[inline(always)]
    fn apply_rev(&mut self, x: Id) {
        if x.is_nil() {
            return;
        }
        let nx = self.node_mut(x);
        nx.ch.swap(0, 1);
        nx.rev ^= true;
    }

    #[inline(always)]
    fn apply_path_add(&mut self, x: Id, delta: P::Act) {
        if x.is_nil() {
            return;
        }
        let sz = self.node(x).sz as i64;
        let nx = self.node_mut(x);
        nx.key = nx.key.wrapping_add(delta);
        let inc = delta.wrapping_mul(sz);
        nx.agg = nx.agg.wrapping_add(inc);
        nx.all_sum = nx.all_sum.wrapping_add(inc);
        if nx.lazy_path_pending {
            nx.lazy_path = nx.lazy_path.wrapping_add(delta);
        } else {
            nx.lazy_path = delta;
            nx.lazy_path_pending = true;
        }
    }

    #[inline(always)]
    fn apply_all_add(&mut self, x: Id, delta: P::Act) {
        if x.is_nil() {
            return;
        }
        let (sz, all_sz, vir_sz) = {
            let nx = self.node(x);
            (nx.sz as i64, nx.all_sz as i64, nx.vir_sz as i64)
        };
        let nx = self.node_mut(x);
        nx.key = nx.key.wrapping_add(delta);
        nx.agg = nx.agg.wrapping_add(delta.wrapping_mul(sz));
        nx.vir_sum = nx.vir_sum.wrapping_add(delta.wrapping_mul(vir_sz));
        nx.all_sum = nx.all_sum.wrapping_add(delta.wrapping_mul(all_sz));
        nx.vir_lazy = nx.vir_lazy.wrapping_add(delta);
        if nx.lazy_all_pending {
            nx.lazy_all = nx.lazy_all.wrapping_add(delta);
        } else {
            nx.lazy_all = delta;
            nx.lazy_all_pending = true;
        }
    }

    fn push(&mut self, x: Id) {
        if x.is_nil() {
            return;
        }
        let (rev, lazy_all_pending, lazy_all, lazy_path_pending, lazy_path, l, r) = {
            let nx = self.node(x);
            (
                nx.rev,
                nx.lazy_all_pending,
                nx.lazy_all,
                nx.lazy_path_pending,
                nx.lazy_path,
                nx.ch[0],
                nx.ch[1],
            )
        };

        if rev {
            self.apply_rev(l);
            self.apply_rev(r);
            self.node_mut(x).rev = false;
        }

        if lazy_all_pending {
            self.apply_all_add(l, lazy_all);
            self.apply_all_add(r, lazy_all);
            let nx = self.node_mut(x);
            nx.lazy_all = 0;
            nx.lazy_all_pending = false;
        }

        if lazy_path_pending {
            self.apply_path_add(l, lazy_path);
            self.apply_path_add(r, lazy_path);
            let nx = self.node_mut(x);
            nx.lazy_path = 0;
            nx.lazy_path_pending = false;
        }
    }

    fn pull(&mut self, x: Id) {
        if x.is_nil() {
            return;
        }
        let (l, r, key, vir_sum, vir_sz) = {
            let nx = self.node(x);
            (nx.ch[0], nx.ch[1], nx.key, nx.vir_sum, nx.vir_sz)
        };

        let (l_sz, l_agg, l_all_sz, l_all_sum) = if l.is_nil() {
            (0_u32, P::agg_unit(), 0_u32, P::agg_unit())
        } else {
            let nl = self.node(l);
            (nl.sz, nl.agg, nl.all_sz, nl.all_sum)
        };
        let (r_sz, r_agg, r_all_sz, r_all_sum) = if r.is_nil() {
            (0_u32, P::agg_unit(), 0_u32, P::agg_unit())
        } else {
            let nr = self.node(r);
            (nr.sz, nr.agg, nr.all_sz, nr.all_sum)
        };

        let sz = 1_u32.wrapping_add(l_sz).wrapping_add(r_sz);
        let agg = l_agg.wrapping_add(key).wrapping_add(r_agg);

        let all_sz = 1_u32
            .wrapping_add(l_all_sz)
            .wrapping_add(r_all_sz)
            .wrapping_add(vir_sz);
        let all_sum = l_all_sum
            .wrapping_add(key)
            .wrapping_add(r_all_sum)
            .wrapping_add(vir_sum);

        let nx = self.node_mut(x);
        nx.sz = sz;
        nx.agg = agg;
        nx.all_sz = all_sz;
        nx.all_sum = all_sum;
    }

    fn rotate(&mut self, x: Id) {
        let p = self.node(x).p;
        let g = self.node(p).p;
        let p_is_aux_root = self.is_aux_root(p);
        self.push(p);
        self.push(x);

        let dir = usize::from(self.node(p).ch[1] == x);
        let b = self.node(x).ch[dir ^ 1];

        if !self.is_aux_root(p) {
            if self.node(g).ch[0] == p {
                self.node_mut(g).ch[0] = x;
            } else if self.node(g).ch[1] == p {
                self.node_mut(g).ch[1] = x;
            }
        }
        self.node_mut(x).p = g;

        // When rotating at the top of an auxiliary tree, `p.p` is the path-parent pointer and is
        // moved from `p` to `x`. The "virtual child snapshot" must move with it, otherwise later
        // `vir_lazy` diffing will use the wrong baseline.
        if p_is_aux_root {
            let snap = self.node(p).vir_from_parent;
            self.node_mut(x).vir_from_parent = snap;
        }

        self.node_mut(x).ch[dir ^ 1] = p;
        self.node_mut(p).p = x;

        self.node_mut(p).ch[dir] = b;
        if !b.is_nil() {
            self.node_mut(b).p = p;
        }

        self.pull(p);
        self.pull(x);
    }

    fn push_path(&mut self, x: Id) {
        self.stack.clear();
        let mut y = x;
        self.stack.push(y);
        while !self.is_aux_root(y) {
            y = self.node(y).p;
            self.stack.push(y);
        }
        for i in (0..self.stack.len()).rev() {
            let v = self.stack[i];
            self.push(v);
        }
    }

    fn splay(&mut self, x: Id) {
        self.push_path(x);

        while !self.is_aux_root(x) {
            let p = self.node(x).p;
            let g = self.node(p).p;
            if !self.is_aux_root(p) {
                let zigzig = (self.node(g).ch[0] == p) == (self.node(p).ch[0] == x);
                if zigzig {
                    self.rotate(p);
                } else {
                    self.rotate(x);
                }
            }
            self.rotate(x);
        }
    }

    #[inline(always)]
    fn virtual_add_with_meta(&mut self, parent: Id, child_root: Id) -> (P::Agg, u32) {
        if child_root.is_nil() {
            return (P::agg_unit(), 0);
        }
        let (sum, sz) = {
            let nc = self.node(child_root);
            (nc.all_sum, nc.all_sz)
        };
        let parent_vir_lazy = self.node(parent).vir_lazy;
        {
            let np = self.node_mut(parent);
            np.vir_sum = np.vir_sum.wrapping_add(sum);
            np.vir_sz = np.vir_sz.wrapping_add(sz);
        }
        self.node_mut(child_root).vir_from_parent = parent_vir_lazy;
        (sum, sz)
    }

    fn virtual_add(&mut self, parent: Id, child_root: Id) {
        if child_root.is_nil() {
            return;
        }
        let (sum, sz) = {
            let nc = self.node(child_root);
            (nc.all_sum, nc.all_sz)
        };
        let parent_vir_lazy = self.node(parent).vir_lazy;
        {
            let np = self.node_mut(parent);
            np.vir_sum = np.vir_sum.wrapping_add(sum);
            np.vir_sz = np.vir_sz.wrapping_add(sz);
        }
        self.node_mut(child_root).vir_from_parent = parent_vir_lazy;
    }

    fn virtual_remove(&mut self, parent: Id, child_root: Id) {
        if child_root.is_nil() {
            return;
        }

        // Catch up the virtual child to `parent.vir_lazy` before removing.
        let parent_vir_lazy = self.node(parent).vir_lazy;
        let snap = self.node(child_root).vir_from_parent;
        let diff = parent_vir_lazy.wrapping_sub(snap);
        if diff != 0 {
            self.apply_all_add(child_root, diff);
        }
        self.node_mut(child_root).vir_from_parent = parent_vir_lazy;

        let (sum, sz) = {
            let nc = self.node(child_root);
            (nc.all_sum, nc.all_sz)
        };
        {
            let np = self.node_mut(parent);
            np.vir_sum = np.vir_sum.wrapping_sub(sum);
            np.vir_sz = np.vir_sz.wrapping_sub(sz);
        }
    }

    fn access(&mut self, x: Id) {
        let mut last = Id::NIL;
        let mut y = x;
        while !y.is_nil() {
            self.splay(y);

            // Detach current preferred child (right) into virtual set.
            let old_right = self.node(y).ch[1];
            if old_right != last {
                self.virtual_add(y, old_right);
                // Promote `last` (previously virtual child) to preferred.
                self.virtual_remove(y, last);
            }
            self.node_mut(y).ch[1] = last;
            if !last.is_nil() {
                self.node_mut(last).p = y;
            }
            self.pull(y);

            last = y;
            y = self.node(y).p;
        }
        self.splay(x);
    }

    pub fn makeroot(&mut self, v: usize) {
        debug_assert!(v < self.len());
        let x = id(v);
        self.access(x);
        self.apply_rev(x);
    }

    pub fn find_root(&mut self, v: usize) -> usize {
        debug_assert!(v < self.len());
        let x = id(v);
        self.access(x);
        let mut y = x;
        self.push(y);
        while !self.node(y).ch[0].is_nil() {
            y = self.node(y).ch[0];
            self.push(y);
        }
        self.splay(y);
        y.idx()
    }

    pub fn connected(&mut self, u: usize, v: usize) -> bool {
        debug_assert!(u < self.len() && v < self.len());
        if u == v {
            return true;
        }
        let u = id(u);
        let v = id(v);
        self.makeroot(u.idx());
        self.access(v);
        !self.node(u).p.is_nil()
    }

    pub fn link(&mut self, u: usize, v: usize) -> bool {
        debug_assert!(u < self.len() && v < self.len());
        if u == v {
            return false;
        }
        let u = id(u);
        let v = id(v);
        self.makeroot(u.idx());
        self.access(v);
        if !self.node(u).p.is_nil() {
            return false;
        }

        // `u` becomes a virtual child of `v`.
        self.node_mut(u).p = v;
        let (sum, sz) = self.virtual_add_with_meta(v, u);
        let nv = self.node_mut(v);
        nv.all_sum = nv.all_sum.wrapping_add(sum);
        nv.all_sz = nv.all_sz.wrapping_add(sz);
        true
    }

    pub fn cut(&mut self, u: usize, v: usize) -> bool {
        debug_assert!(u < self.len() && v < self.len());
        if u == v {
            return false;
        }
        let u = id(u);
        let v = id(v);
        self.makeroot(u.idx());
        self.access(v);
        if self.node(v).ch[0] == u && self.node(u).ch[1].is_nil() {
            self.node_mut(v).ch[0] = Id::NIL;
            self.node_mut(u).p = Id::NIL;
            self.pull(v);
            return true;
        }
        false
    }

    pub fn vertex_get(&mut self, v: usize) -> P::Key {
        debug_assert!(v < self.len());
        let x = id(v);
        self.access(x);
        self.node(x).key
    }

    pub fn vertex_set(&mut self, v: usize, key: P::Key) {
        debug_assert!(v < self.len());
        let x = id(v);
        self.access(x);
        let old_key = self.node(x).key;
        let diff = key.wrapping_sub(old_key);
        let nx = self.node_mut(x);
        nx.key = key;
        nx.agg = nx.agg.wrapping_add(diff);
        nx.all_sum = nx.all_sum.wrapping_add(diff);
    }

    pub fn vertex_apply(&mut self, v: usize, delta: P::Act) {
        debug_assert!(v < self.len());
        if delta == 0 {
            return;
        }
        let x = id(v);
        self.access(x);
        let nx = self.node_mut(x);
        nx.key = nx.key.wrapping_add(delta);
        nx.agg = nx.agg.wrapping_add(delta);
        nx.all_sum = nx.all_sum.wrapping_add(delta);
    }

    pub fn path_fold(&mut self, u: usize, v: usize) -> Option<P::Agg> {
        debug_assert!(u < self.len() && v < self.len());
        if u == v {
            return Some(self.vertex_get(u));
        }
        let u_id = id(u);
        let v_id = id(v);
        self.makeroot(u);
        self.access(v_id);
        if self.node(u_id).p.is_nil() {
            return None;
        }
        Some(self.node(v_id).agg)
    }

    pub fn path_apply(&mut self, u: usize, v: usize, delta: P::Act) -> bool {
        debug_assert!(u < self.len() && v < self.len());
        if u == v {
            self.vertex_apply(u, delta);
            return true;
        }
        let u_id = id(u);
        let v_id = id(v);
        self.makeroot(u);
        self.access(v_id);
        if self.node(u_id).p.is_nil() {
            return false;
        }
        self.apply_path_add(v_id, delta);
        true
    }

    pub fn path_len(&mut self, u: usize, v: usize) -> Option<usize> {
        debug_assert!(u < self.len() && v < self.len());
        if u == v {
            return Some(1);
        }
        let u_id = id(u);
        let v_id = id(v);
        self.makeroot(u);
        self.access(v_id);
        if self.node(u_id).p.is_nil() {
            return None;
        }
        Some(self.node(v_id).sz as usize)
    }

    pub fn path_kth(&mut self, u: usize, v: usize, mut k: usize) -> Option<usize> {
        debug_assert!(u < self.len() && v < self.len());
        if u == v {
            return if k == 0 { Some(u) } else { None };
        }
        let u_id = id(u);
        let v_id = id(v);
        self.makeroot(u);
        self.access(v_id);
        if self.node(u_id).p.is_nil() {
            return None;
        }
        let mut x = v_id;
        let total = self.node(x).sz as usize;
        if k >= total {
            return None;
        }
        loop {
            self.push(x);
            let l = self.node(x).ch[0];
            let lsz = self.sz(l) as usize;
            if k < lsz {
                x = l;
                continue;
            }
            if k == lsz {
                self.splay(x);
                return Some(x.idx());
            }
            k -= lsz + 1;
            x = self.node(x).ch[1];
        }
    }

    pub fn component_fold(&mut self, v: usize) -> P::Agg {
        debug_assert!(v < self.len());
        let x = id(v);
        self.access(x);
        self.node(x).all_sum
    }

    pub fn component_apply(&mut self, v: usize, delta: P::Act) {
        debug_assert!(v < self.len());
        let x = id(v);
        self.access(x);
        self.apply_all_add(x, delta);
    }

    pub fn component_size(&mut self, v: usize) -> usize {
        debug_assert!(v < self.len());
        let x = id(v);
        self.access(x);
        self.node(x).all_sz as usize
    }

    #[inline(always)]
    fn apply_exposed_subtree_add(&mut self, x: Id, delta: P::Act) {
        if x.is_nil() || delta == 0 {
            return;
        }
        debug_assert!(
            self.node(x).ch[1].is_nil(),
            "apply_exposed_subtree_add expects `access(child)` state",
        );
        let vir_sz = self.node(x).vir_sz as i64;
        let vir_inc = delta.wrapping_mul(vir_sz);
        let nx = self.node_mut(x);
        nx.key = nx.key.wrapping_add(delta);
        nx.agg = nx.agg.wrapping_add(delta);
        nx.vir_sum = nx.vir_sum.wrapping_add(vir_inc);
        nx.all_sum = nx.all_sum.wrapping_add(delta.wrapping_add(vir_inc));
        nx.vir_lazy = nx.vir_lazy.wrapping_add(delta);
    }

    pub fn subtree_fold(&mut self, child: usize, parent: usize) -> P::Agg {
        debug_assert!(child < self.len() && parent < self.len());
        debug_assert_ne!(child, parent);
        let child_id = id(child);
        #[cfg(debug_assertions)]
        let parent_id = id(parent);

        self.makeroot(parent);
        self.access(child_id);

        #[cfg(debug_assertions)]
        {
            // After `makeroot(parent); access(child)`, the rightmost node of `child.left`
            // must be `parent` when `(child, parent)` is an actual edge.
            let mut y = self.node(child_id).ch[0];
            debug_assert!(!y.is_nil(), "subtree_fold requires an existing edge");
            self.push(y);
            while !self.node(y).ch[1].is_nil() {
                y = self.node(y).ch[1];
                self.push(y);
            }
            debug_assert_eq!(y, parent_id, "subtree_fold requires an existing edge");
        }

        let nx = self.node(child_id);
        nx.key.wrapping_add(nx.vir_sum)
    }

    pub fn subtree_apply(&mut self, child: usize, parent: usize, delta: P::Act) {
        debug_assert!(child < self.len() && parent < self.len());
        debug_assert_ne!(child, parent);
        let child_id = id(child);
        #[cfg(debug_assertions)]
        let parent_id = id(parent);

        self.makeroot(parent);
        self.access(child_id);

        #[cfg(debug_assertions)]
        {
            // Same adjacency check as `subtree_fold`.
            let mut y = self.node(child_id).ch[0];
            debug_assert!(!y.is_nil(), "subtree_apply requires an existing edge");
            self.push(y);
            while !self.node(y).ch[1].is_nil() {
                y = self.node(y).ch[1];
                self.push(y);
            }
            debug_assert_eq!(y, parent_id, "subtree_apply requires an existing edge");
        }

        self.apply_exposed_subtree_add(child_id, delta);
    }
}

impl LinkCutTreeSubtree<VertexSumAdd> {
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

impl<P: LazyMapMonoid<Key = i64, Agg = i64, Act = i64>> DynamicForest for LinkCutTreeSubtree<P> {
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

impl<P: LazyMapMonoid<Key = i64, Agg = i64, Act = i64>> VertexOps for LinkCutTreeSubtree<P> {
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

impl<P: LazyMapMonoid<Key = i64, Agg = i64, Act = i64>> PathOps for LinkCutTreeSubtree<P> {
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

impl<P: LazyMapMonoid<Key = i64, Agg = i64, Act = i64>> ComponentOps for LinkCutTreeSubtree<P> {
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

impl<P: LazyMapMonoid<Key = i64, Agg = i64, Act = i64>> SubtreeOps for LinkCutTreeSubtree<P> {
    type Agg = P::Agg;
    type Act = P::Act;

    fn subtree_fold(&mut self, child: usize, parent: usize) -> Self::Agg {
        self.subtree_fold(child, parent)
    }

    fn subtree_apply(&mut self, child: usize, parent: usize, act: Self::Act) {
        self.subtree_apply(child, parent, act)
    }
}
