use std::collections::VecDeque;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub const SIZES: [usize; 4] = [1_024, 4_096, 16_384, 65_536];
pub const OPS_PER_SIZE: usize = 5_000;
pub const VALUE_RANGE: std::ops::RangeInclusive<i64> = -1_000_000_000..=1_000_000_000;
pub const DELTA_RANGE: std::ops::RangeInclusive<i64> = -1_000..=1_000;

const SEED_MIX: u64 = 0x9E37_79B9_7F4A_7C15;

fn mix_seed(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn rng_for(kind: u64, size: usize) -> StdRng {
    let seed = 0x5EED_2026
        ^ (kind.wrapping_mul(SEED_MIX))
        ^ (size as u64).wrapping_mul(SEED_MIX.rotate_left(17));
    StdRng::seed_from_u64(mix_seed(seed))
}

fn generate_values(rng: &mut impl Rng, n: usize) -> Vec<i64> {
    let mut values = Vec::with_capacity(n);
    for _ in 0..n {
        values.push(rng.random_range(VALUE_RANGE));
    }
    values
}

fn generate_random_tree_edges(rng: &mut impl Rng, n: usize) -> Vec<(usize, usize)> {
    if n <= 1 {
        return Vec::new();
    }
    let mut edges = Vec::with_capacity(n - 1);
    for i in 1..n {
        let parent = rng.random_range(0..i);
        edges.push((i, parent));
    }
    edges
}

#[derive(Clone, Copy, Debug)]
pub enum ConnOp {
    Link { u: usize, v: usize },
    Cut { u: usize, v: usize },
    Connected { u: usize, v: usize },
}

#[derive(Clone, Copy, Debug)]
pub enum PathOp {
    VertexAdd {
        v: usize,
        delta: i64,
    },
    PathSum {
        u: usize,
        v: usize,
    },
    EdgeSwap {
        cut_u: usize,
        cut_v: usize,
        link_u: usize,
        link_v: usize,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum CompOp {
    VertexAdd { v: usize, delta: i64 },
    ComponentSum { v: usize },
    Link { u: usize, v: usize },
    Cut { u: usize, v: usize },
}

#[derive(Clone, Copy, Debug)]
pub enum PathApplyOp {
    PathApply {
        u: usize,
        v: usize,
        delta: i64,
    },
    PathFold {
        u: usize,
        v: usize,
    },
    EdgeSwap {
        cut_u: usize,
        cut_v: usize,
        link_u: usize,
        link_v: usize,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum CompApplyOp {
    ComponentApply { v: usize, delta: i64 },
    ComponentFold { v: usize },
    Link { u: usize, v: usize },
    Cut { u: usize, v: usize },
}

#[derive(Clone, Copy, Debug)]
pub enum SubtreeOp {
    SubtreeApply {
        child: usize,
        parent: usize,
        delta: i64,
    },
    SubtreeFold {
        child: usize,
        parent: usize,
    },
    EdgeSwap {
        cut_u: usize,
        cut_v: usize,
        link_u: usize,
        link_v: usize,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum EdgeOp {
    Get { u: usize, v: usize },
    Set { u: usize, v: usize, w: i64 },
    Apply { u: usize, v: usize, delta: i64 },
}

#[derive(Clone, Debug)]
pub struct ConnectivityCase {
    pub values: Vec<i64>,
    pub edges: Vec<(usize, usize)>,
    pub ops: Vec<ConnOp>,
}

#[derive(Clone, Debug)]
pub struct PathCase {
    pub values: Vec<i64>,
    pub edges: Vec<(usize, usize)>,
    pub ops: Vec<PathOp>,
}

#[derive(Clone, Debug)]
pub struct ComponentCase {
    pub values: Vec<i64>,
    pub edges: Vec<(usize, usize)>,
    pub ops: Vec<CompOp>,
}

#[derive(Clone, Debug)]
pub struct PathApplyCase {
    pub values: Vec<i64>,
    pub edges: Vec<(usize, usize)>,
    pub ops: Vec<PathApplyOp>,
}

#[derive(Clone, Debug)]
pub struct ComponentApplyCase {
    pub values: Vec<i64>,
    pub edges: Vec<(usize, usize)>,
    pub ops: Vec<CompApplyOp>,
}

#[derive(Clone, Debug)]
pub struct SubtreeCase {
    pub values: Vec<i64>,
    pub edges: Vec<(usize, usize)>,
    pub ops: Vec<SubtreeOp>,
}

#[derive(Clone, Debug)]
pub struct EdgeCase {
    pub values: Vec<i64>,
    pub edges: Vec<(usize, usize, i64)>, // (u, v, weight)
    pub ops: Vec<EdgeOp>,
}

struct ForestState {
    n: usize,
    adj: Vec<Vec<usize>>,
    comp_id: Vec<usize>,
    comps: Vec<Vec<usize>>,
    edges: Vec<(usize, usize)>,
    mark: Vec<u32>,
    mark_gen: u32,
    comp_cnt: usize,
}

impl ForestState {
    fn new(n: usize) -> Self {
        let mut comps = Vec::with_capacity(n);
        for i in 0..n {
            comps.push(vec![i]);
        }
        Self {
            n,
            adj: vec![Vec::new(); n],
            comp_id: (0..n).collect(),
            comps,
            edges: Vec::new(),
            mark: vec![0; n],
            mark_gen: 1,
            comp_cnt: n,
        }
    }

    fn add_edge_undirected(&mut self, u: usize, v: usize) {
        self.adj[u].push(v);
        self.adj[v].push(u);
        self.edges.push((u, v));
    }

    fn remove_adj_one(vec: &mut Vec<usize>, x: usize) {
        if let Some(pos) = vec.iter().position(|&y| y == x) {
            vec.swap_remove(pos);
        } else {
            debug_assert!(false, "edge not found in adjacency");
        }
    }

    fn remove_edge_undirected(&mut self, u: usize, v: usize) {
        Self::remove_adj_one(&mut self.adj[u], v);
        Self::remove_adj_one(&mut self.adj[v], u);
    }

    fn link(&mut self, u: usize, v: usize) {
        debug_assert!(self.comp_id[u] != self.comp_id[v]);
        self.add_edge_undirected(u, v);
        let mut a = self.comp_id[u];
        let mut b = self.comp_id[v];
        if self.comps[a].len() < self.comps[b].len() {
            std::mem::swap(&mut a, &mut b);
        }
        let moved = std::mem::take(&mut self.comps[b]);
        for x in moved {
            self.comp_id[x] = a;
            self.comps[a].push(x);
        }
        self.comp_cnt -= 1;
    }

    fn bfs_component(&mut self, start: usize) -> Vec<usize> {
        let tag = self.mark_gen;
        self.mark_gen = self.mark_gen.wrapping_add(1);
        let mut q = VecDeque::new();
        q.push_back(start);
        self.mark[start] = tag;
        let mut verts = Vec::new();
        while let Some(v) = q.pop_front() {
            verts.push(v);
            for &to in &self.adj[v] {
                if self.mark[to] == tag {
                    continue;
                }
                self.mark[to] = tag;
                q.push_back(to);
            }
        }
        verts
    }

    fn cut_by_index(&mut self, idx: usize) -> (usize, usize) {
        let (u, v) = self.edges.swap_remove(idx);
        let old = self.comp_id[u];
        debug_assert_eq!(old, self.comp_id[v]);
        self.remove_edge_undirected(u, v);

        // Split component by BFS.
        self.bfs_component(u);
        let tag = self.mark_gen.wrapping_sub(1);

        let old_list = std::mem::take(&mut self.comps[old]);
        let mut a = Vec::new();
        let mut b = Vec::new();
        for x in old_list {
            if self.mark[x] == tag {
                a.push(x);
            } else {
                b.push(x);
            }
        }
        debug_assert!(!a.is_empty() && !b.is_empty());

        self.comps[old] = a;
        for &x in &self.comps[old] {
            self.comp_id[x] = old;
        }
        let new_id = self.comps.len();
        for &x in &b {
            self.comp_id[x] = new_id;
        }
        self.comps.push(b);
        self.comp_cnt += 1;

        (u, v)
    }

    fn random_vertex_in_comp(&self, rng: &mut impl Rng, comp: usize) -> usize {
        let list = &self.comps[comp];
        list[rng.random_range(0..list.len())]
    }

    fn pick_two_components(&self, rng: &mut impl Rng) -> Option<(usize, usize)> {
        if self.comp_cnt <= 1 {
            return None;
        }
        let a = rng.random_range(0..self.n);
        let ca = self.comp_id[a];
        let mut tries = 0;
        loop {
            let b = rng.random_range(0..self.n);
            let cb = self.comp_id[b];
            if ca != cb {
                return Some((ca, cb));
            }
            tries += 1;
            if tries >= 100 {
                break;
            }
        }
        // Fallback: linear scan.
        for b in 0..self.n {
            let cb = self.comp_id[b];
            if cb != ca {
                return Some((ca, cb));
            }
        }
        None
    }
}

pub fn generate_connectivity_case(n: usize) -> ConnectivityCase {
    let mut rng = rng_for(1, n);
    let values = vec![0_i64; n];
    let mut state = ForestState::new(n);

    for (u, v) in generate_random_tree_edges(&mut rng, n) {
        state.link(u, v);
    }
    let init_cuts = n / 4;
    for _ in 0..init_cuts {
        if state.edges.is_empty() {
            break;
        }
        let idx = rng.random_range(0..state.edges.len());
        state.cut_by_index(idx);
    }
    let edges = state.edges.clone();

    let mut ops = Vec::with_capacity(OPS_PER_SIZE);
    for _ in 0..OPS_PER_SIZE {
        let roll = rng.random_range(0..100_u32);
        if roll < 50 {
            let u = rng.random_range(0..n);
            let v = rng.random_range(0..n);
            ops.push(ConnOp::Connected { u, v });
            continue;
        }

        if roll < 75 {
            // link
            if let Some((ca, cb)) = state.pick_two_components(&mut rng) {
                let u = state.random_vertex_in_comp(&mut rng, ca);
                let v = state.random_vertex_in_comp(&mut rng, cb);
                state.link(u, v);
                ops.push(ConnOp::Link { u, v });
            } else {
                let u = rng.random_range(0..n);
                let v = rng.random_range(0..n);
                ops.push(ConnOp::Connected { u, v });
            }
            continue;
        }

        // cut
        if state.edges.is_empty() {
            let u = rng.random_range(0..n);
            let v = rng.random_range(0..n);
            ops.push(ConnOp::Connected { u, v });
            continue;
        }
        let idx = rng.random_range(0..state.edges.len());
        let (u, v) = state.cut_by_index(idx);
        ops.push(ConnOp::Cut { u, v });
    }

    ConnectivityCase { values, edges, ops }
}

pub fn generate_path_case(n: usize) -> PathCase {
    let mut rng = rng_for(2, n);
    let values = generate_values(&mut rng, n);
    let mut state = ForestState::new(n);
    for (u, v) in generate_random_tree_edges(&mut rng, n) {
        state.link(u, v);
    }
    let edges = state.edges.clone();

    let mut ops = Vec::with_capacity(OPS_PER_SIZE);
    for _ in 0..OPS_PER_SIZE {
        let roll = rng.random_range(0..100_u32);
        if roll < 30 {
            let v = rng.random_range(0..n);
            let delta = rng.random_range(DELTA_RANGE);
            ops.push(PathOp::VertexAdd { v, delta });
        } else if roll < 70 {
            let u = rng.random_range(0..n);
            let v = rng.random_range(0..n);
            ops.push(PathOp::PathSum { u, v });
        } else {
            let idx = rng.random_range(0..state.edges.len());
            let (cut_u, cut_v) = state.cut_by_index(idx);
            let ca = state.comp_id[cut_u];
            let cb = state.comp_id[cut_v];
            let link_u = state.random_vertex_in_comp(&mut rng, ca);
            let link_v = state.random_vertex_in_comp(&mut rng, cb);
            state.link(link_u, link_v);
            ops.push(PathOp::EdgeSwap {
                cut_u,
                cut_v,
                link_u,
                link_v,
            });
        }
    }

    PathCase { values, edges, ops }
}

pub fn generate_component_case(n: usize) -> ComponentCase {
    let mut rng = rng_for(3, n);
    let values = generate_values(&mut rng, n);
    let mut state = ForestState::new(n);
    for (u, v) in generate_random_tree_edges(&mut rng, n) {
        state.link(u, v);
    }
    let init_cuts = n / 8;
    for _ in 0..init_cuts {
        if state.edges.is_empty() {
            break;
        }
        let idx = rng.random_range(0..state.edges.len());
        state.cut_by_index(idx);
    }
    let edges = state.edges.clone();

    let mut ops = Vec::with_capacity(OPS_PER_SIZE);
    for _ in 0..OPS_PER_SIZE {
        let roll = rng.random_range(0..100_u32);
        if roll < 30 {
            let v = rng.random_range(0..n);
            let delta = rng.random_range(DELTA_RANGE);
            ops.push(CompOp::VertexAdd { v, delta });
        } else if roll < 70 {
            let v = rng.random_range(0..n);
            ops.push(CompOp::ComponentSum { v });
        } else if roll < 85 {
            // link
            if let Some((ca, cb)) = state.pick_two_components(&mut rng) {
                let u = state.random_vertex_in_comp(&mut rng, ca);
                let v = state.random_vertex_in_comp(&mut rng, cb);
                state.link(u, v);
                ops.push(CompOp::Link { u, v });
            } else {
                let v = rng.random_range(0..n);
                ops.push(CompOp::ComponentSum { v });
            }
        } else {
            // cut
            if state.edges.is_empty() {
                let v = rng.random_range(0..n);
                ops.push(CompOp::ComponentSum { v });
                continue;
            }
            let idx = rng.random_range(0..state.edges.len());
            let (u, v) = state.cut_by_index(idx);
            ops.push(CompOp::Cut { u, v });
        }
    }

    ComponentCase { values, edges, ops }
}

pub fn generate_path_apply_case(n: usize) -> PathApplyCase {
    let mut rng = rng_for(4, n);
    let values = generate_values(&mut rng, n);
    let mut state = ForestState::new(n);
    for (u, v) in generate_random_tree_edges(&mut rng, n) {
        state.link(u, v);
    }
    let edges = state.edges.clone();

    let mut ops = Vec::with_capacity(OPS_PER_SIZE);
    for _ in 0..OPS_PER_SIZE {
        let roll = rng.random_range(0..100_u32);
        if roll < 40 {
            let u = rng.random_range(0..n);
            let v = rng.random_range(0..n);
            let delta = rng.random_range(DELTA_RANGE);
            ops.push(PathApplyOp::PathApply { u, v, delta });
        } else if roll < 80 {
            let u = rng.random_range(0..n);
            let v = rng.random_range(0..n);
            ops.push(PathApplyOp::PathFold { u, v });
        } else {
            let idx = rng.random_range(0..state.edges.len());
            let (cut_u, cut_v) = state.cut_by_index(idx);
            let ca = state.comp_id[cut_u];
            let cb = state.comp_id[cut_v];
            let link_u = state.random_vertex_in_comp(&mut rng, ca);
            let link_v = state.random_vertex_in_comp(&mut rng, cb);
            state.link(link_u, link_v);
            ops.push(PathApplyOp::EdgeSwap {
                cut_u,
                cut_v,
                link_u,
                link_v,
            });
        }
    }

    PathApplyCase { values, edges, ops }
}

pub fn generate_component_apply_case(n: usize) -> ComponentApplyCase {
    let mut rng = rng_for(5, n);
    let values = generate_values(&mut rng, n);
    let mut state = ForestState::new(n);
    for (u, v) in generate_random_tree_edges(&mut rng, n) {
        state.link(u, v);
    }
    let init_cuts = n / 8;
    for _ in 0..init_cuts {
        if state.edges.is_empty() {
            break;
        }
        let idx = rng.random_range(0..state.edges.len());
        state.cut_by_index(idx);
    }
    let edges = state.edges.clone();

    let mut ops = Vec::with_capacity(OPS_PER_SIZE);
    for _ in 0..OPS_PER_SIZE {
        let roll = rng.random_range(0..100_u32);
        if roll < 30 {
            let v = rng.random_range(0..n);
            let delta = rng.random_range(DELTA_RANGE);
            ops.push(CompApplyOp::ComponentApply { v, delta });
        } else if roll < 70 {
            let v = rng.random_range(0..n);
            ops.push(CompApplyOp::ComponentFold { v });
        } else if roll < 85 {
            // link
            if let Some((ca, cb)) = state.pick_two_components(&mut rng) {
                let u = state.random_vertex_in_comp(&mut rng, ca);
                let v = state.random_vertex_in_comp(&mut rng, cb);
                state.link(u, v);
                ops.push(CompApplyOp::Link { u, v });
            } else {
                let v = rng.random_range(0..n);
                ops.push(CompApplyOp::ComponentFold { v });
            }
        } else {
            // cut
            if state.edges.is_empty() {
                let v = rng.random_range(0..n);
                ops.push(CompApplyOp::ComponentFold { v });
                continue;
            }
            let idx = rng.random_range(0..state.edges.len());
            let (u, v) = state.cut_by_index(idx);
            ops.push(CompApplyOp::Cut { u, v });
        }
    }

    ComponentApplyCase { values, edges, ops }
}

pub fn generate_subtree_case(n: usize) -> SubtreeCase {
    let mut rng = rng_for(6, n);
    let values = generate_values(&mut rng, n);
    let mut state = ForestState::new(n);
    for (u, v) in generate_random_tree_edges(&mut rng, n) {
        state.link(u, v);
    }
    let edges = state.edges.clone();

    let mut ops = Vec::with_capacity(OPS_PER_SIZE);
    for _ in 0..OPS_PER_SIZE {
        let roll = rng.random_range(0..100_u32);
        if roll < 30 {
            let idx = rng.random_range(0..state.edges.len());
            let (a, b) = state.edges[idx];
            let (child, parent) = if rng.random_bool(0.5) { (a, b) } else { (b, a) };
            let delta = rng.random_range(DELTA_RANGE);
            ops.push(SubtreeOp::SubtreeApply {
                child,
                parent,
                delta,
            });
        } else if roll < 60 {
            let idx = rng.random_range(0..state.edges.len());
            let (a, b) = state.edges[idx];
            let (child, parent) = if rng.random_bool(0.5) { (a, b) } else { (b, a) };
            ops.push(SubtreeOp::SubtreeFold { child, parent });
        } else {
            let idx = rng.random_range(0..state.edges.len());
            let (cut_u, cut_v) = state.cut_by_index(idx);
            let ca = state.comp_id[cut_u];
            let cb = state.comp_id[cut_v];
            let link_u = state.random_vertex_in_comp(&mut rng, ca);
            let link_v = state.random_vertex_in_comp(&mut rng, cb);
            state.link(link_u, link_v);
            ops.push(SubtreeOp::EdgeSwap {
                cut_u,
                cut_v,
                link_u,
                link_v,
            });
        }
    }

    SubtreeCase { values, edges, ops }
}

pub fn generate_edge_case(n: usize) -> EdgeCase {
    let mut rng = rng_for(7, n);
    let values = generate_values(&mut rng, n);
    let mut state = ForestState::new(n);
    let mut edges = Vec::with_capacity(n.saturating_sub(1));
    for (u, v) in generate_random_tree_edges(&mut rng, n) {
        state.link(u, v);
        let w = rng.random_range(DELTA_RANGE);
        edges.push((u, v, w));
    }

    let mut ops = Vec::with_capacity(OPS_PER_SIZE);
    for _ in 0..OPS_PER_SIZE {
        let idx = rng.random_range(0..state.edges.len());
        let (u, v) = state.edges[idx];
        let roll = rng.random_range(0..100_u32);
        if roll < 40 {
            ops.push(EdgeOp::Get { u, v });
        } else if roll < 70 {
            let w = rng.random_range(DELTA_RANGE);
            ops.push(EdgeOp::Set { u, v, w });
        } else {
            let delta = rng.random_range(DELTA_RANGE);
            ops.push(EdgeOp::Apply { u, v, delta });
        }
    }

    EdgeCase { values, edges, ops }
}
