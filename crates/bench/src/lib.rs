use std::time::Duration;

use criterion::BenchmarkGroup;
use criterion::measurement::Measurement;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const SMALL_RUNTIME_SAMPLE_SIZE: usize = 15;
const SMALL_RUNTIME_WARM_UP_MS: u64 = 100;
const SMALL_RUNTIME_MEASURE_MS: u64 = 200;
const MEDIUM_RUNTIME_SAMPLE_SIZE: usize = 15;
const MEDIUM_RUNTIME_WARM_UP_MS: u64 = 500;
const MEDIUM_RUNTIME_MEASURE_MS: u64 = 1000;
const LARGE_RUNTIME_SAMPLE_SIZE: usize = 10;
const LARGE_RUNTIME_WARM_UP_MS: u64 = 800;
const LARGE_RUNTIME_MEASURE_MS: u64 = 1500;
const RNG_SEED: u64 = 0x5EED_2026;

pub fn apply_small_runtime_config<M: Measurement>(group: &mut BenchmarkGroup<'_, M>) {
    group.sample_size(SMALL_RUNTIME_SAMPLE_SIZE);
    group.warm_up_time(Duration::from_millis(SMALL_RUNTIME_WARM_UP_MS));
    group.measurement_time(Duration::from_millis(SMALL_RUNTIME_MEASURE_MS));
}

pub fn apply_medium_runtime_config<M: Measurement>(group: &mut BenchmarkGroup<'_, M>) {
    group.sample_size(MEDIUM_RUNTIME_SAMPLE_SIZE);
    group.warm_up_time(Duration::from_millis(MEDIUM_RUNTIME_WARM_UP_MS));
    group.measurement_time(Duration::from_millis(MEDIUM_RUNTIME_MEASURE_MS));
}

pub fn apply_large_runtime_config<M: Measurement>(group: &mut BenchmarkGroup<'_, M>) {
    group.sample_size(LARGE_RUNTIME_SAMPLE_SIZE);
    group.warm_up_time(Duration::from_millis(LARGE_RUNTIME_WARM_UP_MS));
    group.measurement_time(Duration::from_millis(LARGE_RUNTIME_MEASURE_MS));
}

pub fn default_rng() -> StdRng {
    StdRng::seed_from_u64(RNG_SEED)
}

pub fn random_with_bits<R: Rng + ?Sized>(rng: &mut R, bits: u32) -> u64 {
    if bits == 0 {
        return 0;
    }

    let high_bit = (bits - 1).min(63);
    let min = 1_u64 << high_bit;
    let max = if bits >= 64 {
        u64::MAX
    } else {
        (1_u64 << bits) - 1
    };
    rng.random_range(min..=max)
}
