use std::ops::{Bound, RangeBounds};

use crate::policy::LazyMapMonoid;
use crate::traits::{SequenceAgg, SequenceBase, SequenceLazy, SequenceReverse, SequenceSplitMerge};

const DEFAULT_SEED: u64 = 0x5EED_71F5;

#[derive(Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        let mut z = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        self.state = z;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn fork(&mut self) -> Self {
        Self::new(self.next_u64())
    }
}

pub struct ImplicitZipTree<P: LazyMapMonoid> {
    root: Link<P>,
    len: u32,
    rng: SplitMix64,
}

struct Node<P: LazyMapMonoid> {
    key: P::Key,
    agg: P::Agg,
    agg_rev: P::Agg,
    lazy: P::Act,
    lazy_pending: bool,
    rev: bool,
    size: u32,
    left_size: u32,
    rank: u8,
    tie: u16,
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
            lazy_pending: self.lazy_pending,
            rev: self.rev,
            size: self.size,
            left_size: self.left_size,
            rank: self.rank,
            tie: self.tie,
            left: self.left.clone(),
            right: self.right.clone(),
        }
    }
}

impl<P: LazyMapMonoid> Node<P> {
    fn new(key: P::Key, rng: &mut SplitMix64) -> Self {
        let agg = P::agg_from_key(&key);
        let rand = rng.next_u64();
        let rank = rand.leading_zeros() as u8;
        let tie = (rand >> 32) as u16;
        Self {
            key,
            agg: agg.clone(),
            agg_rev: agg,
            lazy: P::act_unit(),
            lazy_pending: false,
            rev: false,
            size: 1,
            left_size: 0,
            rank,
            tie,
            left: None,
            right: None,
        }
    }

    fn size(node: &Link<P>) -> u32 {
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

        self.left_size = left_size;
        self.size = 1 + left_size + right_size;
        self.agg = P::agg_merge(&left_agg, &self.key, &right_agg);
        self.agg_rev = P::agg_merge(&right_rev, &self.key, &left_rev);
    }

    fn apply_action(&mut self, act: &P::Act) {
        self.key = P::act_apply_key(&self.key, act);
        let size = self.size as usize;
        self.agg = P::act_apply_agg(&self.agg, act, size);
        self.agg_rev = P::act_apply_agg(&self.agg_rev, act, size);
        self.lazy = P::act_compose(act, &self.lazy);
        self.lazy_pending = true;
    }

    fn apply_reverse(&mut self) {
        self.rev = !self.rev;
        std::mem::swap(&mut self.left, &mut self.right);
        std::mem::swap(&mut self.agg, &mut self.agg_rev);
        self.left_size = self.size - 1 - self.left_size;
    }

    fn push(&mut self) {
        if !self.rev && !self.lazy_pending {
            return;
        }
        if self.rev {
            if let Some(left) = self.left.as_deref_mut() {
                left.apply_reverse();
            }
            if let Some(right) = self.right.as_deref_mut() {
                right.apply_reverse();
            }
            self.rev = false;
        }

        if self.lazy_pending {
            if self.left.is_some() || self.right.is_some() {
                let act = self.lazy.clone();
                if let Some(left) = self.left.as_deref_mut() {
                    left.apply_action(&act);
                }
                if let Some(right) = self.right.as_deref_mut() {
                    right.apply_action(&act);
                }
            }
            self.lazy = P::act_unit();
            self.lazy_pending = false;
        }
    }

    fn higher_priority(&self, other: &Self) -> bool {
        (self.rank, self.tie) < (other.rank, other.tie)
    }
}

impl<P: LazyMapMonoid> ImplicitZipTree<P> {
    pub fn new() -> Self {
        Self::with_seed(DEFAULT_SEED)
    }

