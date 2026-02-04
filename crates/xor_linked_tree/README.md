# XOR Linked Tree Diameter

This crate benchmarks tree diameter computations across several adjacency
representations, including an XOR linked tree.

## Implementations

- `diameter_vec`: adjacency lists using `Vec<Vec<_>>`.
- `diameter_chinese`: forward-star style adjacency (head/next arrays).
- `diameter_csr`: compressed sparse row adjacency.
- `diameter_xor`: XOR linked tree with leaf pruning.

## Notes

All inputs are weighted undirected trees. Distances are accumulated with
saturating arithmetic to avoid overflow.

## References

- https://codeforces.com/blog/entry/135239
