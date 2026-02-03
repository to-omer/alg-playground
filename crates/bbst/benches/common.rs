use std::hint::black_box;
use std::time::{Duration, Instant};

use bench::apply_small_runtime_config;
use criterion::measurement::Measurement;
use criterion::{BenchmarkGroup, BenchmarkId, Criterion};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use bbst::{
    ImplicitAaTree, ImplicitAvl, ImplicitRbTree, ImplicitSplay, ImplicitTreap, ImplicitWbt,
    ImplicitZipTree, LazyMapMonoid, SequenceAgg, SequenceBase, SequenceLazy, SequenceReverse,
};

const SIZES: [usize; 5] = [1_000, 4_000, 16_000, 64_000, 256_000];
const OPS_PER_SIZE: usize = 100;
const VALUE_RANGE: std::ops::RangeInclusive<i64> = -1_000_000_000..=1_000_000_000;
const DELTA_RANGE: std::ops::RangeInclusive<i64> = -1_000..=1_000;
const WORKLOAD_WEIGHTS: &[(OpKind, u32)] = &[
    (OpKind::Get, 20),
    (OpKind::Fold, 20),
    (OpKind::Update, 20),
    (OpKind::Insert, 20),
    (OpKind::Remove, 20),
    (OpKind::Reverse, 10),
];

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum FeatureSet {
    Core,
    Agg,
    AggReverse,
    AggLazy,
    Full,
}

#[derive(Clone, Copy)]
enum OpKind {
    Get,
    Fold,
    Update,
    Insert,
    Remove,
    Reverse,
}

#[derive(Clone)]
enum Op<K, A> {
    Get { index: usize },
    Fold { start: usize, end: usize },
    Update { start: usize, end: usize, act: A },
    Insert { index: usize, value: K },
    Remove { index: usize },
    Reverse { start: usize, end: usize },
}

pub trait ActFromDelta: Clone {
    fn from_delta(delta: i64) -> Self;
}

trait BenchTree<P: LazyMapMonoid<Key = i64>>:
    SequenceBase<Key = i64> + SequenceAgg<Agg = P::Agg> + SequenceLazy<Act = P::Act> + SequenceReverse
{
    fn with_seed(seed: u64) -> Self;
}

impl<P: LazyMapMonoid<Key = i64>> BenchTree<P> for ImplicitTreap<P> {
    fn with_seed(seed: u64) -> Self {
        Self::with_seed(seed)
    }
}

impl<P: LazyMapMonoid<Key = i64>> BenchTree<P> for ImplicitSplay<P> {
    fn with_seed(seed: u64) -> Self {
        Self::with_seed(seed)
    }
}

impl<P: LazyMapMonoid<Key = i64>> BenchTree<P> for ImplicitWbt<P> {
    fn with_seed(seed: u64) -> Self {
        Self::with_seed(seed)
    }
}

impl<P: LazyMapMonoid<Key = i64>> BenchTree<P> for ImplicitZipTree<P> {
    fn with_seed(seed: u64) -> Self {
        Self::with_seed(seed)
    }
}

impl<P: LazyMapMonoid<Key = i64>> BenchTree<P> for ImplicitAaTree<P> {
    fn with_seed(seed: u64) -> Self {
        Self::with_seed(seed)
    }
}

impl<P: LazyMapMonoid<Key = i64>> BenchTree<P> for ImplicitAvl<P> {
    fn with_seed(seed: u64) -> Self {
        Self::with_seed(seed)
    }
}

impl<P: LazyMapMonoid<Key = i64>> BenchTree<P> for ImplicitRbTree<P> {
    fn with_seed(seed: u64) -> Self {
        Self::with_seed(seed)
    }
}

impl ActFromDelta for i64 {
    fn from_delta(delta: i64) -> Self {
        delta
    }
}

impl ActFromDelta for () {
    fn from_delta(_delta: i64) -> Self {}
}

