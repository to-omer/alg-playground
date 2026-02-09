use std::mem::MaybeUninit;

use crate::OrderedMap;

use super::FastHashMap;

const W: usize = 64;
const NIL: u32 = u32::MAX;

#[derive(Clone, Copy, PartialEq, Eq)]
struct ChildId(u32);

impl ChildId {
    const LEAF_FLAG: u32 = 1_u32 << 31;

    fn none() -> Self {
        Self(0)
    }

    fn internal(id: u32) -> Self {
        // `id + 1` must not set the leaf flag.
        debug_assert!(id < (Self::LEAF_FLAG - 1));
        Self(id + 1)
    }

    fn leaf(id: u32) -> Self {
        debug_assert!(id < (Self::LEAF_FLAG - 1));
        Self((id + 1) | Self::LEAF_FLAG)
    }

    fn is_none(self) -> bool {
        self.0 == 0
    }

    fn is_leaf(self) -> bool {
        (self.0 & Self::LEAF_FLAG) != 0
    }

    fn as_internal(self) -> u32 {
        debug_assert!(!self.is_none() && !self.is_leaf());
        self.0 - 1
    }

    fn as_leaf(self) -> u32 {
        debug_assert!(!self.is_none() && self.is_leaf());
        (self.0 & !Self::LEAF_FLAG) - 1
    }
}

#[derive(Clone, Copy)]
struct InternalNode {
    parent: u32, // NIL for root
    child: [ChildId; 2],
    jump: u32, // leaf id or NIL
}

struct LeafNode<V> {
    key: u64,
    value: MaybeUninit<V>,
    parent: u32, // NIL => free slot
    prev: u32,   // leaf id or NIL
    next: u32,   // leaf id or NIL
}

pub struct XFastTrieMap<V> {
    internals: Vec<InternalNode>,
    leaves: Vec<LeafNode<V>>,
    free_internals: Vec<u32>,
    free_leaves: Vec<u32>,
    tables: Vec<FastHashMap<u64, u32>>,
    head: u32,
    tail: u32,
    len: usize,
}

impl<V> XFastTrieMap<V> {
    pub fn new_with_capacity(capacity: usize) -> Self {
        let internals = vec![InternalNode {
            parent: NIL,
            child: [ChildId::none(), ChildId::none()],
            jump: NIL,
        }];

        let mut tables = (0..=W).map(|_| FastHashMap::default()).collect::<Vec<_>>();
        tables[0].insert(0, 0);
        tables[W].reserve(capacity);

        Self {
            internals,
            leaves: Vec::with_capacity(capacity),
            free_internals: Vec::new(),
            free_leaves: Vec::new(),
            tables,
            head: NIL,
            tail: NIL,
            len: 0,
        }
    }

    fn prefix(key: u64, depth: usize) -> u64 {
        if depth == 0 { 0 } else { key >> (W - depth) }
    }

    fn bit_at(key: u64, depth: usize) -> usize {
        debug_assert!(depth < W);
        ((key >> (W - 1 - depth)) & 1) as usize
    }

    pub(crate) fn max_entry(&mut self) -> Option<(&u64, &V)> {
        if self.len == 0 {
            return None;
        }
        debug_assert_ne!(self.tail, NIL);
        let leaf = &self.leaves[self.tail as usize];
        debug_assert_ne!(leaf.parent, NIL);
        let vref = unsafe { &*leaf.value.as_ptr() };
        Some((&leaf.key, vref))
    }

    fn alloc_internal(&mut self, parent: u32) -> u32 {
        if let Some(id) = self.free_internals.pop() {
            self.internals[id as usize] = InternalNode {
                parent,
                child: [ChildId::none(), ChildId::none()],
                jump: NIL,
            };
            id
        } else {
            let id = self.internals.len() as u32;
            self.internals.push(InternalNode {
                parent,
                child: [ChildId::none(), ChildId::none()],
                jump: NIL,
            });
            id
        }
    }

