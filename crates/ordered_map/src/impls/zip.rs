use crate::OrderedMap;

const DEFAULT_SEED: u64 = 0x5EED_21B7_2026;

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

    fn next_rank(&mut self) -> u8 {
        // Geometric-like: P(rank >= k) ~= 2^-k.
        let z = self.next_u64();
        let tz = z.trailing_zeros().min(63);
        tz as u8
    }
}

pub struct ZipTreeMap<K: Ord, V> {
    root: Link<K, V>,
    len: usize,
    rng: XorShift64,
}

type Link<K, V> = Option<Box<Node<K, V>>>;

struct Node<K, V> {
    key: K,
    value: V,
    rank: u8,
    left: Link<K, V>,
    right: Link<K, V>,
}

impl<K, V> Node<K, V> {
    fn new(key: K, value: V, rank: u8) -> Self {
        Self {
            key,
            value,
            rank,
            left: None,
            right: None,
        }
    }
}

impl<K: Ord, V> ZipTreeMap<K, V> {
    pub fn with_seed(seed: u64) -> Self {
        Self {
            root: None,
            len: 0,
            rng: XorShift64::new(seed),
        }
    }

    fn split_lt(root: Link<K, V>, key: &K) -> (Link<K, V>, Link<K, V>) {
        let Some(mut node) = root else {
            return (None, None);
        };
        if node.key < *key {
            let (a, b) = Self::split_lt(node.right.take(), key);
            node.right = a;
            (Some(node), b)
        } else {
            let (a, b) = Self::split_lt(node.left.take(), key);
            node.left = b;
            (a, Some(node))
        }
    }

    fn split_le(root: Link<K, V>, key: &K) -> (Link<K, V>, Link<K, V>) {
        let Some(mut node) = root else {
            return (None, None);
        };
        if node.key <= *key {
            let (a, b) = Self::split_le(node.right.take(), key);
            node.right = a;
            (Some(node), b)
        } else {
            let (a, b) = Self::split_le(node.left.take(), key);
            node.left = b;
            (a, Some(node))
        }
    }

    fn merge(a: Link<K, V>, b: Link<K, V>) -> Link<K, V> {
        match (a, b) {
            (None, b) => b,
            (a, None) => a,
            (Some(mut a), Some(mut b)) => {
                if a.rank >= b.rank {
                    let right = a.right.take();
                    a.right = Self::merge(right, Some(b));
                    Some(a)
                } else {
                    let left = b.left.take();
                    b.left = Self::merge(Some(a), left);
                    Some(b)
                }
            }
        }
    }

    fn remove_root(root: Node<K, V>) -> (Link<K, V>, V) {
        let Node {
            value, left, right, ..
        } = root;
        (Self::merge(left, right), value)
    }

    fn remove_node(root: Link<K, V>, key: &K) -> (Link<K, V>, Option<V>) {
        let (lt, ge) = Self::split_lt(root, key);
        let (eq, gt) = Self::split_le(ge, key);
        let Some(eq_root) = eq else {
            return (Self::merge(lt, gt), None);
        };
        let (eq_removed, value) = Self::remove_root(*eq_root);
        debug_assert!(eq_removed.is_none(), "duplicate keys in zip tree");
        (Self::merge(lt, gt), Some(value))
    }
}

impl<K: Ord, V> OrderedMap for ZipTreeMap<K, V> {
    type Key = K;
    type Value = V;

    fn new() -> Self {
        Self::with_seed(DEFAULT_SEED)
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
        // Avoid `remove+insert` (extra split/merge) on the common "new key" path.
        let (lt, ge) = Self::split_lt(self.root.take(), &key);
        let (eq, gt) = Self::split_le(ge, &key);

        if let Some(mut eq_root) = eq {
            let old = std::mem::replace(&mut eq_root.value, value);
            self.root = Self::merge(Self::merge(lt, Some(eq_root)), gt);
            return Some(old);
        }

        let rank = self.rng.next_rank();
        let new_node = Some(Box::new(Node::new(key, value, rank)));
        self.root = Self::merge(Self::merge(lt, new_node), gt);
        self.len += 1;
        None
    }

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
        let (root, removed) = Self::remove_node(self.root.take(), key);
        self.root = root;
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
