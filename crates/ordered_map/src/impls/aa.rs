use crate::OrderedMap;

pub struct AaTreeMap<K: Ord, V> {
    root: Link<K, V>,
    len: usize,
}

type Link<K, V> = Option<Box<Node<K, V>>>;

struct Node<K, V> {
    key: K,
    value: V,
    level: u8,
    left: Link<K, V>,
    right: Link<K, V>,
}

impl<K, V> Node<K, V> {
    fn new(key: K, value: V) -> Self {
        Self {
            key,
            value,
            level: 1,
            left: None,
            right: None,
        }
    }

    fn level(node: &Link<K, V>) -> u8 {
        node.as_ref().map(|n| n.level).unwrap_or(0)
    }
}

impl<K: Ord, V> AaTreeMap<K, V> {
    fn skew(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
        let left_level = Node::level(&node.left);
        if left_level != 0 && left_level == node.level {
            let mut left = node.left.take().unwrap();
            node.left = left.right.take();
            left.right = Some(node);
            return left;
        }
        node
    }

    fn split(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
        let right_right_level = node
            .right
            .as_ref()
            .and_then(|r| r.right.as_ref())
            .map(|rr| rr.level)
            .unwrap_or(0);
        if right_right_level != 0 && right_right_level == node.level {
            let mut right = node.right.take().unwrap();
            node.right = right.left.take();
            right.left = Some(node);
            right.level = right.level.saturating_add(1);
            return right;
        }
        node
    }

    fn decrease_level(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
        let should_be = Node::level(&node.left).min(Node::level(&node.right)) + 1;
        if should_be < node.level {
            node.level = should_be;
            if let Some(right) = node.right.as_deref_mut()
                && right.level > should_be
            {
                right.level = should_be;
            }
        }
        node
    }

    fn rebalance_after_delete(mut node: Box<Node<K, V>>) -> Box<Node<K, V>> {
        node = Self::decrease_level(node);
        node = Self::skew(node);

        if let Some(right) = node.right.take() {
            let right = Self::skew(right);
            node.right = Some(right);
        }

        if let Some(mut right) = node.right.take() {
            if let Some(right_right) = right.right.take() {
                let right_right = Self::skew(right_right);
                right.right = Some(right_right);
            }
            node.right = Some(right);
        }

        node = Self::split(node);

        if let Some(right) = node.right.take() {
            let right = Self::split(right);
            node.right = Some(right);
        }

        node
    }

    fn remove_min(mut node: Box<Node<K, V>>) -> (Link<K, V>, Box<Node<K, V>>) {
        if node.left.is_none() {
            let right = node.right.take();
            return (right, node);
        }

        let (new_left, min_node) = Self::remove_min(node.left.take().unwrap());
        node.left = new_left;
        let node = Self::rebalance_after_delete(node);
        (Some(node), min_node)
    }

    fn remove_max(mut node: Box<Node<K, V>>) -> (Link<K, V>, Box<Node<K, V>>) {
        if node.right.is_none() {
            let left = node.left.take();
            return (left, node);
        }

        let (new_right, max_node) = Self::remove_max(node.right.take().unwrap());
        node.right = new_right;
        let node = Self::rebalance_after_delete(node);
        (Some(node), max_node)
    }

    fn insert_node(root: Link<K, V>, key: K, value: V) -> (Link<K, V>, Option<V>, bool) {
        let Some(mut node) = root else {
            return (Some(Box::new(Node::new(key, value))), None, true);
        };

        let (old, inserted) = match key.cmp(&node.key) {
            std::cmp::Ordering::Less => {
                let (left, old, inserted) = Self::insert_node(node.left.take(), key, value);
                node.left = left;
                (old, inserted)
            }
            std::cmp::Ordering::Greater => {
                let (right, old, inserted) = Self::insert_node(node.right.take(), key, value);
                node.right = right;
                (old, inserted)
            }
            std::cmp::Ordering::Equal => {
                let old = std::mem::replace(&mut node.value, value);
                (Some(old), false)
            }
        };

        let node = Self::split(Self::skew(node));
        (Some(node), old, inserted)
    }

    fn remove_node(root: Link<K, V>, key: &K) -> (Link<K, V>, Option<V>, bool) {
        let Some(mut node) = root else {
            return (None, None, false);
        };

        let (removed, did_remove) = match key.cmp(&node.key) {
            std::cmp::Ordering::Less => {
                let (left, r, did) = Self::remove_node(node.left.take(), key);
                node.left = left;
                (r, did)
            }
            std::cmp::Ordering::Greater => {
                let (right, r, did) = Self::remove_node(node.right.take(), key);
                node.right = right;
                (r, did)
            }
            std::cmp::Ordering::Equal => {
                let removed_value = node.value;
                let removed = Some(removed_value);

                if node.left.is_none() && node.right.is_none() {
                    return (None, removed, true);
                }

                if node.left.is_none() {
                    let (new_right, succ) = Self::remove_min(node.right.take().unwrap());
                    node.right = new_right;
                    node.key = succ.key;
                    node.value = succ.value;
                } else {
                    let (new_left, pred) = Self::remove_max(node.left.take().unwrap());
                    node.left = new_left;
                    node.key = pred.key;
                    node.value = pred.value;
                }
                (removed, true)
            }
        };

        let node = Self::rebalance_after_delete(node);
        (Some(node), removed, did_remove)
    }
}

impl<K: Ord, V> OrderedMap for AaTreeMap<K, V> {
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
