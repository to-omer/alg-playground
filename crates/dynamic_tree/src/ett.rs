use crate::policy::{LazyMapMonoid, VertexSumAdd};
use crate::traits::{ComponentOps, DynamicForest, SubtreeOps, VertexOps};

type Id = u32;
const NIL: Id = Id::MAX;

#[inline(always)]
fn idx(x: Id) -> usize {
    x as usize
}

#[derive(Clone, Copy, Debug)]
struct Node<P: LazyMapMonoid> {
    ch: [Id; 2],
    p: Id,

    is_vertex: bool,
    v_cnt: u32,

    key: P::Key,
    agg: P::Agg,

    lazy: P::Act,
    lazy_pending: bool,
}

impl<P: LazyMapMonoid> Node<P> {
    fn new_vertex(key: P::Key) -> Self {
        Self {
            ch: [NIL, NIL],
            p: NIL,
            is_vertex: true,
            v_cnt: 1,
            key,
            agg: P::agg_from_key(&key),
            lazy: P::act_unit(),
            lazy_pending: false,
        }
    }

    fn new_arc() -> Self {
        let key = P::key_unit();
        Self {
            ch: [NIL, NIL],
            p: NIL,
            is_vertex: false,
            v_cnt: 0,
            key,
            agg: P::agg_from_key(&key),
            lazy: P::act_unit(),
            lazy_pending: false,
        }
    }
}

/// Euler Tour Tree implemented as a splay-sequence.
///
/// Generic over a `LazyMapMonoid` policy.
pub struct EulerTourTree<P: LazyMapMonoid = VertexSumAdd> {
    nodes: Vec<Node<P>>,
    vertex_node: Vec<Id>,
    arcs: Vec<Vec<(u32, Id)>>, // arcs[u] contains (v, arc(u->v) node id)
    free: Vec<Id>,
    stack: Vec<Id>,
}

impl<P: LazyMapMonoid> EulerTourTree<P> {
    pub fn new(values: &[P::Key]) -> Self {
        let n = values.len();
        let mut nodes = Vec::with_capacity(n.saturating_mul(3));
        let mut vertex_node = Vec::with_capacity(n);
        for &v in values {
            debug_assert!(nodes.len() < u32::MAX as usize);
            let id = nodes.len() as Id;
            nodes.push(Node::<P>::new_vertex(v));
            vertex_node.push(id);
        }
        let arcs = vec![Vec::new(); n];
        Self {
            nodes,
            vertex_node,
            arcs,
            free: Vec::new(),
            stack: Vec::with_capacity(n),
        }
    }

    pub fn len(&self) -> usize {
        self.vertex_node.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vertex_node.is_empty()
    }

    #[inline(always)]
    fn node(&self, x: Id) -> &Node<P> {
        debug_assert!(x != NIL);
        debug_assert!(idx(x) < self.nodes.len());
        if cfg!(debug_assertions) {
            &self.nodes[idx(x)]
        } else {
            // SAFETY: `Id` values are only created from valid indices and `NIL` is checked.
            unsafe { self.nodes.get_unchecked(idx(x)) }
        }
    }

    #[inline(always)]
    fn node_mut(&mut self, x: Id) -> &mut Node<P> {
        debug_assert!(x != NIL);
        debug_assert!(idx(x) < self.nodes.len());
        if cfg!(debug_assertions) {
            &mut self.nodes[idx(x)]
        } else {
            // SAFETY: `Id` values are only created from valid indices and `NIL` is checked.
            unsafe { self.nodes.get_unchecked_mut(idx(x)) }
        }
    }

    #[inline(always)]
    fn v_cnt(&self, x: Id) -> u32 {
        if x == NIL { 0 } else { self.node(x).v_cnt }
    }

    #[inline(always)]
    fn agg(&self, x: Id) -> P::Agg {
        if x == NIL {
            P::agg_unit()
        } else {
            self.node(x).agg
        }
    }

    fn pull(&mut self, x: Id) {
        let (l, r, is_vertex, key) = {
            let nx = self.node(x);
            (nx.ch[0], nx.ch[1], nx.is_vertex, nx.key)
        };
        let v_cnt = self
            .v_cnt(l)
            .wrapping_add(self.v_cnt(r))
            .wrapping_add(u32::from(is_vertex));
        let agg = P::agg_merge(&self.agg(l), &key, &self.agg(r));
        let nx = self.node_mut(x);
        nx.v_cnt = v_cnt;
        nx.agg = agg;
    }