    pub fn with_seed(seed: u64) -> Self {
        Self {
            root: None,
            len: 0,
            rng: SplitMix64::new(seed),
        }
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

    fn fold_range(node: &mut Link<P>, start: usize, end: usize) -> P::Agg {
        if start >= end {
            return P::agg_unit();
        }
        let Some(node_ref) = node.as_deref_mut() else {
            return P::agg_unit();
        };
        let size = node_ref.size as usize;
        if start == 0 && end == size {
            return node_ref.agg.clone();
        }

        node_ref.push();
        let left_size = node_ref.left_size as usize;
        if end <= left_size {
            return Self::fold_range(&mut node_ref.left, start, end);
        }
        if start > left_size {
            return Self::fold_range(
                &mut node_ref.right,
                start - left_size - 1,
                end - left_size - 1,
            );
        }

        let left_agg = if start < left_size {
            Self::fold_range(&mut node_ref.left, start, left_size)
        } else {
            P::agg_unit()
        };
        let right_agg = if end > left_size + 1 {
            Self::fold_range(&mut node_ref.right, 0, end - left_size - 1)
        } else {
            P::agg_unit()
        };

        P::agg_merge(&left_agg, &node_ref.key, &right_agg)
    }

    fn update_range(node: &mut Link<P>, start: usize, end: usize, act: &P::Act) {
        if start >= end {
            return;
        }
        let Some(node_ref) = node.as_deref_mut() else {
            return;
        };
        let size = node_ref.size as usize;
        if start == 0 && end == size {
            node_ref.apply_action(act);
            return;
        }

        node_ref.push();
        let left_size = node_ref.left_size as usize;
        if start < left_size {
            let left_end = left_size.min(end);
            Self::update_range(&mut node_ref.left, start, left_end, act);
        }
        if start <= left_size && end > left_size {
            node_ref.key = P::act_apply_key(&node_ref.key, act);
        }
        if end > left_size + 1 {
            let right_start = if start > left_size + 1 {
                start - left_size - 1
            } else {
                0
            };
            let right_end = end - left_size - 1;
            Self::update_range(&mut node_ref.right, right_start, right_end, act);
        }

        node_ref.recalc();
    }

    fn rotate_left(mut root: Box<Node<P>>) -> Box<Node<P>> {
        root.push();
        let mut right = root.right.take().expect("rotate_left needs right");
        right.push();
        root.right = right.left.take();
        root.recalc();
        right.left = Some(root);
        right.recalc();
        right
    }

    fn rotate_right(mut root: Box<Node<P>>) -> Box<Node<P>> {
        root.push();
        let mut left = root.left.take().expect("rotate_right needs left");
        left.push();
        root.left = left.right.take();
        root.recalc();
        left.right = Some(root);
        left.recalc();
        left
    }

    fn insert_node(root: Link<P>, index: usize, key: P::Key, rng: &mut SplitMix64) -> Link<P> {
        let mut node = match root {
            Some(node) => node,
            None => return Some(Box::new(Node::new(key, rng))),
        };

        node.push();
        let left_size = node.left_size as usize;
        if index <= left_size {
            node.left = Self::insert_node(node.left.take(), index, key, rng);
            if let Some(left) = node.left.as_ref()
                && left.higher_priority(&node)
            {
                return Some(Self::rotate_right(node));
            }
        } else {
            node.right = Self::insert_node(node.right.take(), index - left_size - 1, key, rng);
            if let Some(right) = node.right.as_ref()
                && right.higher_priority(&node)
            {
                return Some(Self::rotate_left(node));
            }
        }
        node.recalc();
        Some(node)
    }

    fn remove_node(root: Link<P>, index: usize) -> (Link<P>, Option<P::Key>) {
        let mut node = match root {
            Some(node) => node,
            None => return (None, None),
        };

        node.push();
        let left_size = node.left_size as usize;
        if index < left_size {
            let (left, removed) = Self::remove_node(node.left.take(), index);
            node.left = left;
            node.recalc();
            return (Some(node), removed);
        }
        if index > left_size {
            let (right, removed) = Self::remove_node(node.right.take(), index - left_size - 1);
            node.right = right;
            node.recalc();
            return (Some(node), removed);
        }

        let removed = node.key;
        let merged = Self::merge(node.left.take(), node.right.take());
        (merged, Some(removed))
    }

    fn split(root: Link<P>, left_count: usize) -> (Link<P>, Link<P>) {
        let mut node = match root {
            Some(node) => node,
            None => return (None, None),
        };

        node.push();
        if left_count == 0 {
            return (None, Some(node));
        }
        if left_count >= node.size as usize {
            return (Some(node), None);
        }

        let left_size = node.left_size as usize;
        if left_count <= left_size {
            let (left, right) = Self::split(node.left.take(), left_count);
            node.left = right;
            node.recalc();
            (left, Some(node))
        } else {
            let (left, right) = Self::split(node.right.take(), left_count - left_size - 1);
            node.right = left;
            node.recalc();
            (Some(node), right)
        }
    }

    fn merge(left: Link<P>, right: Link<P>) -> Link<P> {
        match (left, right) {
            (None, right) => right,
            (left, None) => left,
            (Some(mut left), Some(mut right)) => {
                if left.higher_priority(&right) {
                    left.push();
                    left.right = Self::merge(left.right.take(), Some(right));
                    left.recalc();
                    Some(left)
                } else {
                    right.push();
                    right.left = Self::merge(Some(left), right.left.take());
                    right.recalc();
                    Some(right)
                }
            }
        }
    }

    fn get_node(node: &mut Link<P>, index: usize) -> Option<&P::Key> {
        let mut current = node.as_deref_mut()?;
        let mut index = index;
        loop {
            current.push();
            let left_size = current.left_size as usize;
            if index < left_size {
                current = current.left.as_deref_mut()?;
            } else if index == left_size {
                return Some(&current.key);
            } else {
                index -= left_size + 1;
                current = current.right.as_deref_mut()?;
            }
        }
    }
}

impl<P> Clone for ImplicitZipTree<P>
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
            rng: self.rng,
        }
    }
}

