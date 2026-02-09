use std::collections::BTreeMap;

use crate::OrderedMap;

pub struct StdBTreeMap<K: Ord, V> {
    inner: BTreeMap<K, V>,
}

impl<K: Ord, V> StdBTreeMap<K, V> {
    pub fn into_inner(self) -> BTreeMap<K, V> {
        self.inner
    }
}

impl<K: Ord, V> OrderedMap for StdBTreeMap<K, V> {
    type Key = K;
    type Value = V;

    fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
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
        self.inner.range(key..).next()
    }
}
