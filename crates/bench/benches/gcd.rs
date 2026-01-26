use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use gcd::{gcd_binary, gcd_euclid};
use rand::{Rng, SeedableRng, rngs::StdRng};
use std::hint::black_box;
use std::time::Duration;

fn random_with_bits(rng: &mut StdRng, bits: u32) -> u64 {
    if bits == 0 {
        return 0;
    }
    if bits >= 64 {
        let value = rng.random::<u64>() | (1_u64 << 63);
        return if value == 0 { 1_u64 << 63 } else { value };
    }

    let min = 1_u64 << (bits - 1);
    let max = (1_u64 << bits) - 1;
    rng.random_range(min..=max)
}

fn bench_gcd(c: &mut Criterion) {
    const DATASET_SIZE: usize = 1024;
    const BIT_LENGTHS: [u32; 8] = [8, 16, 24, 32, 40, 48, 56, 64];
    const SAMPLE_SIZE: usize = 15;
    const WARM_UP_MS: u64 = 100;
    const MEASURE_MS: u64 = 200;

    type GcdFn = fn(u64, u64) -> u64;
    let impls: [(&str, GcdFn); 2] = [("euclid", gcd_euclid), ("binary", gcd_binary)];

    let mut rng = StdRng::seed_from_u64(0x5EED_2026);

    let mut group = c.benchmark_group("gcd_bitlen");
    group.sample_size(SAMPLE_SIZE);
    group.warm_up_time(Duration::from_millis(WARM_UP_MS));
    group.measurement_time(Duration::from_millis(MEASURE_MS));

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
