# Monge Row Minima

This crate implements algorithms for finding row minima in totally monotone
(Monge) matrices.

## Overview

This crate provides two related problems on Monge matrices.

1) **Row minima (offline)**
Find the column index that minimizes each row of a totally monotone matrix.

2) **Monge DAG shortest path (online)**
Compute shortest paths in a lower-triangular Monge DAG. This is the common
DP form used with LARSCH variants.

## Cost function contract

All APIs take `cost(row, col)`.

- `row` is the row index (destination).
- `col` is the column index (source).

For online shortest-path APIs, the matrix is lower-triangular. Use a guard
like `if col >= row { INF }` to represent invalid edges.

## Implementations

- `monotone_minima` (divide-and-conquer, O(N log N) for square matrices)
- `smawk` (linear time for totally monotone matrices)
- `simple_larsch_shortest_path` (online, O(N log N))
- `larsch_shortest_path` / `Larsch` (online, O(N))

## Benchmarks

- `row_minima_offline`: 全行列の行最小値（monotone/smawk）。
- `row_minima_online`: 下三角行列の最短路 DP（4 種）。
- `row_minima_offline_heavy` / `row_minima_online_heavy`: コスト関数に行依存の重い計算を加えた版。
  行ごとの定数項を足しているため、argmin と Monge 性は保たれる。

## References

- https://en.wikipedia.org/wiki/Monge_array
- https://en.wikipedia.org/wiki/SMAWK_algorithm
- http://web.cs.unlv.edu/larmore/Courses/CSC477/monge.pdf
- https://noshi91.hatenablog.com/entry/2023/02/18/005856