    fn apply_act(&mut self, x: Id, act: P::Act) {
        if x == NIL {
            return;
        }
        let len = self.node(x).v_cnt as usize;
        let nx = self.node_mut(x);
        if nx.is_vertex {
            nx.key = P::act_apply_key(&nx.key, &act);
        }
        nx.agg = P::act_apply_agg(&nx.agg, &act, len);
        if nx.lazy_pending {
            nx.lazy = P::act_compose(&act, &nx.lazy);
        } else {
            nx.lazy = act;
            nx.lazy_pending = true;
        }
    }

    fn push(&mut self, x: Id) {
        if x == NIL {
            return;
        }
        let (pending, lazy, l, r) = {
            let nx = self.node(x);
            (nx.lazy_pending, nx.lazy, nx.ch[0], nx.ch[1])
        };
        if !pending {
            return;
        }
        self.apply_act(l, lazy);
        self.apply_act(r, lazy);
        let nx = self.node_mut(x);
        nx.lazy = P::act_unit();
        nx.lazy_pending = false;
    }

    fn push_path(&mut self, x: Id) {
        self.stack.clear();
        let mut y = x;
        self.stack.push(y);
        while self.node(y).p != NIL {
            y = self.node(y).p;
            self.stack.push(y);
        }
        for i in (0..self.stack.len()).rev() {
            let v = self.stack[i];
            self.push(v);
        }
    }

    fn rotate(&mut self, x: Id) {
        let p = self.node(x).p;
        let g = self.node(p).p;
        self.push(p);
        self.push(x);

        let dir = usize::from(self.node(p).ch[1] == x);
        let b = self.node(x).ch[dir ^ 1];

        if g != NIL {
            if self.node(g).ch[0] == p {
                self.node_mut(g).ch[0] = x;
            } else {
                self.node_mut(g).ch[1] = x;
            }
        }
        self.node_mut(x).p = g;

        self.node_mut(x).ch[dir ^ 1] = p;
        self.node_mut(p).p = x;

        self.node_mut(p).ch[dir] = b;
        if b != NIL {
            self.node_mut(b).p = p;
        }

        self.pull(p);
        self.pull(x);
    }

    fn splay(&mut self, x: Id) {
        self.push_path(x);

        while self.node(x).p != NIL {
            let p = self.node(x).p;
            let g = self.node(p).p;
            if g == NIL {
                self.rotate(x);
            } else {
                let zigzig = (self.node(g).ch[0] == p) == (self.node(p).ch[0] == x);
                if zigzig {
                    self.rotate(p);
                    self.rotate(x);
                } else {
                    self.rotate(x);
                    self.rotate(x);
                }
            }
        }
    }

    fn split_at_node(&mut self, x: Id) -> (Id, Id) {
        self.splay(x);
        let left = self.node(x).ch[0];
        if left != NIL {
            self.node_mut(left).p = NIL;
        }
        self.node_mut(x).ch[0] = NIL;
        self.pull(x);
        (left, x)
    }

    fn merge(&mut self, a: Id, b: Id) -> Id {
        if a == NIL {
            return b;
        }
        if b == NIL {
            return a;
        }
        let mut x = a;
        while self.node(x).ch[1] != NIL {
            x = self.node(x).ch[1];
        }
        self.splay(x);
        self.node_mut(x).ch[1] = b;
        self.node_mut(b).p = x;
        self.pull(x);
        x
    }

    fn pop_front(&mut self, root: Id) -> (Id, Id) {
        if root == NIL {
            return (NIL, NIL);
        }
        let mut x = root;
        while self.node(x).ch[0] != NIL {
            x = self.node(x).ch[0];
        }
        self.splay(x);
        let right = self.node(x).ch[1];
        if right != NIL {
            self.node_mut(right).p = NIL;
        }
        self.node_mut(x).ch[1] = NIL;
        self.pull(x);
        self.node_mut(x).p = NIL;
        (x, right)
    }

    fn reroot(&mut self, x: Id) -> Id {
        let (l, r) = self.split_at_node(x);
        self.merge(r, l)
    }

    fn root_of(&self, mut x: Id) -> Id {
        while self.node(x).p != NIL {
            x = self.node(x).p;
        }
        x
    }

    fn new_arc_node(&mut self) -> Id {
        if let Some(id) = self.free.pop() {
            *self.node_mut(id) = Node::<P>::new_arc();
            return id;
        }
        debug_assert!(self.nodes.len() < u32::MAX as usize);
        let id = self.nodes.len() as Id;
        self.nodes.push(Node::<P>::new_arc());
        id
    }

    fn remove_arc_opt(&mut self, u: usize, v: usize) -> Option<Id> {
        let list = &mut self.arcs[u];
        let v = v as u32;
        let i = list.iter().position(|&(to, _)| to == v)?;
        let (_to, node) = list.swap_remove(i);
        Some(node)
    }