pub fn bench_workload<P>(c: &mut Criterion, feature: FeatureSet)
where
    P: LazyMapMonoid<Key = i64>,
    P::Act: ActFromDelta + Clone,
    P::Agg: Clone,
{
    let group_name = format!("bbst/{}", feature_label(feature));
    let mut group = c.benchmark_group(group_name);

    for &size in &SIZES {
        apply_runtime_config_for_size(size, &mut group);
        let base_seed = seed_base(feature, size as u64);
        let mut init_rng = StdRng::seed_from_u64(base_seed);
        let initial = generate_initial(size, &mut init_rng);

        bench_tree::<ImplicitTreap<P>, P, _>(
            &mut group, "treap", size, feature, base_seed, &initial,
        );
        bench_tree::<ImplicitSplay<P>, P, _>(
            &mut group, "splay", size, feature, base_seed, &initial,
        );
        bench_tree::<ImplicitWbt<P>, P, _>(&mut group, "wbt", size, feature, base_seed, &initial);
        bench_tree::<ImplicitZipTree<P>, P, _>(
            &mut group, "zip", size, feature, base_seed, &initial,
        );
        bench_tree::<ImplicitAaTree<P>, P, _>(&mut group, "aa", size, feature, base_seed, &initial);
        bench_tree::<ImplicitAvl<P>, P, _>(&mut group, "avl", size, feature, base_seed, &initial);
        bench_tree::<ImplicitRbTree<P>, P, _>(&mut group, "rb", size, feature, base_seed, &initial);
    }

    group.finish();
}

fn apply_runtime_config_for_size<M: Measurement>(_size: usize, group: &mut BenchmarkGroup<'_, M>) {
    apply_small_runtime_config(group);
}

fn feature_supports(feature: FeatureSet, kind: OpKind) -> bool {
    match feature {
        FeatureSet::Core => matches!(kind, OpKind::Get | OpKind::Insert | OpKind::Remove),
        FeatureSet::Agg => matches!(
            kind,
            OpKind::Get | OpKind::Insert | OpKind::Remove | OpKind::Fold
        ),
        FeatureSet::AggReverse => matches!(
            kind,
            OpKind::Get | OpKind::Insert | OpKind::Remove | OpKind::Fold | OpKind::Reverse
        ),
        FeatureSet::AggLazy => matches!(
            kind,
            OpKind::Get | OpKind::Insert | OpKind::Remove | OpKind::Fold | OpKind::Update
        ),
        FeatureSet::Full => true,
    }
}

fn choose_kind<R: Rng + ?Sized>(rng: &mut R, feature: FeatureSet) -> OpKind {
    let mut total = 0_u32;
    for (kind, weight) in WORKLOAD_WEIGHTS {
        if feature_supports(feature, *kind) {
            total += *weight;
        }
    }

    if total == 0 {
        return OpKind::Insert;
    }

    let mut roll = rng.random_range(0..total);
    for (kind, weight) in WORKLOAD_WEIGHTS {
        if !feature_supports(feature, *kind) {
            continue;
        }
        if roll < *weight {
            return *kind;
        }
        roll -= *weight;
    }

    OpKind::Insert
}

fn random_range<R: Rng + ?Sized>(rng: &mut R, len: usize) -> (usize, usize) {
    let start = rng.random_range(0..len);
    let end = rng.random_range((start + 1)..=len);
    (start, end)
}

fn generate_initial<R: Rng + ?Sized>(size: usize, rng: &mut R) -> Vec<i64> {
    let mut initial = Vec::with_capacity(size);
    for _ in 0..size {
        initial.push(rng.random_range(VALUE_RANGE));
    }
    initial
}

