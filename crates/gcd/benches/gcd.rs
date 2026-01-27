use std::hint::black_box;

use bench::{apply_small_runtime_config, default_rng, random_with_bits};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use gcd::{gcd_binary, gcd_euclid};

fn bench_gcd(c: &mut Criterion) {
    const DATASET_SIZE: usize = 1024;
    const BIT_LENGTHS: [u32; 8] = [8, 16, 24, 32, 40, 48, 56, 64];

    type GcdFn = fn(u64, u64) -> u64;
    let impls: [(&str, GcdFn); 2] = [("euclid", gcd_euclid), ("binary", gcd_binary)];

    let mut rng = default_rng();

    let mut group = c.benchmark_group("gcd_bitlen");
    apply_small_runtime_config(&mut group);

    for &bits in &BIT_LENGTHS {
        let pairs = (0..DATASET_SIZE)
            .map(|_| {
                (
                    random_with_bits(&mut rng, bits),
                    random_with_bits(&mut rng, bits),
                )
            })
            .collect::<Vec<_>>();

        for &(name, func) in &impls {
            group.bench_function(BenchmarkId::new(name, bits), |bencher| {
                bencher.iter(|| {
                    for &(a, b) in &pairs {
                        black_box(func(black_box(a), black_box(b)));
                    }
                })
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench_gcd);
criterion_main!(benches);