    pub fn connected(&mut self, u: usize, v: usize) -> bool {
        debug_assert!(u < self.len() && v < self.len());
        if u == v {
            return true;
        }
        let ru = self.root_of(self.vertex_node[u]);
        let rv = self.root_of(self.vertex_node[v]);
        ru == rv
    }

    pub fn link(&mut self, u: usize, v: usize) -> bool {
        debug_assert!(u < self.len() && v < self.len());
        if u == v {
            return false;
        }
        if self.connected(u, v) {
            return false;
        }
        let ru = self.reroot(self.vertex_node[u]);
        let rv = self.reroot(self.vertex_node[v]);
        let a = self.new_arc_node();
        let b = self.new_arc_node();
        self.arcs[u].push((v as u32, a));
        self.arcs[v].push((u as u32, b));
        let t = self.merge(ru, a);
        let t = self.merge(t, rv);
        let _t = self.merge(t, b);
        true
    }

    pub fn cut(&mut self, u: usize, v: usize) -> bool {
        debug_assert!(u < self.len() && v < self.len());
        if u == v {
            return false;
        }
        let Some(arc_uv) = self.remove_arc_opt(u, v) else {
            return false;
        };
        let Some(arc_vu) = self.remove_arc_opt(v, u) else {
            // should not happen; rollback
            self.arcs[u].push((v as u32, arc_uv));
            return false;
        };

        let t = self.reroot(arc_uv);
        let (_removed, rest) = self.pop_front(t);
        let (x, y) = self.split_at_node(arc_vu);
        let (_removed2, y2) = self.pop_front(y);
        let _ = (x, y2, rest);

        self.free.push(arc_uv);
        self.free.push(arc_vu);
        true
    }

    pub fn vertex_get(&mut self, v: usize) -> P::Key {
        debug_assert!(v < self.len());
        let x = self.vertex_node[v];
        self.splay(x);
        self.node(x).key
    }

    pub fn vertex_set(&mut self, v: usize, key: P::Key) {
        debug_assert!(v < self.len());
        let x = self.vertex_node[v];
        self.splay(x);
        self.node_mut(x).key = key;
        self.pull(x);
    }

    pub fn vertex_apply(&mut self, v: usize, act: P::Act) {
        debug_assert!(v < self.len());
        let x = self.vertex_node[v];
        self.splay(x);
        let key = self.node(x).key;
        self.node_mut(x).key = P::act_apply_key(&key, &act);
        self.pull(x);
    }

    pub fn component_fold(&mut self, v: usize) -> P::Agg {
        debug_assert!(v < self.len());
        let r = self.root_of(self.vertex_node[v]);
        self.node(r).agg
    }

    pub fn component_apply(&mut self, v: usize, act: P::Act) {
        debug_assert!(v < self.len());
        let r = self.root_of(self.vertex_node[v]);
        self.apply_act(r, act);
    }

    pub fn component_size(&mut self, v: usize) -> usize {
        debug_assert!(v < self.len());
        let r = self.root_of(self.vertex_node[v]);
        self.node(r).v_cnt as usize
    }

    pub fn subtree_fold(&mut self, child: usize, parent: usize) -> P::Agg {
        debug_assert!(child < self.len() && parent < self.len());
        let ok = self.cut(child, parent);
        debug_assert!(ok, "subtree_fold requires an existing edge");
        let res = self.component_fold(child);
        let ok2 = self.link(child, parent);
        debug_assert!(ok2);
        res
    }

    pub fn subtree_apply(&mut self, child: usize, parent: usize, act: P::Act) {
        debug_assert!(child < self.len() && parent < self.len());
        let ok = self.cut(child, parent);
        debug_assert!(ok, "subtree_apply requires an existing edge");
        self.component_apply(child, act);
        let ok2 = self.link(child, parent);
        debug_assert!(ok2);
    }
}

impl EulerTourTree<VertexSumAdd> {
    pub fn vertex_add(&mut self, v: usize, delta: i64) {
        self.vertex_apply(v, delta);
    }

    pub fn component_sum(&mut self, v: usize) -> i64 {
        self.component_fold(v)
    }
}

impl<P: LazyMapMonoid> DynamicForest for EulerTourTree<P> {
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

impl<P: LazyMapMonoid> VertexOps for EulerTourTree<P> {
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

impl<P: LazyMapMonoid> ComponentOps for EulerTourTree<P> {
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

impl<P: LazyMapMonoid> SubtreeOps for EulerTourTree<P> {
    type Agg = P::Agg;
    type Act = P::Act;

    fn subtree_fold(&mut self, child: usize, parent: usize) -> Self::Agg {
        self.subtree_fold(child, parent)
    }

    fn subtree_apply(&mut self, child: usize, parent: usize, act: Self::Act) {
        self.subtree_apply(child, parent, act)
    }
}