impl<P: LazyMapMonoid> Default for ImplicitZipTree<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: LazyMapMonoid> SequenceBase for ImplicitZipTree<P> {
    type Key = P::Key;

    fn len(&self) -> usize {
        self.len as usize
    }

    fn get(&mut self, index: usize) -> Option<&Self::Key> {
        if index >= self.len as usize {
            return None;
        }
        Self::get_node(&mut self.root, index)
    }

    fn insert(&mut self, index: usize, key: Self::Key) {
        if index > self.len as usize {
            return;
        }
        self.root = Self::insert_node(self.root.take(), index, key, &mut self.rng);
        self.len += 1;
    }

    fn remove(&mut self, index: usize) -> Option<Self::Key> {
        if index >= self.len as usize {
            return None;
        }
        let (root, removed) = Self::remove_node(self.root.take(), index);
        self.root = root;
        self.len -= 1;
        removed
    }
}

impl<P: LazyMapMonoid> SequenceSplitMerge for ImplicitZipTree<P> {
    fn split_at(&mut self, index: usize) -> Self {
        let (left, right) = Self::split(self.root.take(), index.min(self.len as usize));
        self.root = left;
        self.len = self.root.as_ref().map(|node| node.size).unwrap_or(0);
        let len = right.as_ref().map(|node| node.size).unwrap_or(0);
        Self {
            root: right,
            len,
            rng: self.rng.fork(),
        }
    }

    fn merge(&mut self, right: Self) {
        self.root = Self::merge(self.root.take(), right.root);
        self.len = self.root.as_ref().map(|node| node.size).unwrap_or(0);
    }
}

impl<P: LazyMapMonoid> SequenceAgg for ImplicitZipTree<P> {
    type Agg = P::Agg;

    fn fold<R: RangeBounds<usize>>(&mut self, range: R) -> Self::Agg {
        let Some((start, end)) = Self::normalize_range(range, self.len as usize) else {
            return P::agg_unit();
        };
        if start == end {
            return P::agg_unit();
        }

        Self::fold_range(&mut self.root, start, end)
    }
}

impl<P: LazyMapMonoid> SequenceLazy for ImplicitZipTree<P> {
    type Act = P::Act;

    fn update<R: RangeBounds<usize>>(&mut self, range: R, act: Self::Act) {
        let Some((start, end)) = Self::normalize_range(range, self.len as usize) else {
            return;
        };
        if start == end {
            return;
        }

        Self::update_range(&mut self.root, start, end, &act);
    }
}

impl<P: LazyMapMonoid> SequenceReverse for ImplicitZipTree<P> {
    fn reverse<R: RangeBounds<usize>>(&mut self, range: R) {
        let Some((start, end)) = Self::normalize_range(range, self.len as usize) else {
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
    use super::ImplicitZipTree;
    use crate::policy::RangeSumRangeAdd;
    use crate::traits::{SequenceAgg, SequenceBase, SequenceLazy, SequenceReverse};
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    #[test]
    fn insert_and_get() {
        let mut tree = ImplicitZipTree::<RangeSumRangeAdd>::with_seed(1);
        for i in 0..10 {
            tree.insert(i, i as i64);
        }
        assert_eq!(tree.len(), 10);
        for i in 0..10 {
            assert_eq!(tree.get(i), Some(&(i as i64)));
        }
    }

    #[test]
    fn random_operations_match_vec() {
        let mut rng = StdRng::seed_from_u64(0x5EED_2026);
        let mut tree = ImplicitZipTree::<RangeSumRangeAdd>::with_seed(2);
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
                    tree.insert(index, value);
                    vec.insert(index, value);
                }
                1 => {
                    if vec.is_empty() {
                        continue;
                    }
                    let index = rng.random_range(0..vec.len());
                    assert_eq!(tree.remove(index), Some(vec.remove(index)));
                }
                2 => {
                    if vec.is_empty() {
                        continue;
                    }
                    let index = rng.random_range(0..vec.len());
                    assert_eq!(tree.get(index), vec.get(index));
                }
                3 => {
                    if vec.is_empty() {
                        continue;
                    }
                    let l = rng.random_range(0..vec.len());
                    let r = rng.random_range((l + 1)..=vec.len());
                    let delta = rng.random_range(-100..=100);
                    tree.update(l..r, delta);
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
                    tree.reverse(l..r);
                    vec[l..r].reverse();
                }
                _ => {
                    if vec.is_empty() {
                        continue;
                    }
                    let l = rng.random_range(0..vec.len());
                    let r = rng.random_range((l + 1)..=vec.len());
                    let expected: i64 = vec[l..r].iter().sum();
                    assert_eq!(tree.fold(l..r), expected);
                }
            }
        }
    }
}
