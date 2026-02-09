use crate::OrderedMap;

const ALPHA_NUM: usize = 2;
const ALPHA_DEN: usize = 3;

pub struct ScapegoatTreeMap<K: Ord, V> {
    root: Link<K, V>,
    n: usize,
    q: usize,
    scratch_nodes: Vec<*mut Node<K, V>>,
    scratch_links: Vec<*mut Link<K, V>>,
}

type Link<K, V> = Option<Box<Node<K, V>>>;

struct Node<K, V> {
    key: K,
    value: V,
    size: usize,
    left: Link<K, V>,
    right: Link<K, V>,
}

impl<K, V> Node<K, V> {
    fn new(key: K, value: V) -> Self {
        Self {
            key,
            value,
            size: 1,
            left: None,
            right: None,
        }
    }

    fn size(node: &Link<K, V>) -> usize {
        node.as_ref().map(|n| n.size).unwrap_or(0)
    }

    fn recalc(&mut self) {
        self.size = 1 + Self::size(&self.left) + Self::size(&self.right);
    }
}

impl<K: Ord, V> ScapegoatTreeMap<K, V> {
    fn is_unbalanced(node: &Node<K, V>) -> bool {
        let left = Node::size(&node.left);
        let right = Node::size(&node.right);
        left * ALPHA_DEN > node.size * ALPHA_NUM || right * ALPHA_DEN > node.size * ALPHA_NUM
    }

    fn allowed_depth(q: usize) -> usize {
        if q <= 1 {
            return 0;
        }
        let alpha = (ALPHA_NUM as f64) / (ALPHA_DEN as f64);
        let base = (1.0 / alpha).ln();
        ((q as f64).ln() / base).floor() as usize
    }

    fn flatten_inorder(node: Link<K, V>, out: &mut Vec<Option<Box<Node<K, V>>>>) {
        let Some(mut node) = node else {
            return;
        };
        let right = node.right.take();
        let left = node.left.take();
        Self::flatten_inorder(left, out);
        node.size = 1;
        out.push(Some(node));
        Self::flatten_inorder(right, out);
    }

    fn build_balanced(nodes: &mut [Option<Box<Node<K, V>>>]) -> Link<K, V> {
        if nodes.is_empty() {
            return None;
        }
        let mid = nodes.len() / 2;
        let mut root = nodes[mid].take().expect("node exists");
        root.left = Self::build_balanced(&mut nodes[..mid]);
        root.right = Self::build_balanced(&mut nodes[mid + 1..]);
        root.recalc();
        Some(root)
    }

    fn rebuild(link: &mut Link<K, V>) {
        let node = link.take();
        let mut nodes: Vec<Option<Box<Node<K, V>>>> = Vec::new();
        Self::flatten_inorder(node, &mut nodes);
        let new_root = Self::build_balanced(&mut nodes);
        *link = new_root;
    }

    fn pop_min(mut node: Box<Node<K, V>>) -> (Link<K, V>, Box<Node<K, V>>) {
        if node.left.is_none() {
            let right = node.right.take();
            return (right, node);
        }
        let (new_left, min_node) = Self::pop_min(node.left.take().unwrap());
        node.left = new_left;
        node.recalc();
        (Some(node), min_node)
    }

    fn remove_node(root: Link<K, V>, key: &K) -> (Link<K, V>, Option<V>, bool) {
        let Some(mut node) = root else {
            return (None, None, false);
        };

        match key.cmp(&node.key) {
            std::cmp::Ordering::Less => {
                let (left, removed, did) = Self::remove_node(node.left.take(), key);
                node.left = left;
                if did {
                    node.recalc();
                }
                (Some(node), removed, did)
            }
            std::cmp::Ordering::Greater => {
                let (right, removed, did) = Self::remove_node(node.right.take(), key);
                node.right = right;
                if did {
                    node.recalc();
                }
                (Some(node), removed, did)
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
                node.recalc();
                (Some(node), Some(removed_value), true)
            }
        }
    }
}

impl<K: Ord, V> OrderedMap for ScapegoatTreeMap<K, V> {
    type Key = K;
    type Value = V;

    fn new() -> Self {
        Self {
            root: None,
            n: 0,
            q: 0,
            scratch_nodes: Vec::with_capacity(64),
            scratch_links: Vec::with_capacity(64),
        }
    }

    fn len(&self) -> usize {
        self.n
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
        let mut depth = 0_usize;
        self.scratch_nodes.clear();
        self.scratch_links.clear();
        let path_nodes = &mut self.scratch_nodes;
        let path_links = &mut self.scratch_links;

        let mut link: *mut Link<K, V> = &mut self.root;
        unsafe {
            while let Some(node) = (*link).as_deref_mut() {
                path_nodes.push(node as *mut _);
                path_links.push(link);
                depth += 1;
                match key.cmp(&node.key) {
                    std::cmp::Ordering::Less => link = &mut node.left,
                    std::cmp::Ordering::Greater => link = &mut node.right,
                    std::cmp::Ordering::Equal => {
                        let old = std::mem::replace(&mut node.value, value);
                        return Some(old);
                    }
                }
            }
            *link = Some(Box::new(Node::new(key, value)));
        }

        self.n += 1;
        self.q = self.q.max(self.n);

        for &ptr in path_nodes.iter().rev() {
            unsafe {
                (*ptr).recalc();
            }
        }

        if depth > Self::allowed_depth(self.q) {
            let mut scapegoat_idx: Option<usize> = None;
            for (i, &ptr) in path_nodes.iter().enumerate().rev() {
                let node = unsafe { &*ptr };
                if Self::is_unbalanced(node) {
                    scapegoat_idx = Some(i);
                    break;
                }
            }

            if let Some(i) = scapegoat_idx {
                let scapegoat_link = path_links[i];
                unsafe {
                    Self::rebuild(&mut *scapegoat_link);
                }
                for &ptr in path_nodes[..i].iter().rev() {
                    unsafe {
                        (*ptr).recalc();
                    }
                }
            }
        }

        self.scratch_nodes.clear();
        self.scratch_links.clear();
        None
    }

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
        let (root, removed, did_remove) = Self::remove_node(self.root.take(), key);
        self.root = root;
        if did_remove {
            self.n -= 1;
            if self.n * ALPHA_DEN < self.q * ALPHA_NUM {
                Self::rebuild(&mut self.root);
                self.q = self.n;
            }
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
