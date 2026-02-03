use criterion::{Criterion, criterion_group, criterion_main};

use bbst::{CorePolicy, RangeSum, RangeSumRangeAdd};

mod common;

fn bench(c: &mut Criterion) {
    bench_core(c);
    bench_agg(c);
    bench_agg_reverse(c);
    bench_agg_lazy(c);
    bench_full(c);
}

fn bench_core(c: &mut Criterion) {
    common::bench_workload::<CorePolicy>(c, common::FeatureSet::Core);
}

fn bench_agg(c: &mut Criterion) {
    common::bench_workload::<RangeSum>(c, common::FeatureSet::Agg);
}

fn bench_agg_reverse(c: &mut Criterion) {
    common::bench_workload::<RangeSum>(c, common::FeatureSet::AggReverse);
}

fn bench_agg_lazy(c: &mut Criterion) {
    common::bench_workload::<RangeSumRangeAdd>(c, common::FeatureSet::AggLazy);
}

fn bench_full(c: &mut Criterion) {
    common::bench_workload::<RangeSumRangeAdd>(c, common::FeatureSet::Full);
}

criterion_group!(benches, bench);
criterion_main!(benches);
