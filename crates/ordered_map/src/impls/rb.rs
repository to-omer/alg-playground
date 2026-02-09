use crate::OrderedMap;

use super::LlrbTreeMap;

/// Red-black tree map.
///
/// Note: This implementation uses a left-leaning red-black tree (LLRB) algorithm.
pub struct RbTreeMap<K: Ord, V> {
    inner: LlrbTreeMap<K, V>,
}

impl<K: Ord, V> OrderedMap for RbTreeMap<K, V> {
    type Key = K;
    type Value = V;

    fn new() -> Self {
        Self {
            inner: LlrbTreeMap::new(),
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
