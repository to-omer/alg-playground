use crate::OrderedMap;

use super::FastHashMap;
use std::collections::hash_map::Entry;

pub struct VebMap<V> {
    tree: VebNode,
    entries: FastHashMap<u64, V>,
    len: usize,
}

impl<V> OrderedMap for VebMap<V> {
    type Key = u64;
    type Value = V;

    fn new() -> Self {
        Self {
            tree: VebNode::new(64),
            entries: FastHashMap::default(),
            len: 0,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn get(&mut self, key: &Self::Key) -> Option<&Self::Value> {
        self.entries.get(key)
    }

    fn insert(&mut self, key: Self::Key, value: Self::Value) -> Option<Self::Value> {
        match self.entries.entry(key) {
            Entry::Occupied(mut e) => Some(e.insert(value)),
            Entry::Vacant(e) => {
                self.tree.insert(*e.key());
                e.insert(value);
                self.len += 1;
                None
            }
        }
    }

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
        let value = self.entries.remove(key)?;
        self.tree.remove(*key);
        self.len -= 1;
        Some(value)
    }

    fn lower_bound(&mut self, key: &Self::Key) -> Option<(&Self::Key, &Self::Value)> {
        if let Some((k, v)) = self.entries.get_key_value(key) {
            return Some((k, v));
        }
        let succ = self.tree.successor(*key)?;
        self.entries.get_key_value(&succ)
    }
}

struct VebNode {
    w: u32,
    min: Option<u64>,
    max: Option<u64>,
    summary: Option<Box<VebNode>>,
    clusters: FastHashMap<u64, Box<VebNode>>,
}

impl VebNode {
    fn new(w: u32) -> Self {
        Self {
            w,
            min: None,
            max: None,
            summary: None,
            clusters: FastHashMap::default(),
        }
    }

    fn upper_lower(&self) -> (u32, u32) {
        let lower = self.w / 2;
        let upper = self.w - lower;
        (upper, lower)
    }

    fn high(&self, x: u64) -> u64 {
        let (_, lower) = self.upper_lower();
        if lower == 0 { x } else { x >> lower }
    }

    fn low(&self, x: u64) -> u64 {
        let (_, lower) = self.upper_lower();
        if lower == 0 {
            0
        } else {
            let mask = (1_u64 << lower) - 1;
            x & mask
        }
    }

    fn index(&self, high: u64, low: u64) -> u64 {
        let (_, lower) = self.upper_lower();
        if lower == 0 {
            high
        } else {
            (high << lower) | low
        }
    }

    fn empty_insert(&mut self, x: u64) {
        self.min = Some(x);
        self.max = Some(x);
    }

    fn insert(&mut self, mut x: u64) {
        if self.min.is_none() {
            self.empty_insert(x);
            return;
        }

        let min = self.min.unwrap();
        if x < min {
            self.min = Some(x);
            x = min;
        }

        if self.w > 1 {
            let (upper, lower) = self.upper_lower();
            let h = self.high(x);
            let l = self.low(x);

            let cluster = self
                .clusters
                .entry(h)
                .or_insert_with(|| Box::new(VebNode::new(lower)));
            if cluster.min.is_none() {
                if self.summary.is_none() {
                    self.summary = Some(Box::new(VebNode::new(upper)));
                }
                self.summary.as_deref_mut().unwrap().insert(h);
                cluster.empty_insert(l);
            } else {
                cluster.insert(l);
            }
        }

        if x > self.max.unwrap() {
            self.max = Some(x);
        }
    }

    fn remove(&mut self, mut x: u64) {
        let min = self.min.expect("remove on empty vEB");
        let max = self.max.expect("remove on empty vEB");

        if min == max {
            debug_assert_eq!(x, min);
            self.min = None;
            self.max = None;
            self.summary = None;
            self.clusters.clear();
            return;
        }

        if self.w <= 1 {
            // Universe size 2.
            if x == 0 {
                self.min = Some(1);
                self.max = Some(1);
            } else {
                self.min = Some(0);
                self.max = Some(0);
            }
            return;
        }

        if x == min {
            let first_cluster = self
                .summary
                .as_deref()
                .and_then(|s| s.min)
                .expect("non-empty summary");
            let cluster = self.clusters.get(&first_cluster).expect("cluster exists");
            let new_low = cluster.min.expect("non-empty cluster");
            x = self.index(first_cluster, new_low);
            self.min = Some(x);
        }

        let h = self.high(x);
        let l = self.low(x);
        let cluster = self.clusters.get_mut(&h).expect("cluster exists");
        cluster.remove(l);

        if cluster.min.is_none() {
            self.clusters.remove(&h);
            self.summary.as_deref_mut().unwrap().remove(h);
            if self.summary.as_deref().unwrap().min.is_none() {
                self.summary = None;
                self.max = self.min;
            } else if x == max {
                let summary_max = self.summary.as_deref().unwrap().max.unwrap();
                let cluster = self.clusters.get(&summary_max).unwrap();
                let new_low = cluster.max.unwrap();
                self.max = Some(self.index(summary_max, new_low));
            }
        } else if x == max {
            let new_low = cluster.max.unwrap();
            self.max = Some(self.index(h, new_low));
        }
    }

    fn successor(&self, x: u64) -> Option<u64> {
        let min = self.min?;
        let max = self.max?;
        if x < min {
            return Some(min);
        }
        if self.w <= 1 {
            if x == 0 && max == 1 {
                return Some(1);
            }
            return None;
        }

        let h = self.high(x);
        let l = self.low(x);
        if let Some(cluster) = self.clusters.get(&h)
            && cluster.max.is_some()
            && l < cluster.max.unwrap()
        {
            let offset = cluster.successor(l).unwrap();
            return Some(self.index(h, offset));
        }

        let succ_cluster = self.summary.as_deref().and_then(|s| s.successor(h))?;
        let cluster = self.clusters.get(&succ_cluster)?;
        let offset = cluster.min?;
        Some(self.index(succ_cluster, offset))
    }
}
