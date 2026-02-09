#![allow(clippy::vec_box)]

use crate::OrderedMap;

const MIN_DEGREE_CUSTOM: usize = 32;

pub struct BTreeMapCustom<K: Ord, V> {
    inner: BTreeMapBase<K, V, MIN_DEGREE_CUSTOM>,
}

impl<K: Ord, V> OrderedMap for BTreeMapCustom<K, V> {
    type Key = K;
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

pub(crate) struct BTreeMapBase<K: Ord, V, const T: usize> {
    root: Option<Box<Node<K, V, T>>>,
    len: usize,
}

impl<K: Ord, V, const T: usize> BTreeMapBase<K, V, T> {
    pub(crate) fn new() -> Self {
        Self { root: None, len: 0 }
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn get(&mut self, key: &K) -> Option<&V> {
        self.root.as_deref().and_then(|r| r.get(key))
    }

    pub(crate) fn lower_bound(&mut self, key: &K) -> Option<(&K, &V)> {
        self.root.as_deref().and_then(|r| r.lower_bound(key))
    }

    pub(crate) fn insert(&mut self, key: K, value: V) -> Option<V> {
        debug_assert!(T >= 2, "B-tree degree must be >= 2");
        if self.root.is_none() {
            self.root = Some(Box::new(Node::new_leaf(vec![key], vec![value])));
            self.len = 1;
            return None;
        }

        let root_full = self.root.as_ref().unwrap().is_full();
        if root_full {
            let old_root = self.root.take().unwrap();
            let mut new_root = Box::new(Node::new_internal(Vec::new(), Vec::new(), vec![old_root]));
            new_root.split_child(0);
            self.root = Some(new_root);
        }

        let (old, inserted) = self
            .root
            .as_deref_mut()
            .unwrap()
            .insert_non_full(key, value);
        if inserted {
            self.len += 1;
        }
        old
    }

    pub(crate) fn remove(&mut self, key: &K) -> Option<V> {
        let mut root = self.root.take()?;

        let removed = root.remove_from(key);
        if removed.is_none() {
            self.root = Some(root);
            return None;
        }

        self.len -= 1;
        if root.keys.is_empty() {
            // Shrink height.
            self.root = root.children.pop();
        } else {
            self.root = Some(root);
        }
        removed
    }
}

struct Node<K: Ord, V, const T: usize> {
    keys: Vec<K>,
    values: Vec<V>,
    children: Vec<Box<Node<K, V, T>>>,
}

impl<K: Ord, V, const T: usize> Node<K, V, T> {
    fn new_leaf(keys: Vec<K>, values: Vec<V>) -> Self {
        Self {
            keys,
            values,
            children: Vec::new(),
        }
    }

    fn new_internal(keys: Vec<K>, values: Vec<V>, children: Vec<Box<Node<K, V, T>>>) -> Self {
        Self {
            keys,
            values,
            children,
        }
    }

    fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    fn is_full(&self) -> bool {
        self.keys.len() == 2 * T - 1
    }

    fn find_index(&self, key: &K) -> Result<usize, usize> {
        self.keys.binary_search(key)
    }

    fn get(&self, key: &K) -> Option<&V> {
        match self.find_index(key) {
            Ok(i) => Some(&self.values[i]),
            Err(i) => {
                if self.is_leaf() {
                    None
                } else {
                    self.children[i].get(key)
                }
            }
        }
    }

    fn lower_bound(&self, key: &K) -> Option<(&K, &V)> {
        match self.find_index(key) {
            Ok(i) => Some((&self.keys[i], &self.values[i])),
            Err(i) => {
                if self.is_leaf() {
                    if i < self.keys.len() {
                        Some((&self.keys[i], &self.values[i]))
                    } else {
                        None
                    }
                } else {
                    if let Some(ans) = self.children[i].lower_bound(key) {
                        return Some(ans);
                    }
                    if i < self.keys.len() {
                        Some((&self.keys[i], &self.values[i]))
                    } else {
                        None
                    }
                }
            }
        }
    }

    fn split_child(&mut self, i: usize) {
        debug_assert!(self.children[i].is_full());
        let mut y = self.children.remove(i);

        let z_keys = y.keys.split_off(T);
        let median_key = y.keys.pop().expect("full child");

        let z_vals = y.values.split_off(T);
        let median_val = y.values.pop().expect("full child");

        let z_children = if y.is_leaf() {
            Vec::new()
        } else {
            y.children.split_off(T)
        };

        let z = Box::new(Node::new_internal(z_keys, z_vals, z_children));

        self.keys.insert(i, median_key);
        self.values.insert(i, median_val);
        self.children.insert(i, y);
        self.children.insert(i + 1, z);
    }

    fn insert_non_full(&mut self, key: K, value: V) -> (Option<V>, bool) {
        match self.find_index(&key) {
            Ok(i) => {
                let old = std::mem::replace(&mut self.values[i], value);
                (Some(old), false)
            }
            Err(mut i) => {
                if self.is_leaf() {
                    self.keys.insert(i, key);
                    self.values.insert(i, value);
                    return (None, true);
                }

                if self.children[i].is_full() {
                    self.split_child(i);
                    match key.cmp(&self.keys[i]) {
                        std::cmp::Ordering::Greater => i += 1,
                        std::cmp::Ordering::Equal => {
                            let old = std::mem::replace(&mut self.values[i], value);
                            return (Some(old), false);
                        }
                        std::cmp::Ordering::Less => {}
                    }
                }

                self.children[i].insert_non_full(key, value)
            }
        }
    }

