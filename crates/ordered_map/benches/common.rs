use std::hint::black_box;
use std::time::{Duration, Instant};

use bench::apply_small_runtime_config;
use criterion::measurement::Measurement;
use criterion::{BenchmarkGroup, BenchmarkId};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use ordered_map::{
    AaTreeMap, AvlTreeMap, BTreeMapCustom, FusionTreeMap, LlrbTreeMap, OrderedMap, RbTreeMap,
    ScapegoatTreeMap, SkipListMap, SplayTreeMap, StdBTreeMap, TreapMap, VebMap, WbtTreeMap,
    XFastTrieMap, YFastTrieMap, ZipTreeMap,
};

const SIZES: [usize; 5] = [1_000, 4_000, 16_000, 64_000, 256_000];
const OPS_PER_ITER: usize = 200;
const GET_HIT_RATE_PERCENT: u64 = 80;
const MIXED_INSERTS_PER_ITER: usize = OPS_PER_ITER / 10; // 10% inserts, 10% removes, 80% reads.

#[derive(Clone)]
enum ReadOp {
    Get { key: u64 },
    LowerBound { key: u64 },
}

#[derive(Clone)]
enum UpdateOp {
    Insert { key: u64, value: u64 },
    Remove { key: u64 },
}

#[derive(Clone)]
enum MixedOp {
    Get { key: u64 },
    LowerBound { key: u64 },
    Insert { key: u64, value: u64 },
    Remove { key: u64 },
}

pub fn bench_read<M, T>(group: &mut BenchmarkGroup<'_, T>, label: &str)
where
    T: Measurement<Value = Duration>,
    M: OrderedMap<Key = u64, Value = u64>,
{
    for &size in &SIZES {
        apply_small_runtime_config(group);
        let base_seed = seed_base(1, size as u64);
        let keys = generate_initial_keys(size, base_seed);
        let mut init_rng = StdRng::seed_from_u64(base_seed ^ 0x11_22_33_44);
        let mut map = M::new();
        for &k in &keys {
            let v: u64 = init_rng.random();
            black_box(map.insert(k, v));
        }

        group.bench_function(BenchmarkId::new(label, size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for iter in 0..iters {
                    let iter_seed = seed_for_iter(base_seed, iter);
                    let mut rng = StdRng::seed_from_u64(iter_seed);
                    let ops = generate_read_ops(&keys, &mut rng);
                    let start = Instant::now();
                    run_read_ops::<M>(&mut map, &ops);
                    black_box(map.len());
                    total += start.elapsed();
                }
                total
            })
        });
    }
}

pub fn bench_update<M, T>(group: &mut BenchmarkGroup<'_, T>, label: &str)
where
    T: Measurement<Value = Duration>,
    M: OrderedMap<Key = u64, Value = u64>,
{
    for &size in &SIZES {
        apply_small_runtime_config(group);
        let base_seed = seed_base(2, size as u64);
        let keys = generate_initial_keys(size, base_seed);
        let mut init_rng = StdRng::seed_from_u64(base_seed ^ 0x55_66_77_88);
        let mut map = M::new();
        for &k in &keys {
            let v: u64 = init_rng.random();
            black_box(map.insert(k, v));
        }

        group.bench_function(BenchmarkId::new(label, size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for iter in 0..iters {
                    let iter_seed = seed_for_iter(base_seed, iter);
                    let mut rng = StdRng::seed_from_u64(iter_seed);
                    let ops = generate_update_ops(size, base_seed, iter, &mut rng);
                    let start = Instant::now();
                    run_update_ops::<M>(&mut map, &ops);
                    black_box(map.len());
                    total += start.elapsed();
                }
                total
            })
        });
    }
}

