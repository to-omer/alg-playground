use std::ops::{Bound, RangeBounds};

use crate::policy::LazyMapMonoid;
use crate::traits::{SequenceAgg, SequenceBase, SequenceLazy, SequenceReverse, SequenceSplitMerge};

pub struct ImplicitRbTree<P: LazyMapMonoid> {
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
    red: bool,
    black_height: u32,
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
            red: self.red,
            black_height: self.black_height,
            left: self.left.clone(),
            right: self.right.clone(),
        }
    }
}

impl<P: LazyMapMonoid> Node<P> {
    fn new(key: P::Key, red: bool) -> Self {
        let agg = P::agg_from_key(&key);
        Self {
            key,
            agg: agg.clone(),
            agg_rev: agg,
            lazy: P::act_unit(),
            rev: false,
            size: 1,
            red,
            black_height: if red { 0 } else { 1 },
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
        self.recalc_black_height();
    }

    fn recalc_black_height(&mut self) {
        let left_bh = self.left.as_ref().map(|n| n.black_height).unwrap_or(0);
        let right_bh = self.right.as_ref().map(|n| n.black_height).unwrap_or(0);
        let child_bh = if left_bh >= right_bh {
            left_bh
        } else {
            right_bh
        };
        self.black_height = child_bh + if self.red { 0 } else { 1 };
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

impl<P: LazyMapMonoid> ImplicitRbTree<P> {
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

    fn is_red(node: &Link<P>) -> bool {
        node.as_ref().map(|n| n.red).unwrap_or(false)
    }

    fn make_black(mut root: Link<P>) -> Link<P> {
        if let Some(node) = root.as_deref_mut()
            && node.red
        {
            node.red = false;
            node.recalc_black_height();
        }
        root
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

    fn fix_up(mut node: Box<Node<P>>) -> Box<Node<P>> {
        if node.red {
            node.recalc();
            return node;
        }

        let left_red = Self::is_red(&node.left);
        let right_red = Self::is_red(&node.right);
        let left_left_red = node
            .left
            .as_ref()
            .and_then(|left| left.left.as_ref())
            .map(|left| left.red)
            .unwrap_or(false);
        let left_right_red = node
            .left
            .as_ref()
            .and_then(|left| left.right.as_ref())
            .map(|right| right.red)
            .unwrap_or(false);
        let right_left_red = node
            .right
            .as_ref()
            .and_then(|right| right.left.as_ref())
            .map(|left| left.red)
            .unwrap_or(false);
        let right_right_red = node
            .right
            .as_ref()
            .and_then(|right| right.right.as_ref())
            .map(|right| right.red)
            .unwrap_or(false);

        if left_red && (left_left_red || left_right_red) {
            if left_right_red {
                let left = node.left.take().map(Self::rotate_left);
                node.left = left;
            }
            let mut root = Self::rotate_right(node);
            root.red = true;
            if let Some(left) = root.left.as_deref_mut() {
                left.red = false;
                left.recalc();
            }
            if let Some(right) = root.right.as_deref_mut() {
                right.red = false;
                right.recalc();
            }
            root.recalc();
            return root;
        }

        if right_red && (right_left_red || right_right_red) {
            if right_left_red {
                let right = node.right.take().map(Self::rotate_right);
                node.right = right;
            }
            let mut root = Self::rotate_left(node);
            root.red = true;
            if let Some(left) = root.left.as_deref_mut() {
                left.red = false;
                left.recalc();
            }
            if let Some(right) = root.right.as_deref_mut() {
                right.red = false;
                right.recalc();
            }
            root.recalc();
            return root;
        }

        node.recalc();
        node
    }

    fn join_left(
        mut left: Box<Node<P>>,
        key: P::Key,
        right: Link<P>,
        target_bh: u32,
    ) -> Box<Node<P>> {
        left.push();
        let right_bh = left.right.as_ref().map(|n| n.black_height).unwrap_or(0);
        if right_bh == target_bh {
            let mut node = Box::new(Node::new(key, true));
            node.left = left.right.take();
            node.right = right;
            node.recalc();
            left.right = Some(node);
            return Self::fix_up(left);
        }
        if left.right.is_none() {
            let mut node = Box::new(Node::new(key, true));
            node.left = None;
            node.right = right;
            node.recalc();
            left.right = Some(node);
            return Self::fix_up(left);
        }
        let right_child = left.right.take().expect("join_left expects right spine");
        let merged = Self::join_left(right_child, key, right, target_bh);
        left.right = Some(merged);
        Self::fix_up(left)
    }

    fn join_right(
        mut right: Box<Node<P>>,
        key: P::Key,
        left: Link<P>,
        target_bh: u32,
    ) -> Box<Node<P>> {
        right.push();
        let left_bh = right.left.as_ref().map(|n| n.black_height).unwrap_or(0);
        if left_bh == target_bh {
            let mut node = Box::new(Node::new(key, true));
            node.left = left;
            node.right = right.left.take();
            node.recalc();
            right.left = Some(node);
            return Self::fix_up(right);
        }
        if right.left.is_none() {
            let mut node = Box::new(Node::new(key, true));
            node.left = left;
            node.right = None;
            node.recalc();
            right.left = Some(node);
            return Self::fix_up(right);
        }
        let left_child = right.left.take().expect("join_right expects left spine");
        let merged = Self::join_right(left_child, key, left, target_bh);
        right.left = Some(merged);
        Self::fix_up(right)
    }

    fn join(left: Link<P>, key: P::Key, right: Link<P>) -> Link<P> {
        match (left, right) {
            (None, None) => Some(Box::new(Node::new(key, false))),
            (None, Some(right)) => {
                let merged = Self::join_right(right, key, None, 0);
                Self::make_black(Some(merged))
            }
            (Some(left), None) => {
                let merged = Self::join_left(left, key, None, 0);
                Self::make_black(Some(merged))
            }
            (Some(left), Some(right)) => {
                let bh_left = left.black_height;
                let bh_right = right.black_height;
                if bh_left == bh_right {
                    let mut node = Box::new(Node::new(key, true));
                    node.left = Some(left);
                    node.right = Some(right);
                    node.recalc();
                    Self::make_black(Some(Self::fix_up(node)))
                } else if bh_left > bh_right {
                    let merged = Self::join_left(left, key, Some(right), bh_right);
                    Self::make_black(Some(merged))
                } else {
                    let merged = Self::join_right(right, key, Some(left), bh_left);
                    Self::make_black(Some(merged))
                }
            }
        }
    }

    fn pop_last(root: Link<P>) -> (Link<P>, Option<P::Key>) {
        let len = Node::size(&root);
        if len == 0 {
            return (None, None);
        }
        let (left, right) = Self::split(root, len - 1);
        let key = right.map(|node| node.key);
        (left, key)
    }

    fn split(root: Link<P>, left_count: usize) -> (Link<P>, Link<P>) {
        let mut node = match root {
            Some(node) => node,
            None => return (None, None),
        };

        node.push();
        let left_size = Node::size(&node.left);
        if left_count <= left_size {
            let (left, right) = Self::split(node.left.take(), left_count);
            node.left = right;
            let node = Self::fix_up(node);
            (left, Some(node))
        } else {
            let (left, right) = Self::split(node.right.take(), left_count - left_size - 1);
            node.right = left;
            let node = Self::fix_up(node);
            (Some(node), right)
        }
    }

    fn merge(left: Link<P>, right: Link<P>) -> Link<P> {
        match (left, right) {
            (None, right) => right,
            (left, None) => left,
            (Some(left), Some(right)) => {
                let (left, key) = Self::pop_last(Some(left));
                match key {
                    Some(key) => Self::join(left, key, Some(right)),
                    None => Some(right),
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

impl<P> Clone for ImplicitRbTree<P>
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

impl<P: LazyMapMonoid> Default for ImplicitRbTree<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: LazyMapMonoid> SequenceBase for ImplicitRbTree<P> {
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
        let (left, right) = Self::split(self.root.take(), index);
        self.root = Self::join(left, key, right);
        self.len += 1;
    }

    fn remove(&mut self, index: usize) -> Option<Self::Key> {
        if index >= self.len {
            return None;
        }
        let (left, rest) = Self::split(self.root.take(), index);
        let (mid, right) = Self::split(rest, 1);
        let removed = mid.map(|node| node.key);
        self.root = Self::merge(left, right);
        self.len -= 1;
        removed
    }
}

impl<P: LazyMapMonoid> SequenceSplitMerge for ImplicitRbTree<P> {
    fn split_at(&mut self, index: usize) -> Self {
        let (left, right) = Self::split(self.root.take(), index.min(self.len));
        self.root = Self::make_black(left);
        self.len = self.root.as_ref().map(|node| node.size).unwrap_or(0);
        let right = Self::make_black(right);
        let len = right.as_ref().map(|node| node.size).unwrap_or(0);
        Self { root: right, len }
    }

    fn merge(&mut self, right: Self) {
        self.root = Self::merge(self.root.take(), right.root);
        self.len = self.root.as_ref().map(|node| node.size).unwrap_or(0);
    }
}

impl<P: LazyMapMonoid> SequenceAgg for ImplicitRbTree<P> {
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

impl<P: LazyMapMonoid> SequenceLazy for ImplicitRbTree<P> {
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

impl<P: LazyMapMonoid> SequenceReverse for ImplicitRbTree<P> {
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
    use super::ImplicitRbTree;
    use crate::policy::RangeSumRangeAdd;
    use crate::traits::{SequenceAgg, SequenceBase, SequenceLazy, SequenceReverse};
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    #[test]
    fn insert_and_get() {
        let mut tree = ImplicitRbTree::<RangeSumRangeAdd>::new();
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
        let mut tree = ImplicitRbTree::<RangeSumRangeAdd>::new();
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