fn generate_ops<A>(feature: FeatureSet, size: usize, rng: &mut impl Rng) -> Vec<Op<i64, A>>
where
    A: ActFromDelta + Clone,
{
    let mut len = size;
    let allow_resize =
        feature_supports(feature, OpKind::Insert) && feature_supports(feature, OpKind::Remove);
    let ops_count = OPS_PER_SIZE;
    let mut ops = Vec::with_capacity(ops_count);

    for step in 0..ops_count {
        let remaining = ops_count - step;
        let mut kind = choose_kind(rng, feature);
        if allow_resize {
            let delta = len as isize - size as isize;
            if delta > remaining as isize {
                kind = OpKind::Remove;
            } else if delta < -(remaining as isize) {
                kind = OpKind::Insert;
            }
        }
        if len == 0 && !matches!(kind, OpKind::Insert) {
            kind = OpKind::Insert;
        }

        match kind {
            OpKind::Get => {
                let index = rng.random_range(0..len);
                ops.push(Op::Get { index });
            }
            OpKind::Fold => {
                let (start, end) = random_range(rng, len);
                ops.push(Op::Fold { start, end });
            }
            OpKind::Update => {
                let (start, end) = random_range(rng, len);
                let delta = rng.random_range(DELTA_RANGE);
                ops.push(Op::Update {
                    start,
                    end,
                    act: A::from_delta(delta),
                });
            }
            OpKind::Insert => {
                let index = rng.random_range(0..=len);
                let value = rng.random_range(VALUE_RANGE);
                ops.push(Op::Insert { index, value });
                len += 1;
            }
            OpKind::Remove => {
                let index = rng.random_range(0..len);
                ops.push(Op::Remove { index });
                len = len.saturating_sub(1);
            }
            OpKind::Reverse => {
                let (start, end) = random_range(rng, len);
                ops.push(Op::Reverse { start, end });
            }
        }
    }

    ops
}

fn seed_base(feature: FeatureSet, size: u64) -> u64 {
    let seed = 0x5EED_2026 ^ (size.wrapping_mul(SEED_MIX));
    let seed = seed ^ (feature_id(feature).wrapping_mul(SEED_MIX.rotate_left(31)));
    mix_seed(seed)
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

fn feature_id(feature: FeatureSet) -> u64 {
    match feature {
        FeatureSet::Core => 1,
        FeatureSet::Agg => 2,
        FeatureSet::AggReverse => 3,
        FeatureSet::AggLazy => 4,
        FeatureSet::Full => 5,
    }
}

fn bench_tree<T, P, M>(
    group: &mut BenchmarkGroup<'_, M>,
    label: &str,
    size: usize,
    feature: FeatureSet,
    base_seed: u64,
    initial: &[i64],
) where
    M: Measurement<Value = Duration>,
    P: LazyMapMonoid<Key = i64>,
    P::Act: ActFromDelta + Clone,
    P::Agg: Clone,
    T: BenchTree<P>,
{
    let mut tree = T::with_seed(base_seed ^ 0x00C0_FFEE);
    tree.extend(initial.iter().copied());

    group.bench_function(BenchmarkId::new(label, size), |bencher| {
        bencher.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            // Reuse the same tree; regenerate ops per iteration from a fixed seed.
            for iter in 0..iters {
                let iter_seed = seed_for_iter(base_seed, iter);
                let mut op_rng = StdRng::seed_from_u64(iter_seed);
                let current_len = tree.len();
                let ops = generate_ops::<P::Act>(feature, current_len, &mut op_rng);
                let start = Instant::now();
                run_ops::<T, P>(&mut tree, &ops);
                black_box(tree.len());
                total += start.elapsed();
            }
            total
        })
    });
}

fn run_ops<T, P>(tree: &mut T, ops: &[Op<i64, P::Act>])
where
    T: SequenceBase<Key = i64>
        + SequenceAgg<Agg = P::Agg>
        + SequenceLazy<Act = P::Act>
        + SequenceReverse,
    P: LazyMapMonoid<Key = i64>,
    P::Act: Clone,
    P::Agg: Clone,
{
    for op in ops {
        match op {
            Op::Get { index } => {
                if let Some(value) = tree.get(*index) {
                    black_box(*value);
                }
            }
            Op::Fold { start, end } => {
                let agg = tree.fold(*start..*end);
                black_box(agg);
            }
            Op::Update { start, end, act } => {
                tree.update(*start..*end, act.clone());
            }
            Op::Insert { index, value } => {
                tree.insert(*index, *value);
            }
            Op::Remove { index } => {
                let removed = tree.remove(*index);
                black_box(removed);
            }
            Op::Reverse { start, end } => {
                tree.reverse(*start..*end);
            }
        }
    }
}

fn feature_label(feature: FeatureSet) -> &'static str {
    match feature {
        FeatureSet::Core => "core",
        FeatureSet::Agg => "agg",
        FeatureSet::AggReverse => "agg_reverse",
        FeatureSet::AggLazy => "agg_lazy",
        FeatureSet::Full => "full",
    }
}
