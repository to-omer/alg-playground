use crate::OrderedMap;

pub struct SplayTreeMap<K: Ord, V> {
    root: Link<K, V>,
    len: usize,
}

type Link<K, V> = Option<Box<Node<K, V>>>;

struct Node<K, V> {
    key: K,
    value: V,
    left: Link<K, V>,
    right: Link<K, V>,
}

impl<K, V> Node<K, V> {
    fn new(key: K, value: V) -> Self {
        Self {
            key,
            value,
            left: None,
            right: None,
        }
    }
}

impl<K: Ord, V> SplayTreeMap<K, V> {
    fn rotate_right(mut root: Box<Node<K, V>>) -> Box<Node<K, V>> {
        let mut left = match root.left.take() {
            Some(node) => node,
            None => return root,
        };
        root.left = left.right.take();
        left.right = Some(root);
        left
    }

    fn rotate_left(mut root: Box<Node<K, V>>) -> Box<Node<K, V>> {
        let mut right = match root.right.take() {
            Some(node) => node,
            None => return root,
        };
        root.right = right.left.take();
        right.left = Some(root);
        right
    }

    fn splay(&mut self, root: Link<K, V>, key: &K) -> Link<K, V> {
        let mut root = root?;
        let mut left_head: Link<K, V> = None;
        let mut right_head: Link<K, V> = None;
        let mut left_tail: *mut Node<K, V> = std::ptr::null_mut();
        let mut right_tail: *mut Node<K, V> = std::ptr::null_mut();

        loop {
            match key.cmp(&root.key) {
                std::cmp::Ordering::Less => {
                    if root.left.is_none() {
                        break;
                    }
                    if let Some(left) = root.left.as_deref()
                        && key < &left.key
                    {
                        root = Self::rotate_right(root);
                        if root.left.is_none() {
                            break;
                        }
                    }

                    let next = root.left.take().unwrap();
                    if right_head.is_none() {
                        right_head = Some(root);
                        right_tail = right_head.as_deref_mut().unwrap();
                    } else {
                        unsafe {
                            (*right_tail).left = Some(root);
                            right_tail = (*right_tail).left.as_deref_mut().unwrap();
                        }
                    }
                    root = next;
                }
                std::cmp::Ordering::Greater => {
                    if root.right.is_none() {
                        break;
                    }
                    if let Some(right) = root.right.as_deref()
                        && key > &right.key
                    {
                        root = Self::rotate_left(root);
                        if root.right.is_none() {
                            break;
                        }
                    }

                    let next = root.right.take().unwrap();
                    if left_head.is_none() {
                        left_head = Some(root);
                        left_tail = left_head.as_deref_mut().unwrap();
                    } else {
                        unsafe {
                            (*left_tail).right = Some(root);
                            left_tail = (*left_tail).right.as_deref_mut().unwrap();
                        }
                    }
                    root = next;
                }
                std::cmp::Ordering::Equal => break,
            }
        }

        if left_head.is_none() {
            left_head = root.left.take();
        } else {
            unsafe {
                (*left_tail).right = root.left.take();
            }
        }
        if right_head.is_none() {
            right_head = root.right.take();
        } else {
            unsafe {
                (*right_tail).left = root.right.take();
            }
        }
        root.left = left_head;
        root.right = right_head;
        Some(root)
    }
}

impl<K: Ord, V> OrderedMap for SplayTreeMap<K, V> {
    type Key = K;
    type Value = V;

    fn new() -> Self {
        Self { root: None, len: 0 }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn get(&mut self, key: &Self::Key) -> Option<&Self::Value> {
        let root = self.root.take();
        self.root = self.splay(root, key);
        match self.root.as_deref() {
            Some(node) if key == &node.key => Some(&node.value),
            _ => None,
        }
    }

    fn insert(&mut self, key: Self::Key, value: Self::Value) -> Option<Self::Value> {
        let Some(root) = self.root.take() else {
            self.root = Some(Box::new(Node::new(key, value)));
            self.len = 1;
            return None;
        };

        let mut root = self.splay(Some(root), &key).unwrap();
        match key.cmp(&root.key) {
            std::cmp::Ordering::Equal => {
                let old = std::mem::replace(&mut root.value, value);
                self.root = Some(root);
                Some(old)
            }
            std::cmp::Ordering::Less => {
                let mut new_root = Box::new(Node::new(key, value));
                new_root.left = root.left.take();
                new_root.right = Some(root);
                self.root = Some(new_root);
                self.len += 1;
                None
            }
            std::cmp::Ordering::Greater => {
                let mut new_root = Box::new(Node::new(key, value));
                new_root.right = root.right.take();
                new_root.left = Some(root);
                self.root = Some(new_root);
                self.len += 1;
                None
            }
        }
    }

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
        let root = self.root.take();
        let mut root = self.splay(root, key)?;
        if key != &root.key {
            self.root = Some(root);
            return None;
        }

        let removed_value = root.value;
        let left = root.left.take();
        let right = root.right.take();

        self.root = if left.is_none() {
            right
        } else {
            let mut new_root = self.splay(left, key).expect("left non-empty");
            new_root.right = right;
            Some(new_root)
        };

        self.len -= 1;
        Some(removed_value)
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
