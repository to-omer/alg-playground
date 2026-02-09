use std::alloc::{self, Layout};
use std::ptr::NonNull;

use crate::OrderedMap;

const MAX_LEVEL: usize = 32;
const DEFAULT_SEED: u64 = 0x5EED_5A1B_2026;

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

    fn next_level(&mut self) -> usize {
        // Geometric distribution with p=1/2 using a single RNG call.
        // P(level >= k) = 2^{-(k-1)}.
        let x = self.next_u64();
        (x.trailing_ones() as usize + 1).min(MAX_LEVEL)
    }
}

type Link<K, V> = Option<NonNull<Node<K, V>>>;

struct Node<K, V> {
    key: K,
    value: V,
    level: u8,
}

pub struct SkipListMap<K: Ord, V> {
    head: [Link<K, V>; MAX_LEVEL],
    level: usize,
    len: usize,
    rng: XorShift64,
}

impl<K: Ord, V> SkipListMap<K, V> {
    pub fn with_seed(seed: u64) -> Self {
        Self {
            head: [None; MAX_LEVEL],
            level: 1,
            len: 0,
            rng: XorShift64::new(seed),
        }
    }

    fn node_key(ptr: NonNull<Node<K, V>>) -> *const K {
        // Helper to avoid repeating unsafe blocks.
        unsafe { std::ptr::addr_of!((*ptr.as_ptr()).key) }
    }

    #[inline(always)]
    fn links_offset() -> usize {
        let header_size = std::mem::size_of::<Node<K, V>>();
        let align = std::mem::align_of::<Link<K, V>>();
        debug_assert!(align.is_power_of_two());
        (header_size + align - 1) & !(align - 1)
    }

    #[inline(always)]
    fn node_layout(level: usize) -> Layout {
        let align = std::mem::align_of::<Node<K, V>>().max(std::mem::align_of::<Link<K, V>>());
        let size = Self::links_offset() + level * std::mem::size_of::<Link<K, V>>();
        Layout::from_size_align(size, align).unwrap()
    }

    #[inline(always)]
    unsafe fn node_links_ptr(node: *mut Node<K, V>) -> *mut Link<K, V> {
        unsafe { (node as *mut u8).add(Self::links_offset()) as *mut Link<K, V> }
    }

    #[inline(always)]
    unsafe fn node_get_next(node: *mut Node<K, V>, lvl: usize) -> Link<K, V> {
        unsafe {
            debug_assert!(lvl < (*node).level as usize);
            *Self::node_links_ptr(node).add(lvl)
        }
    }

    #[inline(always)]
    unsafe fn node_set_next(node: *mut Node<K, V>, lvl: usize, next: Link<K, V>) {
        unsafe {
            debug_assert!(lvl < (*node).level as usize);
            *Self::node_links_ptr(node).add(lvl) = next;
        }
    }

    unsafe fn alloc_node(key: K, value: V, level: usize) -> NonNull<Node<K, V>> {
        debug_assert!((1..=MAX_LEVEL).contains(&level));
        let layout = Self::node_layout(level);
        unsafe {
            let raw = alloc::alloc(layout);
            if raw.is_null() {
                alloc::handle_alloc_error(layout);
            }

            let node = raw as *mut Node<K, V>;
            std::ptr::write(
                node,
                Node {
                    key,
                    value,
                    level: level as u8,
                },
            );

            let links = Self::node_links_ptr(node);
            // Link is `Option<NonNull<_>>`, and all-zeros is `None`.
            std::ptr::write_bytes(links, 0, level);

            NonNull::new_unchecked(node)
        }
    }

    unsafe fn dealloc_node(ptr: NonNull<Node<K, V>>) {
        unsafe {
            let level = (*ptr.as_ptr()).level as usize;
            std::ptr::drop_in_place(ptr.as_ptr());
            alloc::dealloc(ptr.as_ptr() as *mut u8, Self::node_layout(level));
        }
    }

    unsafe fn dealloc_node_take_value(ptr: NonNull<Node<K, V>>) -> V {
        unsafe {
            let node = ptr.as_ptr();
            let level = (*node).level as usize;

            let value = std::ptr::read(std::ptr::addr_of!((*node).value));
            std::ptr::drop_in_place(std::ptr::addr_of_mut!((*node).key));
            alloc::dealloc(node as *mut u8, Self::node_layout(level));
            value
        }
    }

