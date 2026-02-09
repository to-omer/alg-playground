use crate::OrderedMap;

pub struct LlrbTreeMap<K: Ord, V> {
    root: Link<K, V>,
    len: usize,
}

type Link<K, V> = Option<Box<Node<K, V>>>;

struct Node<K, V> {
    key: K,
    value: V,
    red: bool,
    left: Link<K, V>,
    right: Link<K, V>,
}

impl<K, V> Node<K, V> {
    fn new(key: K, value: V, red: bool) -> Self {
        Self {
            key,
            value,
            red,
            left: None,
            right: None,
        }
    }
}

impl<K: Ord, V> LlrbTreeMap<K, V> {
    fn is_red(node: &Link<K, V>) -> bool {
        node.as_ref().map(|n| n.red).unwrap_or(false)
    }

    fn rotate_left(mut h: Box<Node<K, V>>) -> Box<Node<K, V>> {
        let mut x = h.right.take().expect("rotate_left requires right");
        h.right = x.left.take();
        x.left = Some(h);
        x.red = x.left.as_ref().unwrap().red;
        x.left.as_mut().unwrap().red = true;
        x
    }

    fn rotate_right(mut h: Box<Node<K, V>>) -> Box<Node<K, V>> {
        let mut x = h.left.take().expect("rotate_right requires left");
        h.left = x.right.take();
        x.right = Some(h);
        x.red = x.right.as_ref().unwrap().red;
        x.right.as_mut().unwrap().red = true;
        x
    }

    fn flip_colors(h: &mut Box<Node<K, V>>) {
        h.red = !h.red;
        if let Some(left) = h.left.as_deref_mut() {
            left.red = !left.red;
        }
        if let Some(right) = h.right.as_deref_mut() {
            right.red = !right.red;
        }
    }

    fn fix_up(mut h: Box<Node<K, V>>) -> Box<Node<K, V>> {
        if Self::is_red(&h.right) {
            h = Self::rotate_left(h);
        }
        if Self::is_red(&h.left) && h.left.as_ref().is_some_and(|l| Self::is_red(&l.left)) {
            h = Self::rotate_right(h);
        }
        if Self::is_red(&h.left) && Self::is_red(&h.right) {
            Self::flip_colors(&mut h);
        }
        h
    }

    fn move_red_left(mut h: Box<Node<K, V>>) -> Box<Node<K, V>> {
        Self::flip_colors(&mut h);
        if h.right.as_ref().is_some_and(|r| Self::is_red(&r.left)) {
            let right = h.right.take().unwrap();
            h.right = Some(Self::rotate_right(right));
            h = Self::rotate_left(h);
            Self::flip_colors(&mut h);
        }
        h
    }

    fn move_red_right(mut h: Box<Node<K, V>>) -> Box<Node<K, V>> {
        Self::flip_colors(&mut h);
        if h.left.as_ref().is_some_and(|l| Self::is_red(&l.left)) {
            h = Self::rotate_right(h);
            Self::flip_colors(&mut h);
        }
        h
    }

    fn insert_node(h: Link<K, V>, key: K, value: V) -> (Link<K, V>, Option<V>, bool) {
        let Some(mut h) = h else {
            return (Some(Box::new(Node::new(key, value, true))), None, true);
        };

        let (old, inserted) = match key.cmp(&h.key) {
            std::cmp::Ordering::Less => {
                let (left, old, inserted) = Self::insert_node(h.left.take(), key, value);
                h.left = left;
                (old, inserted)
            }
            std::cmp::Ordering::Greater => {
                let (right, old, inserted) = Self::insert_node(h.right.take(), key, value);
                h.right = right;
                (old, inserted)
            }
            std::cmp::Ordering::Equal => {
                let old = std::mem::replace(&mut h.value, value);
                (Some(old), false)
            }
        };

        let mut h = h;
        if Self::is_red(&h.right) && !Self::is_red(&h.left) {
            h = Self::rotate_left(h);
        }
        if Self::is_red(&h.left) && h.left.as_ref().is_some_and(|l| Self::is_red(&l.left)) {
            h = Self::rotate_right(h);
        }
        if Self::is_red(&h.left) && Self::is_red(&h.right) {
            Self::flip_colors(&mut h);
        }

        (Some(h), old, inserted)
    }

    fn delete_min_with_node(mut h: Box<Node<K, V>>) -> (Link<K, V>, Box<Node<K, V>>) {
        if h.left.is_none() {
            let right = h.right.take();
            return (right, h);
        }
        if !Self::is_red(&h.left) && !h.left.as_ref().is_some_and(|l| Self::is_red(&l.left)) {
            h = Self::move_red_left(h);
        }
        let (new_left, min_node) = Self::delete_min_with_node(h.left.take().unwrap());
        h.left = new_left;
        let h = Self::fix_up(h);
        (Some(h), min_node)
    }

    fn remove_node(h: Link<K, V>, key: &K) -> (Link<K, V>, Option<V>) {
        let Some(mut h) = h else {
            return (None, None);
        };
        let removed = if key.cmp(&h.key) == std::cmp::Ordering::Less {
            if h.left.is_none() {
                return (Some(h), None);
            }
            if !Self::is_red(&h.left) && !h.left.as_ref().is_some_and(|l| Self::is_red(&l.left)) {
                h = Self::move_red_left(h);
            }
            let (new_left, removed) = Self::remove_node(h.left.take(), key);
            h.left = new_left;
            removed
        } else {
            if Self::is_red(&h.left) {
                h = Self::rotate_right(h);
            }
            if key == &h.key && h.right.is_none() {
                return (None, Some(h.value));
            }
            if h.right.is_some()
                && !Self::is_red(&h.right)
                && !h.right.as_ref().is_some_and(|r| Self::is_red(&r.left))
            {
                h = Self::move_red_right(h);
            }

            if key == &h.key {
                let removed_value = h.value;
                let (new_right, min_node) =
                    Self::delete_min_with_node(h.right.take().expect("right exists"));
                h.right = new_right;
                h.key = min_node.key;
                h.value = min_node.value;
                Some(removed_value)
            } else {
                let (new_right, removed) = Self::remove_node(h.right.take(), key);
                h.right = new_right;
                removed
            }
        };

        let h = Self::fix_up(h);
        (Some(h), removed)
    }
}

impl<K: Ord, V> OrderedMap for LlrbTreeMap<K, V> {
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
        if let Some(r) = self.root.as_deref_mut() {
            r.red = false;
        }
        if inserted {
            self.len += 1;
        }
        old
    }

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
        let _ = self.root.as_ref()?;
        if !Self::is_red(&self.root.as_ref().unwrap().left)
            && !Self::is_red(&self.root.as_ref().unwrap().right)
        {
            self.root.as_deref_mut().unwrap().red = true;
        }

        let (root, removed) = Self::remove_node(self.root.take(), key);
        self.root = root;
        if let Some(r) = self.root.as_deref_mut() {
            r.red = false;
        }
        if removed.is_some() {
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
