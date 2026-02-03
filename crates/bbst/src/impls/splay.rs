use std::ops::{Bound, RangeBounds};

use crate::policy::LazyMapMonoid;
use crate::traits::{SequenceAgg, SequenceBase, SequenceLazy, SequenceReverse, SequenceSplitMerge};

pub struct ImplicitSplay<P: LazyMapMonoid> {
    root: Link<P>,
    len: usize,
}

struct Node<P: LazyMapMonoid> {
    key: P::Key,
    agg: P::Agg,
    agg_rev: P::Agg,
    lazy: P::Act,
    rev: bool,
    size: usize,
    left: Link<P>,
    right: Link<P>,
}

type Link<P> = Option<Box<Node<P>>>;

impl<P> Clone for Node<P>
where
    P: LazyMapMonoid,
    P::Key: Clone,
    P::Agg: Clone,
    P::Act: Clone,
{
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            agg: self.agg.clone(),
            agg_rev: self.agg_rev.clone(),
            lazy: self.lazy.clone(),
            rev: self.rev,
            size: self.size,
            left: self.left.clone(),
            right: self.right.clone(),
        }
    }
}

impl<P: LazyMapMonoid> Node<P> {
    fn new(key: P::Key) -> Self {
        let agg = P::agg_from_key(&key);
        Self {
            key,
            agg: agg.clone(),
            agg_rev: agg,
            lazy: P::act_unit(),
            rev: false,
            size: 1,
            left: None,
            right: None,
        }
    }

    fn size(node: &Link<P>) -> usize {
        node.as_ref().map(|n| n.size).unwrap_or(0)
    }

    fn agg(node: &Link<P>) -> P::Agg {
        node.as_ref()
            .map(|n| n.agg.clone())
            .unwrap_or_else(P::agg_unit)
    }

    fn agg_rev(node: &Link<P>) -> P::Agg {
        node.as_ref()
            .map(|n| n.agg_rev.clone())
            .unwrap_or_else(P::agg_unit)
    }

    fn recalc(&mut self) {
        let left_agg = Self::agg(&self.left);
        let right_agg = Self::agg(&self.right);
        let left_rev = Self::agg_rev(&self.left);
        let right_rev = Self::agg_rev(&self.right);
        let left_size = Self::size(&self.left);
        let right_size = Self::size(&self.right);

        self.size = 1 + left_size + right_size;
        self.agg = P::agg_merge(&left_agg, &self.key, &right_agg);
        self.agg_rev = P::agg_merge(&right_rev, &self.key, &left_rev);
    }

    fn apply_action(&mut self, act: &P::Act) {
        self.key = P::act_apply_key(&self.key, act);
        self.agg = P::act_apply_agg(&self.agg, act, self.size);
        self.agg_rev = P::act_apply_agg(&self.agg_rev, act, self.size);
        self.lazy = P::act_compose(act, &self.lazy);
    }

    fn apply_reverse(&mut self) {
        self.rev = !self.rev;
        std::mem::swap(&mut self.left, &mut self.right);
        std::mem::swap(&mut self.agg, &mut self.agg_rev);
    }

    fn push(&mut self) {
        if self.rev {
            if let Some(left) = self.left.as_deref_mut() {
                left.apply_reverse();
            }
            if let Some(right) = self.right.as_deref_mut() {
                right.apply_reverse();
            }
            self.rev = false;
        }

        let act = self.lazy.clone();
        if let Some(left) = self.left.as_deref_mut() {
            left.apply_action(&act);
        }
        if let Some(right) = self.right.as_deref_mut() {
            right.apply_action(&act);
        }
        self.lazy = P::act_unit();
    }
}

impl<P: LazyMapMonoid> ImplicitSplay<P> {
    pub fn new() -> Self {
        Self::with_seed(0)
    }

    pub fn with_seed(_seed: u64) -> Self {
        Self { root: None, len: 0 }
    }

