use criterion::{Criterion, criterion_group, criterion_main};

mod common;

fn bench(c: &mut Criterion) {
    let mut read = c.benchmark_group("ordered_map/read");
    common::bench_all_read(&mut read);
    read.finish();

    let mut mixed = c.benchmark_group("ordered_map/mixed");
    common::bench_all_mixed(&mut mixed);
    mixed.finish();

    let mut update = c.benchmark_group("ordered_map/update");
    common::bench_all_update(&mut update);
    update.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
