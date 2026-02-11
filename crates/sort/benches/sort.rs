use std::hint::black_box;
use std::time::Duration;

use criterion::measurement::Measurement;
use criterion::{
    BenchmarkGroup, BenchmarkId, Criterion, SamplingMode, criterion_group, criterion_main,
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sort::{
    DataTrack, SortAlgorithm, SortContext, algorithm_name, all_algorithms, sort_u64_with_ctx,
    supports_track,
};

const BENCH_SIZES: [usize; 4] = [4096, 16384, 65536, 262144];
const BOUNDED_MAX: u64 = (1 << 20) - 1;
const BENCH_SAMPLE_SIZE: usize = 10;
const BENCH_WARMUP_MS: u64 = 80;
const BENCH_MEASURE_MS_SMALL: u64 = 120;
const BENCH_MEASURE_MS_LARGE: u64 = 300;
const BENCH_MEASURE_MS_XL: u64 = 500;

#[derive(Clone, Copy)]
enum Distribution {
    RandomUniform,
    NearlySorted1pctSwaps,
}

impl Distribution {
    fn label(self) -> &'static str {
        match self {
            Self::RandomUniform => "random_uniform",
            Self::NearlySorted1pctSwaps => "nearly_sorted_1pct_swaps",
        }
    }
}

#[derive(Clone, Copy)]
struct TrackSpec {
    track: DataTrack,
    label: &'static str,
}

const TRACKS: [TrackSpec; 2] = [
    TrackSpec {
        track: DataTrack::FullU64,
        label: "full_u64",
    },
    TrackSpec {
        track: DataTrack::BoundedU20,
        label: "bounded_u20",
    },
];

const DISTRIBUTIONS: [Distribution; 2] = [
    Distribution::RandomUniform,
    Distribution::NearlySorted1pctSwaps,
];

fn bench_sort(c: &mut Criterion) {
    for &track in &TRACKS {
        for &dist in &DISTRIBUTIONS {
            let mut group = c.benchmark_group(format!("sort/{}/{}", track.label, dist.label()));

            for &algo in all_algorithms() {
                if !is_benchmark_target(algo) {
                    continue;
                }
                if !supports_track(algo, track.track) {
                    continue;
                }
                for &size in &BENCH_SIZES {
                    apply_runtime(&mut group, size);
                    let seed = seed_for(track.track, dist, size, algo as u64);
                    let base = generate_dataset(track.track, dist, size, seed);

                    group.bench_function(BenchmarkId::new(algorithm_name(algo), size), |bencher| {
                        bencher.iter_custom(|iters| {
                            let mut total = Duration::ZERO;
                            let mut ctx = SortContext::default();
                            for _ in 0..iters {
                                let mut data = base.clone();
                                let start = std::time::Instant::now();
                                sort_u64_with_ctx(algo, &mut data, &mut ctx);
                                total += start.elapsed();
                                black_box(&data);
                            }
                            total
                        });
                    });
                }
            }

            for &size in &BENCH_SIZES {
                apply_runtime(&mut group, size);
                let seed = seed_for(track.track, dist, size, 0xBA5E_0001);
                let base = generate_dataset(track.track, dist, size, seed);
                group.bench_function(BenchmarkId::new("std_unstable", size), |bencher| {
                    bencher.iter_custom(|iters| {
                        let mut total = Duration::ZERO;
                        for _ in 0..iters {
                            let mut data = base.clone();
                            let start = std::time::Instant::now();
                            data.sort_unstable();
                            total += start.elapsed();
                            black_box(&data);
                        }
                        total
                    });
                });
            }

            for &size in &BENCH_SIZES {
                apply_runtime(&mut group, size);
                let seed = seed_for(track.track, dist, size, 0xBA5E_0002);
                let base = generate_dataset(track.track, dist, size, seed);
                group.bench_function(BenchmarkId::new("std_stable", size), |bencher| {
                    bencher.iter_custom(|iters| {
                        let mut total = Duration::ZERO;
                        for _ in 0..iters {
                            let mut data = base.clone();
                            let start = std::time::Instant::now();
                            data.sort();
                            total += start.elapsed();
                            black_box(&data);
                        }
                        total
                    });
                });
            }

            group.finish();
        }
    }
}

#[inline]
fn is_benchmark_target(algo: SortAlgorithm) -> bool {
    !matches!(
        algo,
        SortAlgorithm::InsertionSort | SortAlgorithm::BinaryInsertionSort
    )
}

fn apply_runtime<M: Measurement>(group: &mut BenchmarkGroup<'_, M>, size: usize) {
    group.sample_size(BENCH_SAMPLE_SIZE);
    group.warm_up_time(Duration::from_millis(BENCH_WARMUP_MS));
    if size <= 16384 {
        group.sampling_mode(SamplingMode::Auto);
        group.measurement_time(Duration::from_millis(BENCH_MEASURE_MS_SMALL));
    } else if size <= 65536 {
        group.sampling_mode(SamplingMode::Flat);
        group.measurement_time(Duration::from_millis(BENCH_MEASURE_MS_LARGE));
    } else {
        group.sampling_mode(SamplingMode::Flat);
        group.measurement_time(Duration::from_millis(BENCH_MEASURE_MS_XL));
    }
}

fn generate_dataset(track: DataTrack, dist: Distribution, size: usize, seed: u64) -> Vec<u64> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut data = Vec::with_capacity(size);

    match dist {
        Distribution::RandomUniform => {
            for _ in 0..size {
                data.push(sample_key(track, &mut rng));
            }
        }
        Distribution::NearlySorted1pctSwaps => {
            for i in 0..size {
                data.push(match track {
                    DataTrack::FullU64 => i as u64,
                    DataTrack::BoundedU20 => (i as u64) & BOUNDED_MAX,
                });
            }
            let swaps = (size / 100).max(1);
            for _ in 0..swaps {
                let a = rng.random_range(0..size);
                let b = rng.random_range(0..size);
                data.swap(a, b);
            }
        }
    }

    data
}

#[inline]
fn sample_key(track: DataTrack, rng: &mut StdRng) -> u64 {
    match track {
        DataTrack::FullU64 => rng.random::<u64>(),
        DataTrack::BoundedU20 => rng.random_range(0..=BOUNDED_MAX),
    }
}

#[inline]
fn seed_for(track: DataTrack, dist: Distribution, size: usize, salt: u64) -> u64 {
    let t = match track {
        DataTrack::FullU64 => 1_u64,
        DataTrack::BoundedU20 => 2_u64,
    };
    let d = match dist {
        Distribution::RandomUniform => 11_u64,
        Distribution::NearlySorted1pctSwaps => 12_u64,
    };

    mix_seed(0x5EED_2026 ^ (t << 56) ^ (d << 48) ^ (size as u64) ^ salt)
}

#[inline]
fn mix_seed(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

criterion_group!(benches, bench_sort);
criterion_main!(benches);
