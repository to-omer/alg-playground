use crate::OrderedMap;

use super::btree::BTreeMapBase;

const MIN_DEGREE_FUSION: usize = 32;

/// Fusion tree map (skeleton).
///
/// This currently uses a high-degree B-tree backbone and can be optimized later
/// by adding fusion-node sketches for faster branching decisions.
pub struct FusionTreeMap<V> {
    inner: BTreeMapBase<u64, V, MIN_DEGREE_FUSION>,
}

impl<V> OrderedMap for FusionTreeMap<V> {
    type Key = u64;
    type Value = V;

    fn new() -> Self {
        Self {
            inner: BTreeMapBase::new(),
        }
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn get(&mut self, key: &Self::Key) -> Option<&Self::Value> {
        self.inner.get(key)
    }

    fn insert(&mut self, key: Self::Key, value: Self::Value) -> Option<Self::Value> {
        self.inner.insert(key, value)
    }

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
        self.inner.remove(key)
    }

    fn lower_bound(&mut self, key: &Self::Key) -> Option<(&Self::Key, &Self::Value)> {
        self.inner.lower_bound(key)
    }
}