pub fn bench_mixed<M, T>(group: &mut BenchmarkGroup<'_, T>, label: &str)
where
    T: Measurement<Value = Duration>,
    M: OrderedMap<Key = u64, Value = u64>,
{
    for &size in &SIZES {
        apply_small_runtime_config(group);
        let base_seed = seed_base(3, size as u64);
        let keys = generate_initial_keys(size, base_seed);
        let mut init_rng = StdRng::seed_from_u64(base_seed ^ 0x99_AA_BB_CC);
        let mut map = M::new();
        for &k in &keys {
            let v: u64 = init_rng.random();
            black_box(map.insert(k, v));
        }

        group.bench_function(BenchmarkId::new(label, size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for iter in 0..iters {
                    let iter_seed = seed_for_iter(base_seed, iter);
                    let mut rng = StdRng::seed_from_u64(iter_seed);
                    let ops = generate_mixed_ops(&keys, size, base_seed, iter, &mut rng);
                    let start = Instant::now();
                    run_mixed_ops::<M>(&mut map, &ops);
                    black_box(map.len());
                    total += start.elapsed();
                }
                total
            })
        });
    }
}

fn generate_initial_keys(size: usize, base_seed: u64) -> Vec<u64> {
    (0..size)
        .map(|i| mix_seed(base_seed ^ (i as u64)))
        .collect()
}

fn generate_read_ops(keys: &[u64], rng: &mut StdRng) -> Vec<ReadOp> {
    let mut ops = Vec::with_capacity(OPS_PER_ITER);
    for _ in 0..OPS_PER_ITER {
        let is_get = rng.random::<u64>() & 1 == 0;
        if is_get {
            let hit = rng.random_range(0..100) < GET_HIT_RATE_PERCENT;
            let key = if hit {
                let idx = rng.random_range(0..keys.len());
                keys[idx]
            } else {
                rng.random()
            };
            ops.push(ReadOp::Get { key });
        } else {
            let key: u64 = rng.random();
            ops.push(ReadOp::LowerBound { key });
        }
    }
    ops
}

fn generate_update_ops(size: usize, base_seed: u64, iter: u64, rng: &mut StdRng) -> Vec<UpdateOp> {
    let inserts = OPS_PER_ITER / 2;
    let mut inserted = Vec::with_capacity(inserts);
    let mut ops = Vec::with_capacity(OPS_PER_ITER);
    for i in 0..OPS_PER_ITER {
        if i % 2 == 0 {
            let id = (size as u64)
                .wrapping_add(iter.wrapping_mul(inserts as u64))
                .wrapping_add((i / 2) as u64);
            let key = mix_seed(base_seed ^ id);
            let value: u64 = rng.random();
            inserted.push(key);
            ops.push(UpdateOp::Insert { key, value });
        } else {
            let idx = rng.random_range(0..inserted.len());
            let key = inserted.swap_remove(idx);
            ops.push(UpdateOp::Remove { key });
        }
    }
    debug_assert!(inserted.is_empty());
    ops
}