    fn alloc_leaf(&mut self, key: u64, value: V, parent: u32, prev: u32, next: u32) -> u32 {
        if let Some(id) = self.free_leaves.pop() {
            let slot = &mut self.leaves[id as usize];
            slot.key = key;
            slot.value = MaybeUninit::new(value);
            slot.parent = parent;
            slot.prev = prev;
            slot.next = next;
            id
        } else {
            let id = self.leaves.len() as u32;
            self.leaves.push(LeafNode {
                key,
                value: MaybeUninit::new(value),
                parent,
                prev,
                next,
            });
            id
        }
    }

    fn min_leaf_from(&self, mut child: ChildId) -> u32 {
        loop {
            if child.is_leaf() {
                return child.as_leaf();
            }
            let node = &self.internals[child.as_internal() as usize];
            child = if !node.child[0].is_none() {
                node.child[0]
            } else {
                node.child[1]
            };
            debug_assert!(!child.is_none(), "non-empty subtree");
        }
    }

    fn max_leaf_from(&self, mut child: ChildId) -> u32 {
        loop {
            if child.is_leaf() {
                return child.as_leaf();
            }
            let node = &self.internals[child.as_internal() as usize];
            child = if !node.child[1].is_none() {
                node.child[1]
            } else {
                node.child[0]
            };
            debug_assert!(!child.is_none(), "non-empty subtree");
        }
    }

    fn recompute_jump(&mut self, internal: u32) {
        let left = self.internals[internal as usize].child[0];
        let right = self.internals[internal as usize].child[1];
        let jump = if !left.is_none() && !right.is_none() {
            NIL
        } else if !left.is_none() {
            self.max_leaf_from(left)
        } else if !right.is_none() {
            self.min_leaf_from(right)
        } else {
            NIL
        };
        self.internals[internal as usize].jump = jump;
    }

    fn fix_jumps_upwards(&mut self, mut internal: u32) {
        while internal != NIL {
            self.recompute_jump(internal);
            let p = self.internals[internal as usize].parent;
            if p == NIL {
                break;
            }
            internal = p;
        }
    }

    fn successor_leaf_id(&mut self, key: u64) -> Option<u32> {
        if self.len == 0 {
            return None;
        }
        if let Some(&leaf) = self.tables[W].get(&key) {
            return Some(leaf);
        }

        let mut l = 0_usize;
        let mut h = W;
        let mut u = 0_u32; // root

        while h - l > 1 {
            let i = (l + h) / 2;
            let p = Self::prefix(key, i);
            if let Some(&id) = self.tables[i].get(&p) {
                u = id;
                l = i;
            } else {
                h = i;
            }
        }

        debug_assert!(l < W);
        let bit = ((key >> (W - l - 1)) & 1) as usize;
        let jump = self.internals[u as usize].jump;
        debug_assert_ne!(jump, NIL, "jump must exist on search boundary");

        let pred = if bit == 1 {
            jump
        } else {
            self.leaves[jump as usize].prev
        };

        let succ = if pred == NIL {
            self.head
        } else {
            self.leaves[pred as usize].next
        };

        if succ == NIL { None } else { Some(succ) }
    }
}

impl<V> Drop for XFastTrieMap<V> {
    fn drop(&mut self) {
        for leaf in &mut self.leaves {
            if leaf.parent != NIL {
                unsafe {
                    std::ptr::drop_in_place(leaf.value.as_mut_ptr());
                }
                leaf.parent = NIL;
            }
        }
    }
}

impl<V> OrderedMap for XFastTrieMap<V> {
    type Key = u64;
    type Value = V;

    fn new() -> Self {
        Self::new_with_capacity(0)
    }

    fn len(&self) -> usize {
        self.len
    }

    fn get(&mut self, key: &Self::Key) -> Option<&Self::Value> {
        let leaf_id = *self.tables[W].get(key)?;
        let leaf = &self.leaves[leaf_id as usize];
        debug_assert_ne!(leaf.parent, NIL);
        Some(unsafe { &*leaf.value.as_ptr() })
    }

