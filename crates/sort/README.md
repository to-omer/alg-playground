# sort

`sort` crate collects 20 integer sorting implementations under one API and benchmark suite.

## Public API

- `sort_u64(algo, data)`
- `sort_u64_with_ctx(algo, data, ctx)`
- `all_algorithms()`
- `algorithm_name(algo)`
- `supports_track(algo, track)`

Tracks:

- `DataTrack::FullU64`: full `u64` range benchmark inputs
- `DataTrack::BoundedU20`: bounded range inputs (`0..2^20`)

## Implemented algorithms

1. insertion_sort
2. binary_insertion_sort
3. shell_sort_ciura
4. heap_sort
5. merge_sort_top_down
6. merge_sort_bottom_up
7. natural_merge_sort
8. timsort
9. quick_sort_median3
10. quick_sort_3way
11. dual_pivot_quick_sort
12. introsort
13. pdqsort_like
14. block_quick_sort
15. quick_merge_sort
16. counting_sort
17. pigeonhole_sort
18. bucket_sort
19. radix_sort_lsd_base256
20. american_flag_sort_msd

## Benchmark

実行:

```bash
RUSTFLAGS="-C target-cpu=native" cargo bench -p sort --bench sort
```

注記:

- `SORT_BENCH_PROFILE` は廃止済みです。
- 計算量的にベンチ不向きな `insertion_sort` / `binary_insertion_sort` はベンチ対象から除外しています。
- ベンチ行列は実行時間と安定性を優先し、分布・サイズを
  `random_uniform` / `nearly_sorted_1pct_swaps` × `4096` / `16384` / `65536` / `262144` に固定しています。
