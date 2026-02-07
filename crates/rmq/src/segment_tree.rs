use std::ops::Range;

use crate::StaticRmq;
use crate::util::better_index_ordered;

const NONE: usize = usize::MAX;

#[inline(always)]
fn better_or_none_ordered(values: &[i64], a: usize, b: usize) -> usize {
    if a == NONE {
        return b;
    }
    if b == NONE {
        return a;
    }
    better_index_ordered(values, a, b)
}

#[derive(Clone, Debug)]
pub struct SegmentTreeRmq {
    values: Vec<i64>,
    size: usize,
    tree: Vec<usize>,
}

impl SegmentTreeRmq {
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl StaticRmq for SegmentTreeRmq {
    fn new(values: &[i64]) -> Self {
        let n = values.len();
        let values = values.to_vec();
        if n == 0 {
            return Self {
                values,
                size: 0,
                tree: Vec::new(),
            };
        }

        let size = n.next_power_of_two();
        let mut tree = vec![NONE; 2 * size];
        for i in 0..n {
            tree[size + i] = i;
        }
        for i in (1..size).rev() {
            tree[i] = better_or_none_ordered(&values, tree[2 * i], tree[2 * i + 1]);
        }

        Self { values, size, tree }
    }

    fn argmin(&self, range: Range<usize>) -> Option<usize> {
        let n = self.values.len();
        if range.start >= range.end || range.end > n {
            return None;
        }

        let mut l = range.start + self.size;
        let mut r = range.end + self.size;
        let mut left = NONE;
        let mut right = NONE;

        let values = &self.values;
        let tree = &self.tree;

        while l < r {
            if (l & 1) == 1 {
                left = better_or_none_ordered(values, left, tree[l]);
                l += 1;
            }
            if (r & 1) == 1 {
                r -= 1;
                right = better_or_none_ordered(values, tree[r], right);
            }
            l >>= 1;
            r >>= 1;
        }

        let ans = better_or_none_ordered(values, left, right);
        (ans != NONE).then_some(ans)
    }
}
