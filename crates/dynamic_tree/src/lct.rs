use crate::policy::{LazyMapMonoid, VertexSumAdd};
use crate::traits::{DynamicForest, PathOps, VertexOps};

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
struct Node<P: LazyMapMonoid> {
    ch: [Id; 2],
    p: Id,
    rev: bool,

    key: P::Key,
    agg: P::Agg,
    agg_rev: P::Agg,
    sz: u32,

    lazy: P::Act,
    lazy_pending: bool,
}

impl<P: LazyMapMonoid> Node<P> {
    fn new(key: P::Key) -> Self {
        let agg = P::agg_from_key(&key);
        Self {
            ch: [Id::NIL, Id::NIL],
            p: Id::NIL,
            rev: false,
            key,
            agg,
            agg_rev: agg,
            sz: 1,
            lazy: P::act_unit(),
            lazy_pending: false,
        }
    }
}

/// Link-Cut Tree (splay-based).
///
/// Generic over a `LazyMapMonoid` policy.
pub struct LinkCutTree<P: LazyMapMonoid = VertexSumAdd> {
    nodes: Vec<Node<P>>,
    stack: Vec<Id>,
}

impl<P: LazyMapMonoid> LinkCutTree<P> {
    pub fn new(values: &[P::Key]) -> Self {
        let mut nodes = Vec::with_capacity(values.len());
        for &v in values {
            debug_assert!(nodes.len() < u32::MAX as usize);
            nodes.push(Node::<P>::new(v));
        }
        Self {
            nodes,
            stack: Vec::with_capacity(values.len()),
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
    fn agg(&self, x: Id) -> P::Agg {
        if x.is_nil() {
            P::agg_unit()
        } else {
            self.node(x).agg
        }
    }

    #[inline(always)]
    fn agg_rev(&self, x: Id) -> P::Agg {
        if x.is_nil() {
            P::agg_unit()
        } else {
            self.node(x).agg_rev
        }
    }

    fn is_aux_root(&self, x: Id) -> bool {
        let p = self.node(x).p;
        if p.is_nil() {
            return true;
        }
        self.node(p).ch[0] != x && self.node(p).ch[1] != x
    }

    fn apply_rev(&mut self, x: Id) {
        if x.is_nil() {
            return;
        }
        let nx = self.node_mut(x);
        nx.ch.swap(0, 1);
        if !P::REVERSAL_INVARIANT {
            std::mem::swap(&mut nx.agg, &mut nx.agg_rev);
        }
        nx.rev ^= true;
    }

    fn apply_act(&mut self, x: Id, act: P::Act) {
        if x.is_nil() {
            return;
        }
        let sz = self.node(x).sz as usize;
        let nx = self.node_mut(x);
        nx.key = P::act_apply_key(&nx.key, &act);
        let new_agg = P::act_apply_agg(&nx.agg, &act, sz);
        nx.agg = new_agg;
        if P::REVERSAL_INVARIANT {
            nx.agg_rev = new_agg;
        } else {
            nx.agg_rev = P::act_apply_agg(&nx.agg_rev, &act, sz);
        }
        if nx.lazy_pending {
            nx.lazy = P::act_compose(&act, &nx.lazy);
        } else {
            nx.lazy = act;
            nx.lazy_pending = true;
        }
    }

    fn push(&mut self, x: Id) {
        if x.is_nil() {
            return;
        }

        let (rev, lazy_pending, lazy, l, r) = {
            let nx = self.node(x);
            (nx.rev, nx.lazy_pending, nx.lazy, nx.ch[0], nx.ch[1])
        };

        if rev {
            self.apply_rev(l);
            self.apply_rev(r);
            self.node_mut(x).rev = false;
        }

        if lazy_pending {
            self.apply_act(l, lazy);
            self.apply_act(r, lazy);
            let nx = self.node_mut(x);
            nx.lazy = P::act_unit();
            nx.lazy_pending = false;
        }
    }

    fn pull(&mut self, x: Id) {
        if x.is_nil() {
            return;
        }
        let (l, r, key) = {
            let nx = self.node(x);
            (nx.ch[0], nx.ch[1], nx.key)
        };
        let sz = 1_u32.wrapping_add(self.sz(l)).wrapping_add(self.sz(r));
        let agg = P::agg_merge(&self.agg(l), &key, &self.agg(r));
        let agg_rev = if P::REVERSAL_INVARIANT {
            agg
        } else {
            P::agg_merge(&self.agg_rev(r), &key, &self.agg_rev(l))
        };
        let nx = self.node_mut(x);
        nx.sz = sz;
        nx.agg = agg;
        nx.agg_rev = agg_rev;
    }

    fn rotate(&mut self, x: Id) {
        let p = self.node(x).p;
        let g = self.node(p).p;
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

    fn access(&mut self, x: Id) {
        let mut last = Id::NIL;
        let mut y = x;
        while !y.is_nil() {
            self.splay(y);
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
        if self.connected(u, v) {
            return false;
        }
        let u = id(u);
        let v = id(v);
        self.makeroot(u.idx());
        self.node_mut(u).p = v;
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
        self.push(x);
        self.node(x).key
    }

    pub fn vertex_set(&mut self, v: usize, key: P::Key) {
        debug_assert!(v < self.len());
        let x = id(v);
        self.access(x);
        self.push(x);
        self.node_mut(x).key = key;
        self.pull(x);
    }

    pub fn vertex_apply(&mut self, v: usize, act: P::Act) {
        debug_assert!(v < self.len());
        let x = id(v);
        self.access(x);
        self.push(x);
        let key = self.node(x).key;
        self.node_mut(x).key = P::act_apply_key(&key, &act);
        self.pull(x);
    }

    pub fn path_fold(&mut self, u: usize, v: usize) -> Option<P::Agg> {
        debug_assert!(u < self.len() && v < self.len());
        if u == v {
            let key = self.vertex_get(u);
            return Some(P::agg_from_key(&key));
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

    pub fn path_apply(&mut self, u: usize, v: usize, act: P::Act) -> bool {
        debug_assert!(u < self.len() && v < self.len());
        if u == v {
            self.vertex_apply(u, act);
            return true;
        }
        let u_id = id(u);
        let v_id = id(v);
        self.makeroot(u);
        self.access(v_id);
        if self.node(u_id).p.is_nil() {
            return false;
        }
        self.apply_act(v_id, act);
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
}

impl LinkCutTree<VertexSumAdd> {
    pub fn vertex_add(&mut self, v: usize, delta: i64) {
        self.vertex_apply(v, delta);
    }

    pub fn path_sum(&mut self, u: usize, v: usize) -> Option<i64> {
        self.path_fold(u, v)
    }
}

impl<P: LazyMapMonoid> DynamicForest for LinkCutTree<P> {
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

impl<P: LazyMapMonoid> VertexOps for LinkCutTree<P> {
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

impl<P: LazyMapMonoid> PathOps for LinkCutTree<P> {
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
