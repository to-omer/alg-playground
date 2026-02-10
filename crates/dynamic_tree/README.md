# dynamic_tree

Dynamic tree data structures (dynamic forest) for algorithm experiments and benchmarking.

## Implementations

- Link-Cut Tree (splay-based): path operations (`path_fold/path_apply/path_kth`), vertex operations.
- Link-Cut Tree (splay-based, subtree-aware): additionally supports component/subtree operations
  (`component_fold/component_apply/subtree_*`) and is policy-parameterized with
  `LazyMapMonoid<Key = i64, Agg = i64, Act = i64>` (default: `VertexSumAdd`).
- Euler Tour Tree (splay-sequence): component/subtree operations (`component_fold/component_apply/subtree_*`), vertex operations.
- Self-adjusting Top Tree (rake/compress + splay): supports both path and component/subtree operations, and edge values (TopTree-only).

## Policy (Aggregate/Update Abstraction)

Most implementations are generic over `policy::LazyMapMonoid` (monoid + lazy action).
`LinkCutTreeSubtree<P>` is available for policies with `Key/Agg/Act = i64`
(`VertexSumAdd` by default).
Built-in policies include:

- `VertexSumAdd` (i64 sum + add)
- `VertexAffineSum` (i64 sum + affine)
- `PathComposite` ((a,b) affine composition along paths)

Traits for the abstract API live in `traits`:
`DynamicForest`, `VertexOps`, `PathOps`, `ComponentOps`, `SubtreeOps`.

Note: `TopTree`'s `path_apply/component_apply` are intended for additive-style actions.

## References

- Sleator & Tarjan (1983) Dynamic Trees (Link-Cut Tree)
- Tarjan (Euler Tour Tree variants)
- Tarjan & Werneck (2005) Self-adjusting Top Trees
