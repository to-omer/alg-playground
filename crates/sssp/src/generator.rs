use std::collections::HashSet;

use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

use crate::graph::DirectedGraph;

const C_MAX: u64 = 1_000_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum GraphCase {
    SparseRandom,
    MaxSparseRandom,
    MaxDenseRandom,
    MaxDenseLong,
    MaxDenseZero,
    AlmostLine,
    GridRandom,
    GridSwirl,
    WrongDijkstraKiller,
    SpfaKiller,
}

impl GraphCase {
    pub fn label(self) -> &'static str {
        match self {
            Self::SparseRandom => "sparse_random",
            Self::MaxSparseRandom => "max_sparse_random",
            Self::MaxDenseRandom => "max_dense_random",
            Self::MaxDenseLong => "max_dense_long",
            Self::MaxDenseZero => "max_dense_zero",
            Self::AlmostLine => "almost_line",
            Self::GridRandom => "grid_random",
            Self::GridSwirl => "grid_swirl",
            Self::WrongDijkstraKiller => "wrong_dijkstra_killer",
            Self::SpfaKiller => "spfa_killer",
        }
    }
}

#[derive(Clone, Debug)]
pub struct GeneratedGraph {
    pub graph: DirectedGraph,
    pub source: usize,
    pub target: usize,
}

pub fn generate_case(case: GraphCase, size: usize, seed: u64) -> GeneratedGraph {
    match case {
        GraphCase::SparseRandom => sparse_random_case(size.max(32), seed, 4),
        GraphCase::MaxSparseRandom => sparse_random_case(size.max(32), seed ^ 0x5A5A, 8),
        GraphCase::MaxDenseRandom => max_dense_random_case(size.max(256), seed),
        GraphCase::MaxDenseLong => max_dense_long_case(size.max(256), seed),
        GraphCase::MaxDenseZero => max_dense_zero_case(size.max(256), seed),
        GraphCase::AlmostLine => almost_line_case(size.max(64), seed),
        GraphCase::GridRandom => grid_random_case(size.max(256), seed),
        GraphCase::GridSwirl => grid_swirl_case(size.max(256), seed),
        GraphCase::WrongDijkstraKiller => wrong_dijkstra_killer_case(size.max(512), seed),
        GraphCase::SpfaKiller => spfa_killer_case(size.max(1_024), seed),
    }
}

fn sparse_random_case(size: usize, seed: u64, edge_factor: usize) -> GeneratedGraph {
    let mut rng = StdRng::seed_from_u64(seed);
    let n = size.max(2);
    let m_target = (n.saturating_mul(edge_factor)).min(complete_edges(n));
    let mut edges = Vec::with_capacity(m_target);
    let mut used = HashSet::with_capacity(m_target * 2 + 1);

    while edges.len() < m_target {
        let u = rng.random_range(0..n);
        let v = rng.random_range(0..n);
        if u == v {
            continue;
        }
        push_unique_edge(&mut edges, &mut used, u, v, rng.random_range(0..=C_MAX));
    }

    let source = rng.random_range(0..n);
    let mut target = rng.random_range(0..n);
    if source == target {
        target = (target + 1) % n;
    }

    GeneratedGraph {
        graph: DirectedGraph::from_edges(n, &edges),
        source,
        target,
    }
}

fn max_dense_random_case(size: usize, seed: u64) -> GeneratedGraph {
    let mut rng = StdRng::seed_from_u64(seed);
    let n = floor_sqrt(size).max(8);
    let mut edges = Vec::with_capacity(complete_edges(n));

    for u in 0..n {
        for v in 0..n {
            if u == v {
                continue;
            }
            edges.push((u as u32, v as u32, rng.random_range(0..=C_MAX)));
        }
    }

    let source = rng.random_range(0..n);
    let mut target = rng.random_range(0..n);
    if source == target {
        target = (target + 1) % n;
    }

    GeneratedGraph {
        graph: DirectedGraph::from_edges(n, &edges),
        source,
        target,
    }
}

fn max_dense_zero_case(size: usize, seed: u64) -> GeneratedGraph {
    let mut rng = StdRng::seed_from_u64(seed);
    let n = floor_sqrt(size).max(8);
    let mut edges = Vec::with_capacity(complete_edges(n));

    for u in 0..n {
        for v in 0..n {
            if u != v {
                edges.push((u as u32, v as u32, 0));
            }
        }
    }

    let source = rng.random_range(0..n);
    let mut target = rng.random_range(0..n);
    if source == target {
        target = (target + 1) % n;
    }

    GeneratedGraph {
        graph: DirectedGraph::from_edges(n, &edges),
        source,
        target,
    }
}

fn max_dense_long_case(size: usize, seed: u64) -> GeneratedGraph {
    let mut rng = StdRng::seed_from_u64(seed);
    let n = floor_sqrt(size).max(8);

    let mut path: Vec<usize> = (0..n).collect();
    path.shuffle(&mut rng);

    let mut next = vec![usize::MAX; n];
    for i in 0..(n - 1) {
        next[path[i]] = path[i + 1];
    }

    let mut edges = Vec::with_capacity(complete_edges(n));
    for (u, &next_u) in next.iter().enumerate().take(n) {
        for v in 0..n {
            if u == v {
                continue;
            }
            let w = if next_u == v {
                rng.random_range(0..=10)
            } else {
                rng.random_range(1_000..=C_MAX)
            };
            edges.push((u as u32, v as u32, w));
        }
    }

    GeneratedGraph {
        graph: DirectedGraph::from_edges(n, &edges),
        source: path[0],
        target: path[n - 1],
    }
}