    fn borrow_from_prev(&mut self, i: usize) {
        debug_assert!(i > 0);
        let (left, rest) = self.children.split_at_mut(i);
        let left_sib = left.last_mut().unwrap();
        let child = &mut rest[0];

        let sep_key = std::mem::replace(&mut self.keys[i - 1], left_sib.keys.pop().unwrap());
        let sep_val = std::mem::replace(&mut self.values[i - 1], left_sib.values.pop().unwrap());
        child.keys.insert(0, sep_key);
        child.values.insert(0, sep_val);

        if !left_sib.is_leaf() {
            let moved = left_sib.children.pop().unwrap();
            child.children.insert(0, moved);
        }
    }

    fn borrow_from_next(&mut self, i: usize) {
        debug_assert!(i < self.keys.len());
        let (child, rest) = self.children.split_at_mut(i + 1);
        let child = &mut child[i];
        let right_sib = &mut rest[0];

        let sep_key = std::mem::replace(&mut self.keys[i], right_sib.keys.remove(0));
        let sep_val = std::mem::replace(&mut self.values[i], right_sib.values.remove(0));
        child.keys.push(sep_key);
        child.values.push(sep_val);

        if !right_sib.is_leaf() {
            let moved = right_sib.children.remove(0);
            child.children.push(moved);
        }
    }

    fn merge_children(&mut self, i: usize) {
        debug_assert!(i < self.keys.len());

        let sep_key = self.keys.remove(i);
        let sep_val = self.values.remove(i);

        let mut right = self.children.remove(i + 1);
        let left = &mut self.children[i];

        left.keys.push(sep_key);
        left.values.push(sep_val);
        left.keys.append(&mut right.keys);
        left.values.append(&mut right.values);
        if !right.is_leaf() {
            left.children.append(&mut right.children);
        }
    }

    fn ensure_child_has_t(&mut self, i: usize) -> usize {
        if self.children[i].keys.len() >= T {
            return i;
        }

        if i > 0 && self.children[i - 1].keys.len() >= T {
            self.borrow_from_prev(i);
            return i;
        }

        if i < self.keys.len() && self.children[i + 1].keys.len() >= T {
            self.borrow_from_next(i);
            return i;
        }

        if i < self.keys.len() {
            self.merge_children(i);
            return i;
        }

        // Merge with previous (i == keys.len()).
        self.merge_children(i - 1);
        i - 1
    }

    fn pop_min(&mut self) -> (K, V) {
        if self.is_leaf() {
            let k = self.keys.remove(0);
            let v = self.values.remove(0);
            return (k, v);
        }

        let child_idx = self.ensure_child_has_t(0);
        let (k, v) = self.children[child_idx].pop_min();
        if self.children[child_idx].keys.is_empty() && !self.children[child_idx].is_leaf() {
            let promoted = self.children[child_idx].children.remove(0);
            self.children[child_idx] = promoted;
        }
        (k, v)
    }

    fn pop_max(&mut self) -> (K, V) {
        if self.is_leaf() {
            let k = self.keys.pop().unwrap();
            let v = self.values.pop().unwrap();
            return (k, v);
        }

        let last = self.keys.len();
        let child_idx = self.ensure_child_has_t(last);
        let (k, v) = self.children[child_idx].pop_max();
        if self.children[child_idx].keys.is_empty() && !self.children[child_idx].is_leaf() {
            let promoted = self.children[child_idx].children.remove(0);
            self.children[child_idx] = promoted;
        }
        (k, v)
    }

    fn remove_from(&mut self, key: &K) -> Option<V> {
        match self.find_index(key) {
            Ok(i) => {
                if self.is_leaf() {
                    self.keys.remove(i);
                    return Some(self.values.remove(i));
                }

                if self.children[i].keys.len() >= T {
                    let (pk, pv) = self.children[i].pop_max();
                    let removed = std::mem::replace(&mut self.values[i], pv);
                    self.keys[i] = pk;
                    return Some(removed);
                }

                if self.children[i + 1].keys.len() >= T {
                    let (sk, sv) = self.children[i + 1].pop_min();
                    let removed = std::mem::replace(&mut self.values[i], sv);
                    self.keys[i] = sk;
                    return Some(removed);
                }

                self.merge_children(i);
                let removed = self.children[i].remove_from(key);
                if self.children[i].keys.is_empty() && !self.children[i].is_leaf() {
                    let promoted = self.children[i].children.remove(0);
                    self.children[i] = promoted;
                }
                removed
            }
            Err(i) => {
                if self.is_leaf() {
                    return None;
                }

                let child_idx = self.ensure_child_has_t(i);
                let removed = self.children[child_idx].remove_from(key);
                if self.children[child_idx].keys.is_empty() && !self.children[child_idx].is_leaf() {
                    let promoted = self.children[child_idx].children.remove(0);
                    self.children[child_idx] = promoted;
                }
                removed
            }
        }
    }
}
