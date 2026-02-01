use std::hint::black_box;
use std::rc::Rc;

use bench::{apply_large_runtime_config, apply_medium_runtime_config, apply_small_runtime_config};
use criterion::measurement::Measurement;
use criterion::{BenchmarkGroup, BenchmarkId, Criterion, criterion_group, criterion_main};
use row_minima::{larsch_shortest_path, monotone_minima, simple_larsch_shortest_path, smawk};

const SIZES: [usize; 7] = [256, 1024, 4096, 16384, 65536, 131072, 262144];

fn heavy_row_penalty(row: usize) -> u64 {
    let mut x = (row as u64) ^ 0x9E37_79B9_7F4A_7C15;
    for _ in 0..6 {
        x = x
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        x ^= x >> 23;
        x = x.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
    x & 0xFFFF
}

fn apply_runtime_config_for_size<M: Measurement>(size: usize, group: &mut BenchmarkGroup<'_, M>) {
    if size <= 1024 {
        apply_small_runtime_config(group);
    } else if size <= 16384 {
        apply_medium_runtime_config(group);
    } else {
        apply_large_runtime_config(group);
    }
}

fn bench_row_minima(c: &mut Criterion) {
    let mut offline = c.benchmark_group("row_minima_offline");
    for &size in &SIZES {
        apply_runtime_config_for_size(size, &mut offline);
        let cost = |i: usize, k: usize| {
            let diff = i.abs_diff(k) as u64;
            diff * diff
        };

        offline.bench_function(BenchmarkId::new("monotone", size), |bencher| {
            bencher.iter(|| {
                let result = monotone_minima(size, size, &cost);
                black_box(result);
            })
        });

        offline.bench_function(BenchmarkId::new("smawk", size), |bencher| {
            bencher.iter(|| {
                let result = smawk(size, size, &cost);
                black_box(result);
            })
        });
    }
    offline.finish();

    let mut offline_heavy = c.benchmark_group("row_minima_offline_heavy");
    for &size in &SIZES {
        apply_runtime_config_for_size(size, &mut offline_heavy);
        let cost = |i: usize, k: usize| {
            let diff = i.abs_diff(k) as u64;
            diff * diff + heavy_row_penalty(i)
        };

        offline_heavy.bench_function(BenchmarkId::new("monotone", size), |bencher| {
            bencher.iter(|| {
                let result = monotone_minima(size, size, &cost);
                black_box(result);
            })
        });

        offline_heavy.bench_function(BenchmarkId::new("smawk", size), |bencher| {
            bencher.iter(|| {
                let result = smawk(size, size, &cost);
                black_box(result);
            })
        });
    }
    offline_heavy.finish();

    let mut online = c.benchmark_group("row_minima_online");
    for &size in &SIZES {
        apply_runtime_config_for_size(size, &mut online);
        let cost = |i: usize, k: usize| {
            if k >= i {
                return u64::MAX / 4;
            }
            let diff = i.abs_diff(k) as u64;
            diff * diff
        };

        online.bench_function(BenchmarkId::new("monotone", size), |bencher| {
            bencher.iter(|| {
                let result = monotone_minima(size, size, &cost);
                black_box(result);
            })
        });

        online.bench_function(BenchmarkId::new("smawk", size), |bencher| {
            bencher.iter(|| {
                let result = smawk(size, size, &cost);
                black_box(result);
            })
        });

        online.bench_function(BenchmarkId::new("simple_larsch", size), |bencher| {
            bencher.iter(|| {
                let result = simple_larsch_shortest_path(size, &cost);
                black_box(result);
            })
        });

        online.bench_function(BenchmarkId::new("larsch", size), |bencher| {
            let cost = Rc::new(cost);
            bencher.iter(|| {
                let cost = Rc::clone(&cost);
                let result = larsch_shortest_path(size, move |i, k| cost(i, k));
                black_box(result);
            })
        });
    }
    online.finish();

    let mut online_heavy = c.benchmark_group("row_minima_online_heavy");
    for &size in &SIZES {
        apply_runtime_config_for_size(size, &mut online_heavy);
        let cost = |i: usize, k: usize| {
            if k >= i {
                return u64::MAX / 4;
            }
            let diff = i.abs_diff(k) as u64;
            diff * diff + heavy_row_penalty(i)
        };

        online_heavy.bench_function(BenchmarkId::new("monotone", size), |bencher| {
            bencher.iter(|| {
                let result = monotone_minima(size, size, &cost);
                black_box(result);
            })
        });

        online_heavy.bench_function(BenchmarkId::new("smawk", size), |bencher| {
            bencher.iter(|| {
                let result = smawk(size, size, &cost);
                black_box(result);
            })
        });

        online_heavy.bench_function(BenchmarkId::new("simple_larsch", size), |bencher| {
            bencher.iter(|| {
                let result = simple_larsch_shortest_path(size, &cost);
                black_box(result);
            })
        });

        online_heavy.bench_function(BenchmarkId::new("larsch", size), |bencher| {
            let cost = Rc::new(cost);
            bencher.iter(|| {
                let cost = Rc::clone(&cost);
                let result = larsch_shortest_path(size, move |i, k| cost(i, k));
                black_box(result);
            })
        });
    }
    online_heavy.finish();
}

criterion_group!(benches, bench_row_minima);
criterion_main!(benches);
