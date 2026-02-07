# rmq

Static Range Minimum Query (RMQ) implementations.

This crate provides multiple RMQ data structures for an immutable array `A[0..n)`.
All query ranges are half-open: `[l, r)`, and the result is the index of the
minimum element in the range. Ties are broken by the smallest index.

## Implementations

- `SegmentTreeRmq`: segment tree (build `O(n)`, query `O(log n)`).
- `SparseTableRmq`: sparse table (build `O(n log n)`, query `O(1)`).
- `DisjointSparseTableRmq`: disjoint sparse table (build `O(n log n)`, query `O(1)`).
- `AlstrupRmq`: block + bitmask micro-RMQ + sparse table on block minima
  (build `O(n)`, query `O(1)`), based on the idea described in the Qiita article
  below.

## References

- ScrubCrabClub, "Range Minimum Query" (Qiita).
  https://qiita.com/ScrubCrabClub/items/e2f4a734db4e5f90e410
- Disjoint sparse table: common competitive programming references (e.g. cp-algorithms).