fn almost_line_case(size: usize, seed: u64) -> GeneratedGraph {
    let mut rng = StdRng::seed_from_u64(seed);
    let n = size.max(8);
    let mut edges = Vec::with_capacity(n * 2);
    let mut used = HashSet::with_capacity(n * 4);

    for i in 0..(n - 1) {
        push_unique_edge(&mut edges, &mut used, i, i + 1, rng.random_range(0..=C_MAX));
    }

    let m_target = (n.saturating_mul(2)).min(complete_edges(n));
    while edges.len() < m_target {
        let a = rng.random_range(0..(n - 2));
        let mut b = a + rng.random_range(2..=3);
        if b >= n {
            b = n - 1;
        }
        let (u, v) = if rng.random_bool(0.5) { (b, a) } else { (a, b) };
        push_unique_edge(&mut edges, &mut used, u, v, rng.random_range(0..=C_MAX));
    }

    let mut perm: Vec<usize> = (0..n).collect();
    perm.shuffle(&mut rng);
    let source = perm[0];
    let target = perm[n - 1];

    for edge in &mut edges {
        edge.0 = perm[edge.0 as usize] as u32;
        edge.1 = perm[edge.1 as usize] as u32;
    }
    edges.shuffle(&mut rng);

    GeneratedGraph {
        graph: DirectedGraph::from_edges(n, &edges),
        source,
        target,
    }
}

fn grid_random_case(size: usize, seed: u64) -> GeneratedGraph {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut len = floor_sqrt((size / 4).max(16));
    len = len.max(4);
    if len % 2 == 1 {
        len += 1;
    }

    let n = len * len;
    let mut edges = Vec::with_capacity(n * 4);

    let index = |i: usize, j: usize| -> usize { i * len + j };
    for i in 0..len {
        for j in 0..len {
            if j + 1 < len {
                edges.push((
                    index(i, j) as u32,
                    index(i, j + 1) as u32,
                    rng.random_range(0..=C_MAX),
                ));
            }
            if i + 1 < len {
                edges.push((
                    index(i, j) as u32,
                    index(i + 1, j) as u32,
                    rng.random_range(0..=C_MAX),
                ));
            }
            if j > 0 {
                edges.push((
                    index(i, j) as u32,
                    index(i, j - 1) as u32,
                    rng.random_range(0..=C_MAX),
                ));
            }
            if i > 0 {
                edges.push((
                    index(i, j) as u32,
                    index(i - 1, j) as u32,
                    rng.random_range(0..=C_MAX),
                ));
            }
        }
    }

    edges.shuffle(&mut rng);
    let source = rng.random_range(0..n);
    let mut target = rng.random_range(0..n);
    if source == target {
        target = (target + 1) % n;
    }

    GeneratedGraph {
        graph: DirectedGraph::from_edges(n, &edges),
        source,
        target,
    }
}

fn grid_swirl_case(size: usize, seed: u64) -> GeneratedGraph {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut len = floor_sqrt((size / 4).max(16));
    len = len.max(4);
    if len % 2 == 1 {
        len += 1;
    }

    let n = len * len;
    let mut edges = Vec::with_capacity(n * 4);
    let index = |i: usize, j: usize| -> usize { i * len + j };

    for i in 0..len {
        for j in 0..len {
            if j + 1 < len {
                let w = if j < len - 1 - i && j + 1 >= i {
                    rng.random_range(0..=10)
                } else {
                    rng.random_range(0..=C_MAX)
                };
                edges.push((index(i, j) as u32, index(i, j + 1) as u32, w));
            }
            if i + 1 < len {
                let w = if j >= len - 1 - i && j > i {
                    rng.random_range(0..=10)
                } else {
                    rng.random_range(0..=C_MAX)
                };
                edges.push((index(i, j) as u32, index(i + 1, j) as u32, w));
            }
            if j > 0 {
                let w = if j >= len - i && j <= i {
                    rng.random_range(0..=10)
                } else {
                    rng.random_range(0..=C_MAX)
                };
                edges.push((index(i, j) as u32, index(i, j - 1) as u32, w));
            }
            if i > 0 {
                let w = if j < i && j < len - i {
                    rng.random_range(0..=10)
                } else {
                    rng.random_range(0..=C_MAX)
                };
                edges.push((index(i, j) as u32, index(i - 1, j) as u32, w));
            }
        }
    }

    edges.shuffle(&mut rng);
    GeneratedGraph {
        graph: DirectedGraph::from_edges(n, &edges),
        source: 0,
        target: index(len / 2, len / 2 - 1),
    }
}

