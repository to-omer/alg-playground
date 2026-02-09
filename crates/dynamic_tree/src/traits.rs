//! Trait-based API for dynamic forest operations.

pub trait DynamicForest: Sized {
    type Key: Copy;

    fn new(values: &[Self::Key]) -> Self;
    fn len(&self) -> usize;
    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Link `u` and `v` if they are in different components.
    ///
    /// Returns `false` if they are already connected.
    fn link(&mut self, u: usize, v: usize) -> bool;

    /// Cut edge `(u, v)` if it exists.
    ///
    /// Returns `false` if there is no such edge.
    fn cut(&mut self, u: usize, v: usize) -> bool;

    fn connected(&mut self, u: usize, v: usize) -> bool;
}

pub trait VertexOps: DynamicForest {
    type Act: Copy;

    fn vertex_get(&mut self, v: usize) -> Self::Key;
    fn vertex_set(&mut self, v: usize, key: Self::Key);
    fn vertex_apply(&mut self, v: usize, act: Self::Act);
}

pub trait PathOps: DynamicForest {
    type Agg: Copy;
    type Act: Copy;

    fn makeroot(&mut self, v: usize);
    fn find_root(&mut self, v: usize) -> usize;

    fn path_fold(&mut self, u: usize, v: usize) -> Option<Self::Agg>;
    fn path_apply(&mut self, u: usize, v: usize, act: Self::Act) -> bool;

    fn path_len(&mut self, u: usize, v: usize) -> Option<usize>;
    fn path_kth(&mut self, u: usize, v: usize, k: usize) -> Option<usize>;
}

pub trait ComponentOps: DynamicForest {
    type Agg: Copy;
    type Act: Copy;

    fn component_fold(&mut self, v: usize) -> Self::Agg;
    fn component_apply(&mut self, v: usize, act: Self::Act);
    fn component_size(&mut self, v: usize) -> usize;
}

pub trait SubtreeOps: DynamicForest {
    type Agg: Copy;
    type Act: Copy;

    /// Fold the subtree on the `child` side of the edge `(child, parent)`.
    fn subtree_fold(&mut self, child: usize, parent: usize) -> Self::Agg;

    /// Apply to the subtree on the `child` side of the edge `(child, parent)`.
    fn subtree_apply(&mut self, child: usize, parent: usize, act: Self::Act);
}
