use crate::OrderedMap;

pub struct SortedVecMap<K: Ord, V> {
    data: Vec<(K, V)>,
}

impl<K: Ord, V> OrderedMap for SortedVecMap<K, V> {
    type Key = K;
    type Value = V;

    fn new() -> Self {
        Self { data: Vec::new() }
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn get(&mut self, key: &Self::Key) -> Option<&Self::Value> {
        let idx = self.data.binary_search_by(|(k, _)| k.cmp(key)).ok()?;
        self.data.get(idx).map(|(_, v)| v)
    }

    fn insert(&mut self, key: Self::Key, value: Self::Value) -> Option<Self::Value> {
        match self.data.binary_search_by(|(k, _)| k.cmp(&key)) {
            Ok(idx) => {
                let old = std::mem::replace(&mut self.data[idx].1, value);
                Some(old)
            }
            Err(idx) => {
                self.data.insert(idx, (key, value));
                None
            }
        }
    }

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
        let idx = self.data.binary_search_by(|(k, _)| k.cmp(key)).ok()?;
        Some(self.data.remove(idx).1)
    }

    fn lower_bound(&mut self, key: &Self::Key) -> Option<(&Self::Key, &Self::Value)> {
        let idx = self.data.partition_point(|(k, _)| k < key);
        self.data.get(idx).map(|(k, v)| (k, v))
    }
}