fn generate_mixed_ops(
    keys: &[u64],
    size: usize,
    base_seed: u64,
    iter: u64,
    rng: &mut StdRng,
) -> Vec<MixedOp> {
    let mut remaining_inserts = MIXED_INSERTS_PER_ITER;
    let mut remaining_removes = MIXED_INSERTS_PER_ITER;
    let mut remaining_reads = OPS_PER_ITER - 2 * MIXED_INSERTS_PER_ITER;

    let mut live_inserted: Vec<u64> = Vec::with_capacity(MIXED_INSERTS_PER_ITER);
    let mut ops = Vec::with_capacity(OPS_PER_ITER);

    while ops.len() < OPS_PER_ITER {
        let remaining_slots = OPS_PER_ITER - ops.len();
        debug_assert_eq!(
            remaining_slots,
            remaining_reads + remaining_inserts + remaining_removes
        );

        let updates_remaining = remaining_inserts + remaining_removes;
        let do_read = if remaining_reads == 0 {
            false
        } else if updates_remaining == 0 {
            true
        } else {
            rng.random_range(0..remaining_slots) < remaining_reads
        };

        if do_read {
            let is_get = rng.random::<u64>() & 1 == 0;
            if is_get {
                let hit = rng.random_range(0..100) < GET_HIT_RATE_PERCENT;
                let key = if hit {
                    let idx = rng.random_range(0..keys.len());
                    keys[idx]
                } else {
                    rng.random()
                };
                ops.push(MixedOp::Get { key });
            } else {
                let key: u64 = rng.random();
                ops.push(MixedOp::LowerBound { key });
            }
            remaining_reads -= 1;
            continue;
        }

        let can_insert = remaining_inserts > 0;
        let can_remove = remaining_removes > 0 && !live_inserted.is_empty();

        let do_remove = if !can_remove {
            false
        } else if !can_insert {
            true
        } else {
            rng.random_range(0..(remaining_inserts + remaining_removes)) < remaining_removes
        };

        if do_remove {
            let idx = rng.random_range(0..live_inserted.len());
            let key = live_inserted.swap_remove(idx);
            ops.push(MixedOp::Remove { key });
            remaining_removes -= 1;
        } else {
            let insert_index = MIXED_INSERTS_PER_ITER - remaining_inserts;
            let id = (size as u64)
                .wrapping_add(iter.wrapping_mul(MIXED_INSERTS_PER_ITER as u64))
                .wrapping_add(insert_index as u64);
            let key = mix_seed(base_seed ^ id);
            let value: u64 = rng.random();
            live_inserted.push(key);
            ops.push(MixedOp::Insert { key, value });
            remaining_inserts -= 1;
        }
    }

    debug_assert_eq!(remaining_reads, 0);
    debug_assert_eq!(remaining_inserts, 0);
    debug_assert_eq!(remaining_removes, 0);
    debug_assert!(live_inserted.is_empty());
    ops
}

fn run_read_ops<M>(map: &mut M, ops: &[ReadOp])
where
    M: OrderedMap<Key = u64, Value = u64>,
{
    for op in ops {
        match *op {
            ReadOp::Get { key } => {
                black_box(map.get(&key).copied());
            }
            ReadOp::LowerBound { key } => {
                black_box(map.lower_bound(&key).map(|(k, v)| (*k, *v)));
            }
        }
    }
}

fn run_update_ops<M>(map: &mut M, ops: &[UpdateOp])
where
    M: OrderedMap<Key = u64, Value = u64>,
{
    for op in ops {
        match *op {
            UpdateOp::Insert { key, value } => {
                black_box(map.insert(key, value));
            }
            UpdateOp::Remove { key } => {
                black_box(map.remove(&key));
            }
        }
    }
}

fn run_mixed_ops<M>(map: &mut M, ops: &[MixedOp])
where
    M: OrderedMap<Key = u64, Value = u64>,
{
    for op in ops {
        match *op {
            MixedOp::Get { key } => {
                black_box(map.get(&key).copied());
            }
            MixedOp::LowerBound { key } => {
                black_box(map.lower_bound(&key).map(|(k, v)| (*k, *v)));
            }
            MixedOp::Insert { key, value } => {
                black_box(map.insert(key, value));
            }
            MixedOp::Remove { key } => {
                black_box(map.remove(&key));
            }
        }
    }
}

fn seed_base(workload_id: u64, size: u64) -> u64 {
    mix_seed(0x0DDB_A11A_2026_0000_u64 ^ (workload_id << 48) ^ size)
}

fn seed_for_iter(base: u64, iter: u64) -> u64 {
    mix_seed(base ^ iter.wrapping_mul(SEED_MIX))
}

const SEED_MIX: u64 = 0x9E37_79B9_7F4A_7C15;

