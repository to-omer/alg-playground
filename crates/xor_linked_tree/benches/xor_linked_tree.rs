use std::hint::black_box;

use bench::{
    apply_large_runtime_config, apply_medium_runtime_config, apply_small_runtime_config,
    default_rng,
};
use criterion::measurement::Measurement;
use criterion::{BenchmarkGroup, BenchmarkId, Criterion, criterion_group, criterion_main};
use rand::Rng;

use xor_linked_tree::{diameter_chinese, diameter_csr, diameter_vec, diameter_xor};

const SIZES: [usize; 4] = [1_024, 4_096, 16_384, 65_536];

fn apply_runtime_config_for_size<M: Measurement>(size: usize, group: &mut BenchmarkGroup<'_, M>) {
    if size <= 1_024 {
        apply_small_runtime_config(group);
    } else if size <= 16_384 {
        apply_medium_runtime_config(group);
    } else {
        apply_large_runtime_config(group);
    }
}

fn generate_tree_edges(rng: &mut impl Rng, n: usize) -> Vec<(usize, usize, u64)> {
    if n <= 1 {
        return Vec::new();
    }
    let mut edges = Vec::with_capacity(n - 1);
    for i in 1..n {
        let parent = rng.random_range(0..i);
        let weight = rng.random_range(1..=1_000_000_000_u64);
        edges.push((i, parent, weight));
    }
    edges
}

fn bench_xor_linked_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("xor_linked_tree");
    let mut rng = default_rng();

    for &size in &SIZES {
        apply_runtime_config_for_size(size, &mut group);
        let edges = generate_tree_edges(&mut rng, size);

        group.bench_function(BenchmarkId::new("vec", size), |bencher| {
            bencher.iter(|| black_box(diameter_vec(size, &edges)))
        });

        group.bench_function(BenchmarkId::new("chinese", size), |bencher| {
            bencher.iter(|| black_box(diameter_chinese(size, &edges)))
        });

        group.bench_function(BenchmarkId::new("csr", size), |bencher| {
            bencher.iter(|| black_box(diameter_csr(size, &edges)))
        });

        group.bench_function(BenchmarkId::new("xor", size), |bencher| {
            bencher.iter(|| black_box(diameter_xor(size, &edges)))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_xor_linked_tree);
criterion_main!(benches);
