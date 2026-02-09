use crate::OrderedMap;

pub struct AvlTreeMap<K: Ord, V> {
    root: Link<K, V>,
    len: usize,
}

type Link<K, V> = Option<Box<Node<K, V>>>;

struct Node<K, V> {
    key: K,
    value: V,
    height: i16,
    left: Link<K, V>,
    right: Link<K, V>,
}

impl<K, V> Node<K, V> {
    fn new(key: K, value: V) -> Self {
        Self {
            key,
            value,
            height: 1,
            left: None,
            right: None,
        }
    }

    fn height(node: &Link<K, V>) -> i16 {
        node.as_ref().map(|n| n.height).unwrap_or(0)
    }

    fn recalc(&mut self) {
        let hl = Self::height(&self.left);
        let hr = Self::height(&self.right);
        self.height = 1 + hl.max(hr);
    }

    fn balance_factor(&self) -> i16 {
        Self::height(&self.left) - Self::height(&self.right)
    }
}

impl<K: Ord, V> AvlTreeMap<K, V> {
    fn rotate_right(mut root: Box<Node<K, V>>) -> Box<Node<K, V>> {
        let mut left = match root.left.take() {
            Some(node) => node,
            None => return root,
        };
        root.left = left.right.take();
        root.recalc();
        left.right = Some(root);
        left.recalc();
        left
    }

    fn rotate_left(mut root: Box<Node<K, V>>) -> Box<Node<K, V>> {
        let mut right = match root.right.take() {
            Some(node) => node,
            None => return root,
        };
        root.right = right.left.take();
        root.recalc();
        right.left = Some(root);
        right.recalc();
        right
    }

    fn rebalance(mut root: Box<Node<K, V>>) -> Box<Node<K, V>> {
        root.recalc();
        let bf = root.balance_factor();
        if bf > 1 {
            let left_bf = root.left.as_ref().map(|n| n.balance_factor()).unwrap_or(0);
            if left_bf < 0 {
                let left = root.left.take().unwrap();
                root.left = Some(Self::rotate_left(left));
            }
            return Self::rotate_right(root);
        }
        if bf < -1 {
            let right_bf = root.right.as_ref().map(|n| n.balance_factor()).unwrap_or(0);
            if right_bf > 0 {
                let right = root.right.take().unwrap();
                root.right = Some(Self::rotate_right(right));
            }
            return Self::rotate_left(root);
        }
        root
    }

    fn pop_min(mut node: Box<Node<K, V>>) -> (Link<K, V>, Box<Node<K, V>>) {
        if node.left.is_none() {
            let right = node.right.take();
            return (right, node);
        }
        let (new_left, min_node) = Self::pop_min(node.left.take().unwrap());
        node.left = new_left;
        let node = Self::rebalance(node);
        (Some(node), min_node)
    }

    fn insert_node(root: Link<K, V>, key: K, value: V) -> (Link<K, V>, Option<V>, bool) {
        let Some(mut node) = root else {
            return (Some(Box::new(Node::new(key, value))), None, true);
        };

        match key.cmp(&node.key) {
            std::cmp::Ordering::Less => {
                let (left, old, inserted) = Self::insert_node(node.left.take(), key, value);
                node.left = left;
                let node = Self::rebalance(node);
                (Some(node), old, inserted)
            }
            std::cmp::Ordering::Greater => {
                let (right, old, inserted) = Self::insert_node(node.right.take(), key, value);
                node.right = right;
                let node = Self::rebalance(node);
                (Some(node), old, inserted)
            }
            std::cmp::Ordering::Equal => {
                let old = std::mem::replace(&mut node.value, value);
                (Some(node), Some(old), false)
            }
        }
    }

    fn remove_node(root: Link<K, V>, key: &K) -> (Link<K, V>, Option<V>, bool) {
        let Some(mut node) = root else {
            return (None, None, false);
        };

        match key.cmp(&node.key) {
            std::cmp::Ordering::Less => {
                let (left, removed, did_remove) = Self::remove_node(node.left.take(), key);
                node.left = left;
                let node = Self::rebalance(node);
                (Some(node), removed, did_remove)
            }
            std::cmp::Ordering::Greater => {
                let (right, removed, did_remove) = Self::remove_node(node.right.take(), key);
                node.right = right;
                let node = Self::rebalance(node);
                (Some(node), removed, did_remove)
            }
            std::cmp::Ordering::Equal => {
                let removed_value = node.value;
                if node.left.is_none() {
                    return (node.right.take(), Some(removed_value), true);
                }
                if node.right.is_none() {
                    return (node.left.take(), Some(removed_value), true);
                }

                let (new_right, succ) = Self::pop_min(node.right.take().unwrap());
                node.right = new_right;
                node.key = succ.key;
                node.value = succ.value;
                let node = Self::rebalance(node);
                (Some(node), Some(removed_value), true)
            }
        }
    }
}

impl<K: Ord, V> OrderedMap for AvlTreeMap<K, V> {
    type Key = K;
    type Value = V;

    fn new() -> Self {
        Self { root: None, len: 0 }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn get(&mut self, key: &Self::Key) -> Option<&Self::Value> {
        let mut cur = self.root.as_deref();
        while let Some(node) = cur {
            match key.cmp(&node.key) {
                std::cmp::Ordering::Less => cur = node.left.as_deref(),
                std::cmp::Ordering::Greater => cur = node.right.as_deref(),
                std::cmp::Ordering::Equal => return Some(&node.value),
            }
        }
        None
    }

    fn insert(&mut self, key: Self::Key, value: Self::Value) -> Option<Self::Value> {
        let (root, old, inserted) = Self::insert_node(self.root.take(), key, value);
        self.root = root;
        if inserted {
            self.len += 1;
        }
        old
    }

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
        let (root, removed, did_remove) = Self::remove_node(self.root.take(), key);
        self.root = root;
        if did_remove {
            self.len -= 1;
        }
        removed
    }

    fn lower_bound(&mut self, key: &Self::Key) -> Option<(&Self::Key, &Self::Value)> {
        let mut cur = self.root.as_deref();
        let mut candidate = None;
        while let Some(node) = cur {
            match key.cmp(&node.key) {
                std::cmp::Ordering::Less | std::cmp::Ordering::Equal => {
                    candidate = Some(node);
                    cur = node.left.as_deref();
                }
                std::cmp::Ordering::Greater => cur = node.right.as_deref(),
            }
        }
        candidate.map(|n| (&n.key, &n.value))
    }
}