    fn find_update(&mut self, key: &K, update: &mut [*mut Node<K, V>]) -> Link<K, V> {
        debug_assert_eq!(update.len(), MAX_LEVEL);
        let mut cur: *mut Node<K, V> = std::ptr::null_mut();

        // Levels above `self.level` are currently empty.
        update[self.level..].fill(std::ptr::null_mut());

        for lvl in (0..self.level).rev() {
            let mut next = if cur.is_null() {
                self.head[lvl]
            } else {
                unsafe { Self::node_get_next(cur, lvl) }
            };
            while let Some(ptr) = next {
                let nkey = unsafe { &*Self::node_key(ptr) };
                if nkey < key {
                    cur = ptr.as_ptr();
                    next = unsafe { Self::node_get_next(cur, lvl) };
                } else {
                    break;
                }
            }
            update[lvl] = cur;
        }

        Self::get_next(&self.head, update[0], 0)
    }

    fn link_next(head: &mut [Link<K, V>], prev: *mut Node<K, V>, lvl: usize, next: Link<K, V>) {
        if prev.is_null() {
            head[lvl] = next;
        } else {
            unsafe {
                Self::node_set_next(prev, lvl, next);
            }
        }
    }

    fn get_next(head: &[Link<K, V>], prev: *mut Node<K, V>, lvl: usize) -> Link<K, V> {
        if prev.is_null() {
            head[lvl]
        } else {
            unsafe { Self::node_get_next(prev, lvl) }
        }
    }
}

impl<K: Ord, V> Drop for SkipListMap<K, V> {
    fn drop(&mut self) {
        let mut cur = self.head[0];
        while let Some(ptr) = cur {
            unsafe {
                cur = Self::node_get_next(ptr.as_ptr(), 0);
                Self::dealloc_node(ptr);
            }
        }
    }
}

impl<K: Ord, V> OrderedMap for SkipListMap<K, V> {
    type Key = K;
    type Value = V;

    fn new() -> Self {
        Self::with_seed(DEFAULT_SEED)
    }

    fn len(&self) -> usize {
        self.len
    }

    fn get(&mut self, key: &Self::Key) -> Option<&Self::Value> {
        let mut update: [*mut Node<K, V>; MAX_LEVEL] =
            std::array::from_fn(|_| std::ptr::null_mut());
        let next = self.find_update(key, &mut update);
        let ptr = next?;
        let node = unsafe { ptr.as_ref() };
        if &node.key == key {
            Some(&node.value)
        } else {
            None
        }
    }

    fn insert(&mut self, key: Self::Key, value: Self::Value) -> Option<Self::Value> {
        let mut update: [*mut Node<K, V>; MAX_LEVEL] =
            std::array::from_fn(|_| std::ptr::null_mut());
        let next = self.find_update(&key, &mut update);
        if let Some(ptr) = next {
            let node = unsafe { ptr.as_ref() };
            if node.key == key {
                let node_mut = unsafe { ptr.as_ptr().as_mut().unwrap() };
                let old = std::mem::replace(&mut node_mut.value, value);
                return Some(old);
            }
        }

        let level = self.rng.next_level();
        if level > self.level {
            self.level = level;
        }
        let ptr = unsafe { Self::alloc_node(key, value, level) };
        for (lvl, &prev) in update.iter().enumerate().take(level) {
            let next = Self::get_next(&self.head, prev, lvl);
            unsafe {
                Self::node_set_next(ptr.as_ptr(), lvl, next);
            }
        }

        for (lvl, &prev) in update.iter().enumerate().take(level) {
            Self::link_next(&mut self.head, prev, lvl, Some(ptr));
        }
        self.len += 1;
        None
    }

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
        let mut update: [*mut Node<K, V>; MAX_LEVEL] =
            std::array::from_fn(|_| std::ptr::null_mut());
        let next = self.find_update(key, &mut update);
        let ptr = next?;

        let found_key = unsafe { &*Self::node_key(ptr) };
        if found_key != key {
            return None;
        }

        let level = unsafe { (*ptr.as_ptr()).level as usize };
        for (lvl, &prev) in update.iter().enumerate().take(level) {
            let after = unsafe { Self::node_get_next(ptr.as_ptr(), lvl) };
            Self::link_next(&mut self.head, prev, lvl, after);
        }

        self.len -= 1;
        while self.level > 1 && self.head[self.level - 1].is_none() {
            self.level -= 1;
        }
        let value = unsafe { Self::dealloc_node_take_value(ptr) };
        Some(value)
    }

    fn lower_bound(&mut self, key: &Self::Key) -> Option<(&Self::Key, &Self::Value)> {
        let mut update: [*mut Node<K, V>; MAX_LEVEL] =
            std::array::from_fn(|_| std::ptr::null_mut());
        let next = self.find_update(key, &mut update);
        let ptr = next?;
        let node = unsafe { ptr.as_ref() };
        Some((&node.key, &node.value))
    }
}
