use std::ops::{Bound, RangeBounds};

use crate::policy::LazyMapMonoid;
use crate::traits::{SequenceAgg, SequenceBase, SequenceLazy, SequenceReverse, SequenceSplitMerge};

pub struct ImplicitAaTree<P: LazyMapMonoid> {
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
    level: u32,
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
            level: self.level,
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
            level: 1,
            left: None,
            right: None,
        }
    }

    fn size(node: &Link<P>) -> usize {
        node.as_ref().map(|n| n.size).unwrap_or(0)
    }

    fn level(node: &Link<P>) -> u32 {
        node.as_ref().map(|n| n.level).unwrap_or(0)
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

impl<P: LazyMapMonoid> ImplicitAaTree<P> {
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
        let mut left = root.left.take().expect("rotate_right needs left");
        left.push();
        root.left = left.right.take();
        root.recalc();
        left.right = Some(root);
        left.recalc();
        left
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

    fn skew(mut node: Box<Node<P>>) -> Box<Node<P>> {
        node.push();
        let left_level = Node::level(&node.left);
        if left_level == node.level {
            return Self::rotate_right(node);
        }
        node.recalc();
        node
    }

    fn split_level(mut node: Box<Node<P>>) -> Box<Node<P>> {
        node.push();
        let right_right_level = node
            .right
            .as_ref()
            .and_then(|right| right.right.as_ref())
            .map(|node| node.level)
            .unwrap_or(0);
        if right_right_level == node.level {
            let mut node = Self::rotate_left(node);
            node.level += 1;
            return node;
        }
        node.recalc();
        node
    }

    fn insert_node(root: Link<P>, index: usize, key: P::Key) -> Link<P> {
        let mut node = match root {
            Some(node) => node,
            None => return Some(Box::new(Node::new(key))),
        };

        node.push();
        let left_size = Node::size(&node.left);
        if index <= left_size {
            node.left = Self::insert_node(node.left.take(), index, key);
        } else {
            node.right = Self::insert_node(node.right.take(), index - left_size - 1, key);
        }

        node = Self::skew(node);
        node = Self::split_level(node);
        node.recalc();
        Some(node)
    }

    fn remove_min(root: Box<Node<P>>) -> (Link<P>, P::Key) {
        let mut root = root;
        root.push();
        if root.left.is_none() {
            return (root.right.take(), root.key);
        }
        let (left, key) = Self::remove_min(root.left.take().unwrap());
        root.left = left;
        let root = Self::decrease_level(root);
        let root = Self::fix_after_delete(root);
        (Some(root), key)
    }

    fn decrease_level(mut node: Box<Node<P>>) -> Box<Node<P>> {
        let left_level = Node::level(&node.left);
        let right_level = Node::level(&node.right);
        let expected = left_level.min(right_level) + 1;
        if expected < node.level {
            node.level = expected;
            if let Some(right) = node.right.as_deref_mut()
                && right.level > expected
            {
                right.level = expected;
            }
        }
        node
    }

    fn fix_after_delete(mut node: Box<Node<P>>) -> Box<Node<P>> {
        node = Self::skew(node);
        if let Some(right) = node.right.take() {
            let right = Self::skew(right);
            node.right = Some(right);
        }
        if let Some(mut right) = node.right.take() {
            if let Some(right_right) = right.right.take() {
                right.right = Some(Self::skew(right_right));
            }
            node.right = Some(right);
        }
        node = Self::split_level(node);
        if let Some(right) = node.right.take() {
            node.right = Some(Self::split_level(right));
        }
        node.recalc();
        node
    }

    fn remove_node(root: Link<P>, index: usize) -> (Link<P>, Option<P::Key>) {
        let mut node = match root {
            Some(node) => node,
            None => return (None, None),
        };

        node.push();
        let left_size = Node::size(&node.left);
        if index < left_size {
            let (left, removed) = Self::remove_node(node.left.take(), index);
            node.left = left;
            node = Self::decrease_level(node);
            node = Self::fix_after_delete(node);
            return (Some(node), removed);
        }

        if index > left_size {
            let (right, removed) = Self::remove_node(node.right.take(), index - left_size - 1);
            node.right = right;
            node = Self::decrease_level(node);
            node = Self::fix_after_delete(node);
            return (Some(node), removed);
        }

        let removed = node.key;
        if node.left.is_none() {
            return (node.right.take(), Some(removed));
        }
        if node.right.is_none() {
            return (node.left.take(), Some(removed));
        }

        let (right, successor) = Self::remove_min(node.right.take().unwrap());
        node.right = right;
        node.key = successor;
        node = Self::decrease_level(node);
        node = Self::fix_after_delete(node);
        (Some(node), Some(removed))
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
        if left_count >= node.size {
            return (Some(node), None);
        }

        let left_size = Node::size(&node.left);
        if left_count <= left_size {
            let (left, right) = Self::split(node.left.take(), left_count);
            node.left = right;
            node = Self::decrease_level(node);
            node = Self::fix_after_delete(node);
            (left, Some(node))
        } else {
            let (left, right) = Self::split(node.right.take(), left_count - left_size - 1);
            node.right = left;
            node = Self::decrease_level(node);
            node = Self::fix_after_delete(node);
            (Some(node), right)
        }
    }

    fn merge(left: Link<P>, right: Link<P>) -> Link<P> {
        match (left, right) {
            (None, right) => right,
            (left, None) => left,
            (Some(left), Some(right)) => {
                if left.level > right.level {
                    let mut left = left;
                    left.push();
                    left.right = Self::merge(left.right.take(), Some(right));
                    let mut left = Self::skew(left);
                    left = Self::split_level(left);
                    left.recalc();
                    Some(left)
                } else {
                    let mut right = right;
                    right.push();
                    right.left = Self::merge(Some(left), right.left.take());
                    let mut right = Self::skew(right);
                    right = Self::split_level(right);
                    right.recalc();
                    Some(right)
                }
            }
        }
    }

    fn get_node(node: &mut Link<P>, index: usize) -> Option<&P::Key> {
        let node_ref = node.as_deref_mut()?;
        node_ref.push();
        let left_size = Node::size(&node_ref.left);
        if index < left_size {
            Self::get_node(&mut node_ref.left, index)
        } else if index == left_size {
            Some(&node_ref.key)
        } else {
            Self::get_node(&mut node_ref.right, index - left_size - 1)
        }
    }
}

impl<P> Clone for ImplicitAaTree<P>
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

impl<P: LazyMapMonoid> Default for ImplicitAaTree<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: LazyMapMonoid> SequenceBase for ImplicitAaTree<P> {
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
        self.root = Self::insert_node(self.root.take(), index, key);
        self.len += 1;
    }

    fn remove(&mut self, index: usize) -> Option<Self::Key> {
        if index >= self.len {
            return None;
        }
        let (root, removed) = Self::remove_node(self.root.take(), index);
        self.root = root;
        self.len -= 1;
        removed
    }
}

impl<P: LazyMapMonoid> SequenceSplitMerge for ImplicitAaTree<P> {
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

impl<P: LazyMapMonoid> SequenceAgg for ImplicitAaTree<P> {
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

impl<P: LazyMapMonoid> SequenceLazy for ImplicitAaTree<P> {
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

impl<P: LazyMapMonoid> SequenceReverse for ImplicitAaTree<P> {
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
    use super::ImplicitAaTree;
    use crate::policy::RangeSumRangeAdd;
    use crate::traits::{SequenceAgg, SequenceBase, SequenceLazy, SequenceReverse};
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    #[test]
    fn insert_and_get() {
        let mut tree = ImplicitAaTree::<RangeSumRangeAdd>::new();
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
        let mut tree = ImplicitAaTree::<RangeSumRangeAdd>::new();
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