fn wrong_dijkstra_killer_case(size: usize, seed: u64) -> GeneratedGraph {
    let mut rng = StdRng::seed_from_u64(seed);

    let (n, mut source, mut target, mut edges, shuffle_vertices) = if seed & 1 == 0 {
        let one = (size / 4).max(8);
        let n = one * 2 + 3;
        let mut edges: Vec<(u32, u32, u64)> = Vec::new();

        for i in 0..one {
            edges.push((0, (i + 1) as u32, i as u64));
            edges.push(((i + 1) as u32, (one + 1) as u32, ((one - i - 1) * 2) as u64));
            edges.push(((one + 1) as u32, (one + 2 + i) as u32, (2 * i) as u64));
            edges.push(((one + 2 + i) as u32, (n - 1) as u32, (one - i - 1) as u64));
        }

        (n, 0, n - 1, edges, false)
    } else {
        let k = 12;
        let repeat = (size / (6 * k)).max(1);
        let n = 4 * k * repeat;
        let mut edges: Vec<(u32, u32, u64)> = Vec::new();

        for i in 0..repeat {
            for j in 0..k {
                let base = (i * k + j) * 4;
                edges.push((base as u32, (base + 1) as u32, 0));
                edges.push((base as u32, (base + 2) as u32, 1));
                edges.push(((base + 1) as u32, (base + 3) as u32, C_MAX >> (j * 2)));
                edges.push(((base + 2) as u32, (base + 3) as u32, 0));
                if base + 4 < n {
                    edges.push((base as u32, (base + 4) as u32, C_MAX >> (j * 2 + 1)));
                    edges.push(((base + 3) as u32, (base + 4) as u32, 0));
                }
            }
        }

        (n, 0, n - 1, edges, true)
    };

    edges.shuffle(&mut rng);
    if shuffle_vertices {
        let mut perm: Vec<usize> = (0..n).collect();
        perm.shuffle(&mut rng);
        for edge in &mut edges {
            edge.0 = perm[edge.0 as usize] as u32;
            edge.1 = perm[edge.1 as usize] as u32;
        }
        source = perm[source];
        target = perm[target];
    }

    GeneratedGraph {
        graph: DirectedGraph::from_edges(n, &edges),
        source,
        target,
    }
}

fn spfa_killer_case(size: usize, seed: u64) -> GeneratedGraph {
    let mut rng = StdRng::seed_from_u64(seed);

    let long_part = (size / 5).max(64);
    let dist_part = (size / 20).clamp(16, 512);
    let a = (size / 18).max(8);
    let b = (size / 9).max(8);

    let hub = a * 3 + b;
    let n = hub + 1 + long_part + dist_part;
    let long_base = n - long_part;
    let dist_base = n - long_part - dist_part;

    let mut edges = Vec::new();
    let mut used_long = HashSet::new();

    for i in 0..a {
        if i + 1 < a {
            edges.push(((i * 3) as u32, (i * 3 + 1) as u32, 1));
            edges.push(((i * 3 + 1) as u32, (i * 3 + 3) as u32, 2));
        }
        edges.push(((i * 3) as u32, (i * 3 + 2) as u32, 2));
        edges.push(((i * 3) as u32, hub as u32, (4 * (a - i)) as u64));
    }

    for i in 0..b {
        edges.push((hub as u32, (a * 3 + i) as u32, 1));
    }

    edges.push(((a * 3) as u32, long_base as u32, 1));
    for i in 0..(long_part - 1) {
        edges.push(((long_base + i) as u32, (long_base + i + 1) as u32, 1));
    }

    for _ in 0..long_part {
        let x = rng.random_range(0..(long_part - 2));
        let y = x + rng.random_range(2..=3);
        if y < long_part && used_long.insert(((x as u64) << 32) | y as u64) {
            let w = (y - x) as i64 + rng.random_range(-1..=1);
            edges.push((
                (long_base + x) as u32,
                (long_base + y) as u32,
                w.max(0) as u64,
            ));
        }
    }

    for i in 0..dist_part {
        edges.push(((a * 3) as u32, (dist_base + i) as u32, C_MAX));
    }

    let mut perm: Vec<usize> = (0..n).collect();
    perm.shuffle(&mut rng);
    for edge in &mut edges {
        edge.0 = perm[edge.0 as usize] as u32;
        edge.1 = perm[edge.1 as usize] as u32;
    }

    let source = perm[0];
    let target = perm[n - 1];
    edges.shuffle(&mut rng);

    GeneratedGraph {
        graph: DirectedGraph::from_edges(n, &edges),
        source,
        target,
    }
}

#[inline]
fn complete_edges(n: usize) -> usize {
    n.saturating_mul(n.saturating_sub(1))
}

#[inline]
fn floor_sqrt(value: usize) -> usize {
    (value as f64).sqrt().floor() as usize
}

#[inline]
fn push_unique_edge(
    edges: &mut Vec<(u32, u32, u64)>,
    used: &mut HashSet<u64>,
    u: usize,
    v: usize,
    weight: u64,
) -> bool {
    if u == v {
        return false;
    }
    let key = ((u as u64) << 32) | v as u64;
    if used.insert(key) {
        edges.push((u as u32, v as u32, weight));
        true
    } else {
        false
    }
}
