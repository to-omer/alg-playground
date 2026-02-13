# SSSP (Directed, Non-negative)

This crate provides three single-source shortest path (SSSP) implementations for
non-negative weighted directed graphs.

## Implementations

- `dijkstra_binary_heap`: baseline Dijkstra using `BinaryHeap`.
- `dijkstra_radix_heap`: monotone Dijkstra using a radix heap (`u64` keys).
- `bmssp_paper`: recursive BMSSP-style implementation based on
  arXiv:2504.17033v2 (with constant-degree transformation, pivot reduction,
  base-case truncated Dijkstra, and a partial-order pull structure).
  The constant-degree preprocessing uses an adaptive conversion: low-degree
  graphs are kept as-is (already constant-degree), and high-degree vertices are
  expanded into zero-weight cycles with bounded per-slot edge fan-out.

## API

```rust
use sssp::{DirectedGraph, dijkstra_binary_heap, dijkstra_radix_heap, bmssp_paper};

let graph = DirectedGraph::from_edges(3, &[(0, 1, 2), (1, 2, 5), (0, 2, 10)]);
let source = 0;

let a = dijkstra_binary_heap(&graph, source);
let b = dijkstra_radix_heap(&graph, source);
let c = bmssp_paper(&graph, source);
assert_eq!(a, b);
assert_eq!(a, c);
```

`INF = u64::MAX / 4` is used for unreachable vertices.

## Benchmarks

`benches/sssp.rs` compares the three algorithms on 10 case families inspired by
library-checker `graph/shortest_path` generators:

- `sparse_random`
- `max_sparse_random`
- `max_dense_random`
- `max_dense_long`
- `max_dense_zero`
- `almost_line`
- `grid_random`
- `grid_swirl`
- `wrong_dijkstra_killer`
- `spfa_killer`

Sampling policy uses `Auto -> Flat` transition by problem size.

## References

- Ran Duan, Jiayi Mao, Xiao Mao, Xinkai Shu, Longhui Yin.
  *Breaking the Sorting Barrier for Directed Single-Source Shortest Paths*.
  arXiv:2504.17033v2.
- library-checker `graph/shortest_path` dataset generators.
