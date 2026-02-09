use std::ptr::NonNull;

use crate::OrderedMap;

use super::xfast::XFastTrieMap;

const W: u64 = 64;
const DEFAULT_SEED: u64 = 0x5EED_FA55_2026;

#[derive(Clone, Copy)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

struct Bucket<V> {
    data: Vec<(u64, V)>,
}

impl<V> Bucket<V> {
    fn new() -> Self {
        Self { data: Vec::new() }
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn get(&self, key: &u64) -> Option<&V> {
        let idx = self.data.binary_search_by(|(k, _)| k.cmp(key)).ok()?;
        self.data.get(idx).map(|(_, v)| v)
    }

    fn insert(&mut self, key: u64, value: V) -> Option<V> {
        match self.data.binary_search_by(|(k, _)| k.cmp(&key)) {
            Ok(i) => Some(std::mem::replace(&mut self.data[i].1, value)),
            Err(i) => {
                self.data.insert(i, (key, value));
                None
            }
        }
    }

    fn remove(&mut self, key: &u64) -> Option<V> {
        let idx = self.data.binary_search_by(|(k, _)| k.cmp(key)).ok()?;
        Some(self.data.remove(idx).1)
    }

    fn lower_bound(&self, key: &u64) -> Option<(&u64, &V)> {
        let idx = self.data.partition_point(|(k, _)| k < key);
        self.data.get(idx).map(|(k, v)| (k, v))
    }

    fn first_entry(&self) -> Option<(&u64, &V)> {
        self.data.first().map(|(k, v)| (k, v))
    }

    fn max_key(&self) -> Option<u64> {
        self.data.last().map(|(k, _)| *k)
    }

    fn split_le(&mut self, key: u64) -> Bucket<V> {
        let idx = self.data.partition_point(|(k, _)| *k <= key);
        let left = self.data.drain(..idx).collect::<Vec<_>>();
        Bucket { data: left }
    }
}

pub struct YFastTrieMap<V> {
    reps: XFastTrieMap<NonNull<Bucket<V>>>,
    len: usize,
    rng: XorShift64,
}

impl<V> Drop for YFastTrieMap<V> {
    fn drop(&mut self) {
        while let Some((k, v)) = self.reps.lower_bound(&0) {
            let (rep, bucket_ptr) = (*k, *v);
            let _ = self.reps.remove(&rep);
            unsafe {
                drop(Box::from_raw(bucket_ptr.as_ptr()));
            }
        }
    }
}

impl<V> OrderedMap for YFastTrieMap<V> {
    type Key = u64;
    type Value = V;

    fn new() -> Self {
        Self {
            reps: XFastTrieMap::new(),
            len: 0,
            rng: XorShift64::new(DEFAULT_SEED),
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn get(&mut self, key: &Self::Key) -> Option<&Self::Value> {
        let (_, bucket_ptr) = self.reps.lower_bound(key)?;
        let bucket = unsafe { bucket_ptr.as_ref() };
        bucket.get(key)
    }

    fn insert(&mut self, key: Self::Key, value: Self::Value) -> Option<Self::Value> {
        if self.len == 0 {
            let mut bucket = Box::new(Bucket::new());
            let old = bucket.insert(key, value);
            debug_assert!(old.is_none());
            let rep = bucket.max_key().unwrap();
            let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(bucket)) };
            self.reps.insert(rep, ptr);
            self.len = 1;
            return None;
        }

        let (rep_key, bucket_ptr) = match self.reps.lower_bound(&key) {
            Some((k, v)) => (*k, *v),
            None => {
                let (k, v) = self.reps.max_entry().expect("non-empty reps");
                (*k, *v)
            }
        };

        let bucket = unsafe { bucket_ptr.as_ptr().as_mut().unwrap() };
        let old = bucket.insert(key, value);
        if old.is_none() {
            self.len += 1;
        }

        // Update representative if max changed (happens when inserting into the last bucket).
        let new_rep = bucket.max_key().unwrap();
        if new_rep != rep_key {
            let got = self.reps.remove(&rep_key);
            debug_assert_eq!(got, Some(bucket_ptr));
            self.reps.insert(new_rep, bucket_ptr);
        }

        // Probabilistic split to keep buckets small on average.
        if old.is_none() && self.rng.next_u64().is_multiple_of(W) && bucket.len() > (2 * W as usize)
        {
            let mut left = bucket.split_le(key);
            if !bucket.is_empty() {
                let rep_left = left.max_key().unwrap();
                let left_ptr = unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(left))) };
                let prev = self.reps.insert(rep_left, left_ptr);
                debug_assert!(prev.is_none(), "duplicate representative");
            } else {
                // Split produced an empty right bucket; undo.
                bucket.data.append(&mut left.data);
            }
        }

        old
    }

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
        let (rep_key, bucket_ptr) = self.reps.lower_bound(key).map(|(k, v)| (*k, *v))?;
        let bucket = unsafe { bucket_ptr.as_ptr().as_mut().unwrap() };
        let removed = bucket.remove(key)?;
        self.len -= 1;

        if bucket.is_empty() {
            let got = self.reps.remove(&rep_key);
            debug_assert_eq!(got, Some(bucket_ptr));
            unsafe {
                drop(Box::from_raw(bucket_ptr.as_ptr()));
            }
            return Some(removed);
        }

        if *key == rep_key {
            let new_rep = bucket.max_key().unwrap();
            let got = self.reps.remove(&rep_key);
            debug_assert_eq!(got, Some(bucket_ptr));
            self.reps.insert(new_rep, bucket_ptr);
        }

        Some(removed)
    }

    fn lower_bound(&mut self, key: &Self::Key) -> Option<(&Self::Key, &Self::Value)> {
        let (rep_key, bucket_ptr) = self.reps.lower_bound(key).map(|(k, v)| (*k, *v))?;
        let bucket = unsafe { bucket_ptr.as_ref() };
        if let Some((k, v)) = bucket.lower_bound(key) {
            return Some((k, v));
        }

        if rep_key == u64::MAX {
            return None;
        }
        let next_key = rep_key + 1;
        let (_, next_bucket_ptr) = self.reps.lower_bound(&next_key)?;
        let next_bucket = unsafe { next_bucket_ptr.as_ref() };
        next_bucket.first_entry()
    }
}
