use std::ops::{Bound, RangeBounds};

use crate::policy::LazyMapMonoid;
use crate::traits::{SequenceAgg, SequenceBase, SequenceLazy, SequenceReverse, SequenceSplitMerge};

const DEFAULT_SEED: u64 = 0x5EED_1B57;

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

    fn fork(&mut self) -> Self {
        Self::new(self.next_u64())
    }
}

pub struct ImplicitRbst<P: LazyMapMonoid> {
    root: Link<P>,
    len: usize,
    rng: XorShift64,
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
            lazy_pending: false,
            rev: false,
            size: 1,
            left_size: 0,
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
}

impl<P: LazyMapMonoid> ImplicitRbst<P> {
    pub fn new() -> Self {
        Self::with_seed(DEFAULT_SEED)
    }

    pub fn with_seed(seed: u64) -> Self {
        Self {
            root: None,
            len: 0,
            rng: XorShift64::new(seed),
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

    fn split(root: Link<P>, left_count: usize) -> (Link<P>, Link<P>) {
        let mut node = root;
        let mut left_count = left_count;
        let mut left_stack: Vec<Box<Node<P>>> = Vec::new();
        let mut right_stack: Vec<Box<Node<P>>> = Vec::new();
        let mut right = None;

        while let Some(mut current) = node {
            current.push();
            if left_count == 0 {
                right = Some(current);
                break;
            }
            let left_size = current.left_size as usize;
            if left_count <= left_size {
                let next = current.left.take();
                right_stack.push(current);
                node = next;
            } else {
                left_count -= left_size + 1;
                let next = current.right.take();
                left_stack.push(current);
                node = next;
            }
        }

        let mut left = None;
        while let Some(mut current) = left_stack.pop() {
            current.right = left;
            current.recalc();
            left = Some(current);
        }

        while let Some(mut current) = right_stack.pop() {
            current.left = right;
            current.recalc();
            right = Some(current);
        }

        (left, right)
    }

    fn merge_nodes(&mut self, left: Link<P>, right: Link<P>) -> Link<P> {
        match (left, right) {
            (None, right) => right,
            (left, None) => left,
            (Some(mut left), Some(mut right)) => {
                let left_size = left.size as usize;
                let right_size = right.size as usize;
                let total = left_size + right_size;
                let pick = ((self.rng.next_u64() as u128 * total as u128) >> 64) as usize;
                if pick < left_size {
                    left.push();
                    left.right = self.merge_nodes(left.right.take(), Some(right));
                    left.recalc();
                    Some(left)
                } else {
                    right.push();
                    right.left = self.merge_nodes(Some(left), right.left.take());
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

impl<P> Clone for ImplicitRbst<P>
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

impl<P: LazyMapMonoid> Default for ImplicitRbst<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: LazyMapMonoid> SequenceBase for ImplicitRbst<P> {
    type Key = P::Key;

    fn len(&self) -> usize {
        self.len
    }

    fn get(&mut self, index: usize) -> Option<&Self::Key> {
        if index >= self.len {
            return None;
        }
        Self::get_node(&mut self.root, index)
    }

    fn insert(&mut self, index: usize, key: Self::Key) {
        if index > self.len {
            return;
        }
        let node = Some(Box::new(Node::new(key)));
        let (left, right) = Self::split(self.root.take(), index);
        let merged = self.merge_nodes(left, node);
        self.root = self.merge_nodes(merged, right);
        self.len += 1;
    }

    fn remove(&mut self, index: usize) -> Option<Self::Key> {
        if index >= self.len {
            return None;
        }
        let (left, rest) = Self::split(self.root.take(), index);
        let (target, right) = Self::split(rest, 1);
        self.root = self.merge_nodes(left, right);
        self.len -= 1;
        target.map(|node| node.key)
    }
}

impl<P: LazyMapMonoid> SequenceSplitMerge for ImplicitRbst<P> {
    fn split_at(&mut self, index: usize) -> Self {
        let (left, right) = Self::split(self.root.take(), index.min(self.len));
        self.root = left;
        self.len = self
            .root
            .as_ref()
            .map(|node| node.size as usize)
            .unwrap_or(0);
        let len = right.as_ref().map(|node| node.size as usize).unwrap_or(0);
        Self {
            root: right,
            len,
            rng: self.rng.fork(),
        }
    }

    fn merge(&mut self, right: Self) {
        let root = self.root.take();
        self.root = self.merge_nodes(root, right.root);
        self.len = self
            .root
            .as_ref()
            .map(|node| node.size as usize)
            .unwrap_or(0);
    }
}

impl<P: LazyMapMonoid> SequenceAgg for ImplicitRbst<P> {
    type Agg = P::Agg;

    fn fold<R: RangeBounds<usize>>(&mut self, range: R) -> Self::Agg {
        let Some((start, end)) = Self::normalize_range(range, self.len) else {
            return P::agg_unit();
        };
        if start == end {
            return P::agg_unit();
        }

        Self::fold_range(&mut self.root, start, end)
    }
}

impl<P: LazyMapMonoid> SequenceLazy for ImplicitRbst<P> {
    type Act = P::Act;

    fn update<R: RangeBounds<usize>>(&mut self, range: R, act: Self::Act) {
        let Some((start, end)) = Self::normalize_range(range, self.len) else {
            return;
        };
        if start == end {
            return;
        }

        Self::update_range(&mut self.root, start, end, &act);
    }
}

impl<P: LazyMapMonoid> SequenceReverse for ImplicitRbst<P> {
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
        let merged = self.merge_nodes(mid, right);
        self.root = self.merge_nodes(left, merged);
    }
}

#[cfg(test)]
mod tests {
    use super::ImplicitRbst;
    use crate::policy::RangeSumRangeAdd;
    use crate::traits::{SequenceAgg, SequenceBase, SequenceLazy, SequenceReverse};
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    #[test]
    fn insert_and_get() {
        let mut tree = ImplicitRbst::<RangeSumRangeAdd>::new();
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
        let mut tree = ImplicitRbst::<RangeSumRangeAdd>::new();
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
