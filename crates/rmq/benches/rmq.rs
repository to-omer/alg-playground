use bench::apply_large_runtime_config;
use bench::apply_medium_runtime_config;
use bench::apply_small_runtime_config;
use bench::default_rng;
use criterion::BenchmarkGroup;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use criterion::measurement::Measurement;
use rand::Rng;
use rmq::AlstrupRmq;
use rmq::DisjointSparseTableRmq;
use rmq::SegmentTreeRmq;
use rmq::SparseTableRmq;
use rmq::StaticRmq;
use std::hint::black_box;

const SIZES: [usize; 4] = [1_024, 4_096, 16_384, 65_536];
const VALUE_RANGE: std::ops::RangeInclusive<i64> = -1_000_000_000..=1_000_000_000;

#[derive(Clone, Copy, Debug)]
enum Workload {
    NDiv4,
    N,
    NTimes4,
    NTimes16,
}

impl Workload {
    fn label(self) -> &'static str {
        match self {
            Self::NDiv4 => "n_div_4",
            Self::N => "n",
            Self::NTimes4 => "4n",
            Self::NTimes16 => "16n",
        }
    }

    fn query_count(self, n: usize) -> usize {
        match self {
            Self::NDiv4 => (n / 4).max(1),
            Self::N => n.max(1),
            Self::NTimes4 => (4 * n).max(1),
            Self::NTimes16 => (16 * n).max(1),
        }
    }
}

fn apply_runtime_config_for_size<M: Measurement>(group: &mut BenchmarkGroup<'_, M>, size: usize) {
    if size <= 4_096 {
        apply_small_runtime_config(group);
    } else if size <= 16_384 {
        apply_medium_runtime_config(group);
    } else {
        apply_large_runtime_config(group);
    }
}

fn generate_values<R: Rng + ?Sized>(rng: &mut R, n: usize) -> Vec<i64> {
    let mut values = Vec::with_capacity(n);
    for _ in 0..n {
        values.push(rng.random_range(VALUE_RANGE));
    }
    values
}

fn generate_queries<R: Rng + ?Sized>(rng: &mut R, n: usize, q: usize) -> Vec<(usize, usize)> {
    let mut queries = Vec::with_capacity(q);
    for _ in 0..q {
        let l = rng.random_range(0..n);
        let r = rng.random_range((l + 1)..=n);
        queries.push((l, r));
    }
    queries
}

fn bench_impl<M, R>(
    group: &mut BenchmarkGroup<'_, M>,
    name: &str,
    size: usize,
    values: &[i64],
    queries: &[(usize, usize)],
) where
    M: Measurement,
    R: StaticRmq,
{
    group.bench_function(BenchmarkId::new(name, size), |bencher| {
        bencher.iter(|| {
            let rmq = R::new(black_box(values));
            let mut acc = 0_usize;
            for &(l, r) in queries {
                let idx = rmq.argmin(black_box(l)..black_box(r)).unwrap();
                acc ^= idx;
            }
            black_box(acc);
        })
    });
}

fn bench_rmq(c: &mut Criterion) {
    let workloads = [
        Workload::NDiv4,
        Workload::N,
        Workload::NTimes4,
        Workload::NTimes16,
    ];
    let mut rng = default_rng();

    for workload in workloads {
        let mut group = c.benchmark_group(format!("rmq/workload/{}", workload.label()));

        for &size in &SIZES {
            apply_runtime_config_for_size(&mut group, size);
            let values = generate_values(&mut rng, size);
            let q = workload.query_count(size);
            let queries = generate_queries(&mut rng, size, q);

            bench_impl::<_, SegmentTreeRmq>(&mut group, "segtree", size, &values, &queries);
            bench_impl::<_, SparseTableRmq>(&mut group, "sparse", size, &values, &queries);
            bench_impl::<_, DisjointSparseTableRmq>(&mut group, "dst", size, &values, &queries);
            bench_impl::<_, AlstrupRmq>(&mut group, "alstrup", size, &values, &queries);
        }

        group.finish();
    }
}

criterion_group!(benches, bench_rmq);
criterion_main!(benches);
