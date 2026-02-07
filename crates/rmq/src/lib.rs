mod alstrup;
mod disjoint_sparse_table;
mod segment_tree;
mod sparse_table;
mod util;

use std::ops::Range;

pub use alstrup::AlstrupRmq;
pub use disjoint_sparse_table::DisjointSparseTableRmq;
pub use segment_tree::SegmentTreeRmq;
pub use sparse_table::SparseTableRmq;

/// Static RMQ (Range Minimum Query) interface.
///
/// - Query ranges are half-open: `[l, r)`.
/// - The answer is `Some(argmin_index)` when the range is valid and non-empty.
/// - Ties are broken by the smallest index.
pub trait StaticRmq: Sized {
    fn new(values: &[i64]) -> Self;
    fn argmin(&self, range: Range<usize>) -> Option<usize>;
}

#[cfg(test)]
mod tests {
    use super::{AlstrupRmq, DisjointSparseTableRmq, SegmentTreeRmq, SparseTableRmq, StaticRmq};

    fn brute_force_argmin(values: &[i64], l: usize, r: usize) -> usize {
        debug_assert!(l < r);
        let mut best = l;
        for i in (l + 1)..r {
            let vi = values[i];
            let vb = values[best];
            if vi < vb || (vi == vb && i < best) {
                best = i;
            }
        }
        best
    }

    #[derive(Clone)]
    struct XorShift64 {
        state: u64,
    }

    impl XorShift64 {
        fn new(seed: u64) -> Self {
            Self { state: seed }
        }

        fn next_u64(&mut self) -> u64 {
            let mut x = self.state;
            x ^= x << 7;
            x ^= x >> 9;
            x ^= x << 8;
            self.state = x;
            x
        }

        fn gen_usize(&mut self, range: std::ops::Range<usize>) -> usize {
            debug_assert!(range.start < range.end);
            let span = (range.end - range.start) as u64;
            let x = self.next_u64() % span;
            range.start + (x as usize)
        }

        fn gen_i64(&mut self, range: std::ops::RangeInclusive<i64>) -> i64 {
            let start = *range.start();
            let end = *range.end();
            debug_assert!(start <= end);
            let span = (end as i128 - start as i128 + 1) as u64;
            let x = self.next_u64() % span;
            start + (x as i64)
        }
    }

    #[test]
    fn empty_returns_none() {
        let values: Vec<i64> = Vec::new();
        let seg = SegmentTreeRmq::new(&values);
        let st = SparseTableRmq::new(&values);
        let dst = DisjointSparseTableRmq::new(&values);
        let al = AlstrupRmq::new(&values);

        for rmq in [
            seg.argmin(0..0),
            st.argmin(0..0),
            dst.argmin(0..0),
            al.argmin(0..0),
        ] {
            assert_eq!(rmq, None);
        }
    }

    #[test]
    fn invalid_ranges_return_none() {
        let values = vec![5, 1, 4];
        let seg = SegmentTreeRmq::new(&values);
        assert_eq!(seg.argmin(1..1), None);
        assert_eq!(seg.argmin(3..3), None);
        assert_eq!(seg.argmin(0..4), None);

        let st = SparseTableRmq::new(&values);
        assert_eq!(st.argmin(1..1), None);
        assert_eq!(st.argmin(3..3), None);
        assert_eq!(st.argmin(0..4), None);

        let dst = DisjointSparseTableRmq::new(&values);
        assert_eq!(dst.argmin(1..1), None);
        assert_eq!(dst.argmin(3..3), None);
        assert_eq!(dst.argmin(0..4), None);

        let al = AlstrupRmq::new(&values);
        assert_eq!(al.argmin(1..1), None);
        assert_eq!(al.argmin(3..3), None);
        assert_eq!(al.argmin(0..4), None);
    }

    #[test]
    fn known_cases_match_bruteforce() {
        let cases: &[&[i64]] = &[
            &[1],
            &[2, 1],
            &[1, 2],
            &[2, 2],
            &[5, 1, 4, 1, 3],
            &[3, 2, 1, 0],
            &[0, 1, 2, 3],
            &[7, 7, 7, 7],
        ];

        for &values in cases {
            let seg = SegmentTreeRmq::new(values);
            let st = SparseTableRmq::new(values);
            let dst = DisjointSparseTableRmq::new(values);
            let al = AlstrupRmq::new(values);

            let n = values.len();
            for l in 0..n {
                for r in (l + 1)..=n {
                    let expected = brute_force_argmin(values, l, r);
                    assert_eq!(seg.argmin(l..r), Some(expected), "seg l={l} r={r}");
                    assert_eq!(st.argmin(l..r), Some(expected), "st l={l} r={r}");
                    assert_eq!(dst.argmin(l..r), Some(expected), "dst l={l} r={r}");
                    assert_eq!(al.argmin(l..r), Some(expected), "al l={l} r={r}");
                }
            }
        }
    }

    #[test]
    fn random_cases_match_bruteforce() {
        let mut rng = XorShift64::new(0xDEAD_BEEF_CAFE_BABE);

        for n in 0..64 {
            let mut values = Vec::with_capacity(n);
            for _ in 0..n {
                values.push(rng.gen_i64(-8..=8));
            }

            let seg = SegmentTreeRmq::new(&values);
            let st = SparseTableRmq::new(&values);
            let dst = DisjointSparseTableRmq::new(&values);
            let al = AlstrupRmq::new(&values);

            if n == 0 {
                continue;
            }

            for _ in 0..400 {
                let l = rng.gen_usize(0..n);
                let r = rng.gen_usize((l + 1)..(n + 1));
                let expected = brute_force_argmin(&values, l, r);

                assert_eq!(seg.argmin(l..r), Some(expected));
                assert_eq!(st.argmin(l..r), Some(expected));
                assert_eq!(dst.argmin(l..r), Some(expected));
                assert_eq!(al.argmin(l..r), Some(expected));
            }
        }
    }
}