    fn insert(&mut self, key: Self::Key, value: Self::Value) -> Option<Self::Value> {
        if let Some(&leaf_id) = self.tables[W].get(&key) {
            let leaf = &mut self.leaves[leaf_id as usize];
            debug_assert_ne!(leaf.parent, NIL);
            let old = unsafe { std::mem::replace(&mut *leaf.value.as_mut_ptr(), value) };
            return Some(old);
        }

        let succ = self.successor_leaf_id(key).unwrap_or(NIL);
        let pred = if succ == NIL {
            self.tail
        } else {
            self.leaves[succ as usize].prev
        };

        let leaf_id = self.alloc_leaf(key, value, 0, pred, succ);

        if pred == NIL {
            self.head = leaf_id;
        } else {
            self.leaves[pred as usize].next = leaf_id;
        }
        if succ == NIL {
            self.tail = leaf_id;
        } else {
            self.leaves[succ as usize].prev = leaf_id;
        }

        // 1) walk down until falling out of the trie
        let mut u = 0_u32;
        let mut depth = 0_usize;
        while depth < W {
            let bit = Self::bit_at(key, depth);
            let child = self.internals[u as usize].child[bit];
            if child.is_none() {
                break;
            }
            debug_assert!(
                !child.is_leaf(),
                "exact leaf must have been handled by leaf table"
            );
            u = child.as_internal();
            depth += 1;
        }

        // 2) add path to the leaf
        let mut parent = u;
        for d in depth..W {
            let bit = Self::bit_at(key, d);
            if d == W - 1 {
                self.internals[parent as usize].child[bit] = ChildId::leaf(leaf_id);
                self.leaves[leaf_id as usize].parent = parent;
                self.tables[W].insert(key, leaf_id);
                break;
            }

            let new_internal = self.alloc_internal(parent);
            self.internals[parent as usize].child[bit] = ChildId::internal(new_internal);
            let depth_next = d + 1;
            let pfx = Self::prefix(key, depth_next);
            self.tables[depth_next].insert(pfx, new_internal);
            parent = new_internal;
        }

        self.len += 1;
        let start = self.leaves[leaf_id as usize].parent;
        self.fix_jumps_upwards(start);
        None
    }

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
        let leaf_id = *self.tables[W].get(key)?;

        let (parent, prev, next, value) = {
            let leaf = &mut self.leaves[leaf_id as usize];
            debug_assert_ne!(leaf.parent, NIL);
            let value = unsafe { leaf.value.assume_init_read() };
            let parent = leaf.parent;
            let prev = leaf.prev;
            let next = leaf.next;
            leaf.parent = NIL;
            (parent, prev, next, value)
        };

        if prev == NIL {
            self.head = next;
        } else {
            self.leaves[prev as usize].next = next;
        }
        if next == NIL {
            self.tail = prev;
        } else {
            self.leaves[next as usize].prev = prev;
        }

        self.tables[W].remove(key);
        self.free_leaves.push(leaf_id);

        let mut current = ChildId::leaf(leaf_id);
        let mut depth = W;
        let mut parent_id = parent;
        let mut fix_from = 0_u32;

        while parent_id != NIL {
            let parent_node = &mut self.internals[parent_id as usize];
            if parent_node.child[0] == current {
                parent_node.child[0] = ChildId::none();
            } else if parent_node.child[1] == current {
                parent_node.child[1] = ChildId::none();
            }

            if depth < W {
                let pfx = Self::prefix(*key, depth);
                self.tables[depth].remove(&pfx);
                self.free_internals.push(current.as_internal());
            }

            if !parent_node.child[0].is_none() || !parent_node.child[1].is_none() {
                fix_from = parent_id;
                break;
            }

            if parent_node.parent == NIL {
                fix_from = 0;
                break;
            }

            current = ChildId::internal(parent_id);
            depth = depth.saturating_sub(1);
            parent_id = parent_node.parent;
        }

        self.len -= 1;
        if self.len == 0 {
            self.head = NIL;
            self.tail = NIL;
        }

        self.fix_jumps_upwards(fix_from);
        Some(value)
    }

    fn lower_bound(&mut self, key: &Self::Key) -> Option<(&Self::Key, &Self::Value)> {
        let leaf_id = self.successor_leaf_id(*key)?;
        let leaf = &self.leaves[leaf_id as usize];
        debug_assert_ne!(leaf.parent, NIL);
        Some((&leaf.key, unsafe { &*leaf.value.as_ptr() }))
    }
}
