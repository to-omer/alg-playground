use std::hint::black_box;
use std::time::{Duration, Instant};

use bench::apply_small_runtime_config;
use criterion::measurement::Measurement;
use criterion::{BenchmarkGroup, BenchmarkId, Criterion, criterion_group, criterion_main};

use dynamic_tree::policy::VertexSumAdd;
use dynamic_tree::{EulerTourTree, LinkCutTree, LinkCutTreeSubtree, TopTree};

mod common;

fn apply_runtime_config_for_size<M: Measurement>(_size: usize, group: &mut BenchmarkGroup<'_, M>) {
    apply_small_runtime_config(group);
}

fn bench_connectivity(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic_tree/connectivity");

    for &size in &common::SIZES {
        apply_runtime_config_for_size(size, &mut group);
        let case = common::generate_connectivity_case(size);
        let values = case.values;
        let edges = case.edges;
        let ops = case.ops;
        let link_ops = ops
            .iter()
            .filter(|op| matches!(op, common::ConnOp::Link { .. }))
            .count();
        // TopTree `link` may allocate multiple internal nodes; reserve to avoid reallocations
        // inside the measured loop without over-allocating too much.
        let top_tree_reserve = 3_usize
            .saturating_mul(edges.len().saturating_add(link_ops))
            .saturating_add(64);

        group.bench_function(BenchmarkId::new("lct", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = LinkCutTree::<VertexSumAdd>::new(&values);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::ConnOp::Link { u, v } => {
                                let _ = tree.link(u, v);
                            }
                            common::ConnOp::Cut { u, v } => {
                                let _ = tree.cut(u, v);
                            }
                            common::ConnOp::Connected { u, v } => {
                                black_box(tree.connected(u, v));
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });

        group.bench_function(BenchmarkId::new("lct_subtree", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = LinkCutTreeSubtree::<VertexSumAdd>::new(&values);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::ConnOp::Link { u, v } => {
                                let _ = tree.link(u, v);
                            }
                            common::ConnOp::Cut { u, v } => {
                                let _ = tree.cut(u, v);
                            }
                            common::ConnOp::Connected { u, v } => {
                                black_box(tree.connected(u, v));
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });

        group.bench_function(BenchmarkId::new("ett", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = EulerTourTree::<VertexSumAdd>::new(&values);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::ConnOp::Link { u, v } => {
                                let _ = tree.link(u, v);
                            }
                            common::ConnOp::Cut { u, v } => {
                                let _ = tree.cut(u, v);
                            }
                            common::ConnOp::Connected { u, v } => {
                                black_box(tree.connected(u, v));
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });

        group.bench_function(BenchmarkId::new("top_tree", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = TopTree::<VertexSumAdd>::new(&values);
                    tree.reserve_nodes(top_tree_reserve);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::ConnOp::Link { u, v } => {
                                let _ = tree.link(u, v);
                            }
                            common::ConnOp::Cut { u, v } => {
                                let _ = tree.cut(u, v);
                            }
                            common::ConnOp::Connected { u, v } => {
                                black_box(tree.connected(u, v));
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });
    }

    group.finish();
}

fn bench_path_sum(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic_tree/path_sum");

    for &size in &common::SIZES {
        apply_runtime_config_for_size(size, &mut group);
        let case = common::generate_path_case(size);
        let values = case.values;
        let edges = case.edges;
        let ops = case.ops;
        let link_ops = ops
            .iter()
            .filter(|op| matches!(op, common::PathOp::EdgeSwap { .. }))
            .count();
        let top_tree_reserve = 3_usize
            .saturating_mul(edges.len().saturating_add(link_ops))
            .saturating_add(64);

        group.bench_function(BenchmarkId::new("lct", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = LinkCutTree::<VertexSumAdd>::new(&values);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::PathOp::VertexAdd { v, delta } => tree.vertex_add(v, delta),
                            common::PathOp::PathSum { u, v } => {
                                black_box(tree.path_sum(u, v).unwrap());
                            }
                            common::PathOp::EdgeSwap {
                                cut_u,
                                cut_v,
                                link_u,
                                link_v,
                            } => {
                                tree.cut(cut_u, cut_v);
                                tree.link(link_u, link_v);
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });

        group.bench_function(BenchmarkId::new("lct_subtree", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = LinkCutTreeSubtree::<VertexSumAdd>::new(&values);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::PathOp::VertexAdd { v, delta } => tree.vertex_add(v, delta),
                            common::PathOp::PathSum { u, v } => {
                                black_box(tree.path_sum(u, v).unwrap());
                            }
                            common::PathOp::EdgeSwap {
                                cut_u,
                                cut_v,
                                link_u,
                                link_v,
                            } => {
                                tree.cut(cut_u, cut_v);
                                tree.link(link_u, link_v);
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });

        group.bench_function(BenchmarkId::new("top_tree", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = TopTree::<VertexSumAdd>::new(&values);
                    tree.reserve_nodes(top_tree_reserve);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::PathOp::VertexAdd { v, delta } => tree.vertex_add(v, delta),
                            common::PathOp::PathSum { u, v } => {
                                black_box(tree.path_sum(u, v).unwrap());
                            }
                            common::PathOp::EdgeSwap {
                                cut_u,
                                cut_v,
                                link_u,
                                link_v,
                            } => {
                                tree.cut(cut_u, cut_v);
                                tree.link(link_u, link_v);
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });
    }

    group.finish();
}

fn bench_component_sum(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic_tree/component_sum");

    for &size in &common::SIZES {
        apply_runtime_config_for_size(size, &mut group);
        let case = common::generate_component_case(size);
        let values = case.values;
        let edges = case.edges;
        let ops = case.ops;
        let link_ops = ops
            .iter()
            .filter(|op| matches!(op, common::CompOp::Link { .. }))
            .count();
        let top_tree_reserve = 3_usize
            .saturating_mul(edges.len().saturating_add(link_ops))
            .saturating_add(64);

        group.bench_function(BenchmarkId::new("lct_subtree", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = LinkCutTreeSubtree::<VertexSumAdd>::new(&values);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::CompOp::VertexAdd { v, delta } => tree.vertex_add(v, delta),
                            common::CompOp::ComponentSum { v } => {
                                black_box(tree.component_sum(v));
                            }
                            common::CompOp::Link { u, v } => {
                                let _ = tree.link(u, v);
                            }
                            common::CompOp::Cut { u, v } => {
                                let _ = tree.cut(u, v);
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });

        group.bench_function(BenchmarkId::new("ett", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = EulerTourTree::<VertexSumAdd>::new(&values);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::CompOp::VertexAdd { v, delta } => tree.vertex_add(v, delta),
                            common::CompOp::ComponentSum { v } => {
                                black_box(tree.component_sum(v));
                            }
                            common::CompOp::Link { u, v } => {
                                let _ = tree.link(u, v);
                            }
                            common::CompOp::Cut { u, v } => {
                                let _ = tree.cut(u, v);
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });

        group.bench_function(BenchmarkId::new("top_tree", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = TopTree::<VertexSumAdd>::new(&values);
                    tree.reserve_nodes(top_tree_reserve);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::CompOp::VertexAdd { v, delta } => tree.vertex_add(v, delta),
                            common::CompOp::ComponentSum { v } => {
                                black_box(tree.component_sum(v));
                            }
                            common::CompOp::Link { u, v } => {
                                let _ = tree.link(u, v);
                            }
                            common::CompOp::Cut { u, v } => {
                                let _ = tree.cut(u, v);
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });
    }

    group.finish();
}

fn bench_path_apply(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic_tree/path_apply");

    for &size in &common::SIZES {
        apply_runtime_config_for_size(size, &mut group);
        let case = common::generate_path_apply_case(size);
        let values = case.values;
        let edges = case.edges;
        let ops = case.ops;
        let link_ops = ops
            .iter()
            .filter(|op| matches!(op, common::PathApplyOp::EdgeSwap { .. }))
            .count();
        let top_tree_reserve = 3_usize
            .saturating_mul(edges.len().saturating_add(link_ops))
            .saturating_add(64);

        group.bench_function(BenchmarkId::new("lct", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = LinkCutTree::<VertexSumAdd>::new(&values);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::PathApplyOp::PathApply { u, v, delta } => {
                                tree.path_apply(u, v, delta);
                            }
                            common::PathApplyOp::PathFold { u, v } => {
                                black_box(tree.path_fold(u, v).unwrap());
                            }
                            common::PathApplyOp::EdgeSwap {
                                cut_u,
                                cut_v,
                                link_u,
                                link_v,
                            } => {
                                tree.cut(cut_u, cut_v);
                                tree.link(link_u, link_v);
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });

        group.bench_function(BenchmarkId::new("lct_subtree", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = LinkCutTreeSubtree::<VertexSumAdd>::new(&values);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::PathApplyOp::PathApply { u, v, delta } => {
                                tree.path_apply(u, v, delta);
                            }
                            common::PathApplyOp::PathFold { u, v } => {
                                black_box(tree.path_fold(u, v).unwrap());
                            }
                            common::PathApplyOp::EdgeSwap {
                                cut_u,
                                cut_v,
                                link_u,
                                link_v,
                            } => {
                                tree.cut(cut_u, cut_v);
                                tree.link(link_u, link_v);
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });

        group.bench_function(BenchmarkId::new("top_tree", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = TopTree::<VertexSumAdd>::new(&values);
                    tree.reserve_nodes(top_tree_reserve);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::PathApplyOp::PathApply { u, v, delta } => {
                                tree.path_apply(u, v, delta);
                            }
                            common::PathApplyOp::PathFold { u, v } => {
                                black_box(tree.path_fold(u, v).unwrap());
                            }
                            common::PathApplyOp::EdgeSwap {
                                cut_u,
                                cut_v,
                                link_u,
                                link_v,
                            } => {
                                tree.cut(cut_u, cut_v);
                                tree.link(link_u, link_v);
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });
    }

    group.finish();
}

fn bench_component_apply(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic_tree/component_apply");

    for &size in &common::SIZES {
        apply_runtime_config_for_size(size, &mut group);
        let case = common::generate_component_apply_case(size);
        let values = case.values;
        let edges = case.edges;
        let ops = case.ops;
        let link_ops = ops
            .iter()
            .filter(|op| matches!(op, common::CompApplyOp::Link { .. }))
            .count();
        let top_tree_reserve = 3_usize
            .saturating_mul(edges.len().saturating_add(link_ops))
            .saturating_add(64);

        group.bench_function(BenchmarkId::new("lct_subtree", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = LinkCutTreeSubtree::<VertexSumAdd>::new(&values);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::CompApplyOp::ComponentApply { v, delta } => {
                                tree.component_apply(v, delta);
                            }
                            common::CompApplyOp::ComponentFold { v } => {
                                black_box(tree.component_fold(v));
                            }
                            common::CompApplyOp::Link { u, v } => {
                                tree.link(u, v);
                            }
                            common::CompApplyOp::Cut { u, v } => {
                                tree.cut(u, v);
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });

        group.bench_function(BenchmarkId::new("ett", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = EulerTourTree::<VertexSumAdd>::new(&values);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::CompApplyOp::ComponentApply { v, delta } => {
                                tree.component_apply(v, delta);
                            }
                            common::CompApplyOp::ComponentFold { v } => {
                                black_box(tree.component_fold(v));
                            }
                            common::CompApplyOp::Link { u, v } => {
                                tree.link(u, v);
                            }
                            common::CompApplyOp::Cut { u, v } => {
                                tree.cut(u, v);
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });

        group.bench_function(BenchmarkId::new("top_tree", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = TopTree::<VertexSumAdd>::new(&values);
                    tree.reserve_nodes(top_tree_reserve);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::CompApplyOp::ComponentApply { v, delta } => {
                                tree.component_apply(v, delta);
                            }
                            common::CompApplyOp::ComponentFold { v } => {
                                black_box(tree.component_fold(v));
                            }
                            common::CompApplyOp::Link { u, v } => {
                                tree.link(u, v);
                            }
                            common::CompApplyOp::Cut { u, v } => {
                                tree.cut(u, v);
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });
    }

    group.finish();
}

fn bench_subtree_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic_tree/subtree_ops");

    for &size in &common::SIZES {
        apply_runtime_config_for_size(size, &mut group);
        let case = common::generate_subtree_case(size);
        let values = case.values;
        let edges = case.edges;
        let ops = case.ops;
        let link_ops = ops
            .iter()
            .filter(|op| matches!(op, common::SubtreeOp::EdgeSwap { .. }))
            .count();
        let top_tree_reserve = 3_usize
            .saturating_mul(edges.len().saturating_add(link_ops))
            .saturating_add(64);

        group.bench_function(BenchmarkId::new("lct_subtree", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = LinkCutTreeSubtree::<VertexSumAdd>::new(&values);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::SubtreeOp::SubtreeApply {
                                child,
                                parent,
                                delta,
                            } => tree.subtree_apply(child, parent, delta),
                            common::SubtreeOp::SubtreeFold { child, parent } => {
                                black_box(tree.subtree_fold(child, parent));
                            }
                            common::SubtreeOp::EdgeSwap {
                                cut_u,
                                cut_v,
                                link_u,
                                link_v,
                            } => {
                                tree.cut(cut_u, cut_v);
                                tree.link(link_u, link_v);
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });

        group.bench_function(BenchmarkId::new("ett", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = EulerTourTree::<VertexSumAdd>::new(&values);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::SubtreeOp::SubtreeApply {
                                child,
                                parent,
                                delta,
                            } => tree.subtree_apply(child, parent, delta),
                            common::SubtreeOp::SubtreeFold { child, parent } => {
                                black_box(tree.subtree_fold(child, parent));
                            }
                            common::SubtreeOp::EdgeSwap {
                                cut_u,
                                cut_v,
                                link_u,
                                link_v,
                            } => {
                                tree.cut(cut_u, cut_v);
                                tree.link(link_u, link_v);
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });

        group.bench_function(BenchmarkId::new("top_tree", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = TopTree::<VertexSumAdd>::new(&values);
                    tree.reserve_nodes(top_tree_reserve);
                    for &(u, v) in &edges {
                        tree.link(u, v);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::SubtreeOp::SubtreeApply {
                                child,
                                parent,
                                delta,
                            } => tree.subtree_apply(child, parent, delta),
                            common::SubtreeOp::SubtreeFold { child, parent } => {
                                black_box(tree.subtree_fold(child, parent));
                            }
                            common::SubtreeOp::EdgeSwap {
                                cut_u,
                                cut_v,
                                link_u,
                                link_v,
                            } => {
                                tree.cut(cut_u, cut_v);
                                tree.link(link_u, link_v);
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });
    }

    group.finish();
}

fn bench_edge_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic_tree/edge_ops");

    for &size in &common::SIZES {
        apply_runtime_config_for_size(size, &mut group);
        let case = common::generate_edge_case(size);
        let values = case.values;
        let edges = case.edges;
        let ops = case.ops;
        let top_tree_reserve = 3_usize.saturating_mul(edges.len()).saturating_add(64);

        group.bench_function(BenchmarkId::new("top_tree", size), |bencher| {
            bencher.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut tree = TopTree::<VertexSumAdd>::new(&values);
                    tree.reserve_nodes(top_tree_reserve);
                    for &(u, v, w) in &edges {
                        tree.link_with_edge(u, v, w);
                    }
                    let start = Instant::now();
                    for op in &ops {
                        match *op {
                            common::EdgeOp::Get { u, v } => {
                                black_box(tree.edge_get(u, v).unwrap());
                            }
                            common::EdgeOp::Set { u, v, w } => {
                                tree.edge_set(u, v, w);
                            }
                            common::EdgeOp::Apply { u, v, delta } => {
                                tree.edge_apply(u, v, delta);
                            }
                        }
                    }
                    black_box(tree.len());
                    total += start.elapsed();
                }
                total
            })
        });
    }

    group.finish();
}

fn bench(c: &mut Criterion) {
    bench_connectivity(c);
    bench_path_sum(c);
    bench_component_sum(c);
    bench_path_apply(c);
    bench_component_apply(c);
    bench_subtree_ops(c);
    bench_edge_ops(c);
}

criterion_group!(benches, bench);
criterion_main!(benches);