fn mix_seed(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

pub fn bench_all_read<T>(group: &mut BenchmarkGroup<'_, T>)
where
    T: Measurement<Value = Duration>,
{
    bench_read::<StdBTreeMap<u64, u64>, _>(group, "std_btree");
    bench_read::<AvlTreeMap<u64, u64>, _>(group, "avl");
    bench_read::<WbtTreeMap<u64, u64>, _>(group, "wbt");
    bench_read::<AaTreeMap<u64, u64>, _>(group, "aa");
    bench_read::<LlrbTreeMap<u64, u64>, _>(group, "llrb");
    bench_read::<RbTreeMap<u64, u64>, _>(group, "rb");
    bench_read::<TreapMap<u64, u64>, _>(group, "treap");
    bench_read::<ZipTreeMap<u64, u64>, _>(group, "zip");
    bench_read::<SplayTreeMap<u64, u64>, _>(group, "splay");
    bench_read::<ScapegoatTreeMap<u64, u64>, _>(group, "scapegoat");
    bench_read::<SkipListMap<u64, u64>, _>(group, "skiplist");
    bench_read::<BTreeMapCustom<u64, u64>, _>(group, "btree");
    bench_read::<VebMap<u64>, _>(group, "veb");
    bench_read::<XFastTrieMap<u64>, _>(group, "xfast");
    bench_read::<YFastTrieMap<u64>, _>(group, "yfast");
    bench_read::<FusionTreeMap<u64>, _>(group, "fusion");
}

pub fn bench_all_mixed<T>(group: &mut BenchmarkGroup<'_, T>)
where
    T: Measurement<Value = Duration>,
{
    bench_mixed::<StdBTreeMap<u64, u64>, _>(group, "std_btree");
    bench_mixed::<AvlTreeMap<u64, u64>, _>(group, "avl");
    bench_mixed::<WbtTreeMap<u64, u64>, _>(group, "wbt");
    bench_mixed::<AaTreeMap<u64, u64>, _>(group, "aa");
    bench_mixed::<LlrbTreeMap<u64, u64>, _>(group, "llrb");
    bench_mixed::<RbTreeMap<u64, u64>, _>(group, "rb");
    bench_mixed::<TreapMap<u64, u64>, _>(group, "treap");
    bench_mixed::<ZipTreeMap<u64, u64>, _>(group, "zip");
    bench_mixed::<SplayTreeMap<u64, u64>, _>(group, "splay");
    bench_mixed::<ScapegoatTreeMap<u64, u64>, _>(group, "scapegoat");
    bench_mixed::<SkipListMap<u64, u64>, _>(group, "skiplist");
    bench_mixed::<BTreeMapCustom<u64, u64>, _>(group, "btree");
    bench_mixed::<VebMap<u64>, _>(group, "veb");
    bench_mixed::<XFastTrieMap<u64>, _>(group, "xfast");
    bench_mixed::<YFastTrieMap<u64>, _>(group, "yfast");
    bench_mixed::<FusionTreeMap<u64>, _>(group, "fusion");
}

pub fn bench_all_update<T>(group: &mut BenchmarkGroup<'_, T>)
where
    T: Measurement<Value = Duration>,
{
    bench_update::<StdBTreeMap<u64, u64>, _>(group, "std_btree");
    bench_update::<AvlTreeMap<u64, u64>, _>(group, "avl");
    bench_update::<WbtTreeMap<u64, u64>, _>(group, "wbt");
    bench_update::<AaTreeMap<u64, u64>, _>(group, "aa");
    bench_update::<LlrbTreeMap<u64, u64>, _>(group, "llrb");
    bench_update::<RbTreeMap<u64, u64>, _>(group, "rb");
    bench_update::<TreapMap<u64, u64>, _>(group, "treap");
    bench_update::<ZipTreeMap<u64, u64>, _>(group, "zip");
    bench_update::<SplayTreeMap<u64, u64>, _>(group, "splay");
    bench_update::<ScapegoatTreeMap<u64, u64>, _>(group, "scapegoat");
    bench_update::<SkipListMap<u64, u64>, _>(group, "skiplist");
    bench_update::<BTreeMapCustom<u64, u64>, _>(group, "btree");
    bench_update::<VebMap<u64>, _>(group, "veb");
    bench_update::<XFastTrieMap<u64>, _>(group, "xfast");
    bench_update::<YFastTrieMap<u64>, _>(group, "yfast");
    bench_update::<FusionTreeMap<u64>, _>(group, "fusion");
}