    fn normalize_range<R: RangeBounds<usize>>(range: R, len: usize) -> Option<(usize, usize)> {
        let start = match range.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start.checked_add(1)?,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&end) => end.checked_add(1)?,
            Bound::Excluded(&end) => end,
            Bound::Unbounded => len,
        };

        if start > end || end > len {
            return None;
        }

        Some((start, end))
    }

    fn rotate_right(mut root: Box<Node<P>>) -> Box<Node<P>> {
        root.push();
        let mut left = match root.left.take() {
            Some(node) => node,
            None => return root,
        };
        left.push();
        root.left = left.right.take();
        root.recalc();
        left.right = Some(root);
        left.recalc();
        left
    }

    fn rotate_left(mut root: Box<Node<P>>) -> Box<Node<P>> {
        root.push();
        let mut right = match root.right.take() {
            Some(node) => node,
            None => return root,
        };
        right.push();
        root.right = right.left.take();
        root.recalc();
        right.left = Some(root);
        right.recalc();
        right
    }

    fn splay(root: Link<P>, index: usize) -> Link<P> {
        let mut root = root?;
        let mut index = index;
        let mut left_head: Link<P> = None;
        let mut right_head: Link<P> = None;
        let mut left_tail: *mut Node<P> = std::ptr::null_mut();
        let mut right_tail: *mut Node<P> = std::ptr::null_mut();
        let mut left_chain: Vec<*mut Node<P>> = Vec::new();
        let mut right_chain: Vec<*mut Node<P>> = Vec::new();

        loop {
            root.push();
            let left_size = Node::size(&root.left);
            if index < left_size {
                if root.left.is_none() {
                    break;
                }
                if let Some(left) = root.left.as_deref_mut() {
                    left.push();
                }
                let left_left_size = root
                    .left
                    .as_ref()
                    .map(|left| Node::size(&left.left))
                    .unwrap_or(0);
                if index < left_left_size {
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
                right_chain.push(right_tail);
                root = next;
            } else if index > left_size {
                index -= left_size + 1;
                if root.right.is_none() {
                    break;
                }
                if let Some(right) = root.right.as_deref_mut() {
                    right.push();
                }
                let right_left_size = root
                    .right
                    .as_ref()
                    .map(|right| Node::size(&right.left))
                    .unwrap_or(0);
                if index > right_left_size {
                    root = Self::rotate_left(root);
                    if root.right.is_none() {
                        break;
                    }
                    index = index.saturating_sub(right_left_size + 1);
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
                left_chain.push(left_tail);
                root = next;
            } else {
                break;
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

        for &node in left_chain.iter().rev() {
            unsafe {
                (*node).recalc();
            }
        }
        for &node in right_chain.iter().rev() {
            unsafe {
                (*node).recalc();
            }
        }
        root.recalc();
        Some(root)
    }

    fn split(root: Link<P>, left_count: usize) -> (Link<P>, Link<P>) {
        let Some(root) = root else {
            return (None, None);
        };
        let size = root.size;
        if left_count == 0 {
            return (None, Some(root));
        }
        if left_count >= size {
            return (Some(root), None);
        }

        let mut root = Self::splay(Some(root), left_count).unwrap();
        let left = root.left.take();
        root.recalc();
        (left, Some(root))
    }

    fn merge(left: Link<P>, right: Link<P>) -> Link<P> {
        match (left, right) {
            (None, right) => right,
            (left, None) => left,
            (Some(left), Some(right)) => {
                let size = left.size;
                let mut root = Self::splay(Some(left), size - 1).unwrap();
                root.right = Some(right);
                root.recalc();
                Some(root)
            }
        }
    }
}

impl<P> Clone for ImplicitSplay<P>
where
    P: LazyMapMonoid,
    P::Key: Clone,
    P::Agg: Clone,
    P::Act: Clone,
{
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            len: self.len,
        }
    }
}

impl<P: LazyMapMonoid> Default for ImplicitSplay<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: LazyMapMonoid> SequenceBase for ImplicitSplay<P> {
    type Key = P::Key;

    fn len(&self) -> usize {
        self.len
    }

    fn get(&mut self, index: usize) -> Option<&Self::Key> {
        if index >= self.len {
            return None;
        }
        self.root = Self::splay(self.root.take(), index);
        self.root.as_ref().map(|node| &node.key)
    }

    fn insert(&mut self, index: usize, key: Self::Key) {
        if index > self.len {
            return;
        }
        let node = Some(Box::new(Node::new(key)));
        let (left, right) = Self::split(self.root.take(), index);
        self.root = Self::merge(Self::merge(left, node), right);
        self.len += 1;
    }

    fn remove(&mut self, index: usize) -> Option<Self::Key> {
        if index >= self.len {
            return None;
        }
        let (left, rest) = Self::split(self.root.take(), index);
        let (target, right) = Self::split(rest, 1);
        self.root = Self::merge(left, right);
        self.len -= 1;
        target.map(|node| node.key)
    }
}

impl<P: LazyMapMonoid> SequenceSplitMerge for ImplicitSplay<P> {
    fn split_at(&mut self, index: usize) -> Self {
        let (left, right) = Self::split(self.root.take(), index.min(self.len));
        self.root = left;
        self.len = self.root.as_ref().map(|node| node.size).unwrap_or(0);
        let len = right.as_ref().map(|node| node.size).unwrap_or(0);
        Self { root: right, len }
    }

    fn merge(&mut self, right: Self) {
        self.root = Self::merge(self.root.take(), right.root);
        self.len = self.root.as_ref().map(|node| node.size).unwrap_or(0);
    }
}

impl<P: LazyMapMonoid> SequenceAgg for ImplicitSplay<P> {
    type Agg = P::Agg;

    fn fold<R: RangeBounds<usize>>(&mut self, range: R) -> Self::Agg {
        let Some((start, end)) = Self::normalize_range(range, self.len) else {
            return P::agg_unit();
        };
        if start == end {
            return P::agg_unit();
        }

        let (left, rest) = Self::split(self.root.take(), start);
        let (mid, right) = Self::split(rest, end - start);
        let agg = mid
            .as_ref()
            .map(|node| node.agg.clone())
            .unwrap_or_else(P::agg_unit);
        self.root = Self::merge(left, Self::merge(mid, right));
        agg
    }
}

impl<P: LazyMapMonoid> SequenceLazy for ImplicitSplay<P> {
    type Act = P::Act;

    fn update<R: RangeBounds<usize>>(&mut self, range: R, act: Self::Act) {
        let Some((start, end)) = Self::normalize_range(range, self.len) else {
            return;
        };
        if start == end {
            return;
        }

        let (left, rest) = Self::split(self.root.take(), start);
        let (mut mid, right) = Self::split(rest, end - start);
        if let Some(node) = mid.as_deref_mut() {
            node.apply_action(&act);
        }
        self.root = Self::merge(left, Self::merge(mid, right));
    }
}

impl<P: LazyMapMonoid> SequenceReverse for ImplicitSplay<P> {
    fn reverse<R: RangeBounds<usize>>(&mut self, range: R) {
        let Some((start, end)) = Self::normalize_range(range, self.len) else {
            return;
        };
        if start == end {
            return;
        }

        let (left, rest) = Self::split(self.root.take(), start);
        let (mut mid, right) = Self::split(rest, end - start);
        if let Some(node) = mid.as_deref_mut() {
            node.apply_reverse();
        }
        self.root = Self::merge(left, Self::merge(mid, right));
    }
}

#[cfg(test)]
mod tests {
    use super::ImplicitSplay;
    use crate::policy::RangeSumRangeAdd;
    use crate::traits::{SequenceAgg, SequenceBase, SequenceLazy, SequenceReverse};
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    #[test]
    fn insert_and_get() {
        let mut splay = ImplicitSplay::<RangeSumRangeAdd>::new();
        for i in 0..10 {
            splay.insert(i, i as i64);
        }
        assert_eq!(splay.len(), 10);
        for i in 0..10 {
            assert_eq!(splay.get(i), Some(&(i as i64)));
        }
    }

    #[test]
    fn random_operations_match_vec() {
        let mut rng = StdRng::seed_from_u64(0x5EED_2026);
        let mut splay = ImplicitSplay::<RangeSumRangeAdd>::new();
        let mut vec = Vec::<i64>::new();

        for _ in 0..2000 {
            let choice = rng.random_range(0..6);
            match choice {
                0 => {
                    let index = if vec.is_empty() {
                        0
                    } else {
                        rng.random_range(0..=vec.len())
                    };
                    let value = rng.random_range(-1000..=1000);
                    splay.insert(index, value);
                    vec.insert(index, value);
                }
                1 => {
                    if vec.is_empty() {
                        continue;
                    }
                    let index = rng.random_range(0..vec.len());
                    assert_eq!(splay.remove(index), Some(vec.remove(index)));
                }
                2 => {
                    if vec.is_empty() {
                        continue;
                    }
                    let index = rng.random_range(0..vec.len());
                    assert_eq!(splay.get(index), vec.get(index));
                }
                3 => {
                    if vec.is_empty() {
                        continue;
                    }
                    let l = rng.random_range(0..vec.len());
                    let r = rng.random_range((l + 1)..=vec.len());
                    let delta = rng.random_range(-100..=100);
                    splay.update(l..r, delta);
                    for value in &mut vec[l..r] {
                        *value += delta;
                    }
                }
                4 => {
                    if vec.is_empty() {
                        continue;
                    }
                    let l = rng.random_range(0..vec.len());
                    let r = rng.random_range((l + 1)..=vec.len());
                    splay.reverse(l..r);
                    vec[l..r].reverse();
                }
                _ => {
                    if vec.is_empty() {
                        continue;
                    }
                    let l = rng.random_range(0..vec.len());
                    let r = rng.random_range((l + 1)..=vec.len());
                    let expected: i64 = vec[l..r].iter().sum();
                    assert_eq!(splay.fold(l..r), expected);
                }
            }
        }
    }
}
