use std::hint::black_box;
use std::time::Duration;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::SamplingMode;
use criterion::criterion_group;
use criterion::criterion_main;
use sssp::DirectedGraph;
use sssp::bmssp_paper;
use sssp::dijkstra_binary_heap;
use sssp::dijkstra_radix_heap;
use sssp::generator::GraphCase;
use sssp::generator::generate_case;

type Solver = fn(&DirectedGraph, usize) -> Vec<u64>;

const ALGORITHMS: [(&str, Solver); 3] = [
    ("bmssp_paper", bmssp_paper),
    ("dijkstra_binary", dijkstra_binary_heap),
    ("dijkstra_radix", dijkstra_radix_heap),
];

const CASES: [GraphCase; 10] = [
    GraphCase::SparseRandom,
    GraphCase::MaxSparseRandom,
    GraphCase::MaxDenseRandom,
    GraphCase::MaxDenseLong,
    GraphCase::MaxDenseZero,
    GraphCase::AlmostLine,
    GraphCase::GridRandom,
    GraphCase::GridSwirl,
    GraphCase::WrongDijkstraKiller,
    GraphCase::SpfaKiller,
];

const SIZES: [usize; 3] = [2_048, 8_192, 32_768];

fn apply_runtime(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    size: usize,
) {
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(120));
    if size <= 8_192 {
        group.sampling_mode(SamplingMode::Auto);
        group.measurement_time(Duration::from_millis(220));
    } else {
        group.sampling_mode(SamplingMode::Flat);
        group.measurement_time(Duration::from_millis(360));
    }
}

fn bench_sssp(c: &mut Criterion) {
    for case in CASES {
        let mut group = c.benchmark_group(format!("sssp/{}", case.label()));

        for &size in &SIZES {
            apply_runtime(&mut group, size);
            let seed = 0x5EED_2026 ^ ((size as u64) << 7) ^ (case as u64);
            let input = generate_case(case, size, seed);

            for (algo_name, solver) in ALGORITHMS {
                group.bench_function(BenchmarkId::new(algo_name, size), |bencher| {
                    bencher.iter(|| {
                        let dist = solver(&input.graph, input.source);
                        black_box(dist);
                    });
                });
            }
        }

        group.finish();
    }
}

criterion_group!(benches, bench_sssp);
criterion_main!(benches);
