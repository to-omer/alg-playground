use std::collections::VecDeque;

const UNVISITED: u64 = u64::MAX;
const NONE: usize = usize::MAX;

pub fn diameter_vec(n: usize, edges: &[(usize, usize, u64)]) -> u64 {
    if n <= 1 {
        return 0;
    }
    let mut adj = vec![Vec::new(); n];
    for &(u, v, w) in edges {
        adj[u].push((v, w));
        adj[v].push((u, w));
    }
    let (start, _) = farthest_vec(0, &adj);
    let (_, dist) = farthest_vec(start, &adj);
    dist
}

pub fn diameter_chinese(n: usize, edges: &[(usize, usize, u64)]) -> u64 {
    if n <= 1 {
        return 0;
    }
    let adj = ChineseAdj::new(n, edges);
    let (start, _) = farthest_chinese(0, &adj);
    let (_, dist) = farthest_chinese(start, &adj);
    dist
}

pub fn diameter_csr(n: usize, edges: &[(usize, usize, u64)]) -> u64 {
    if n <= 1 {
        return 0;
    }
    let adj = Csr::new(n, edges);
    let (start, _) = farthest_csr(0, &adj);
    let (_, dist) = farthest_csr(start, &adj);
    dist
}

pub fn diameter_xor(n: usize, edges: &[(usize, usize, u64)]) -> u64 {
    if n <= 1 {
        return 0;
    }
    let mut degree = vec![0_usize; n];
    let mut xor_edge = vec![0_usize; n];
    for (id, &(u, v, _)) in edges.iter().enumerate() {
        degree[u] += 1;
        degree[v] += 1;
        xor_edge[u] ^= id;
        xor_edge[v] ^= id;
    }

    let mut best1 = vec![0_u64; n];
    let mut best2 = vec![0_u64; n];
    let mut queue = VecDeque::with_capacity(n);
    for (v, deg) in degree.iter().enumerate() {
        if *deg <= 1 {
            queue.push_back(v);
        }
    }

    while let Some(v) = queue.pop_front() {
        if degree[v] == 0 {
            continue;
        }
        let edge_id = xor_edge[v];
        let (a, b, w) = edges[edge_id];
        let u = if a == v { b } else { a };
        degree[v] = 0;

        let candidate = best1[v].saturating_add(w);
        push_best(&mut best1[u], &mut best2[u], candidate);

        xor_edge[u] ^= edge_id;
        degree[u] -= 1;
        if degree[u] == 1 {
            queue.push_back(u);
        }
    }

    let mut ans = 0_u64;
    for v in 0..n {
        let candidate = best1[v].saturating_add(best2[v]);
        if candidate > ans {
            ans = candidate;
        }
    }
    ans
}

fn farthest_vec(start: usize, adj: &[Vec<(usize, u64)>]) -> (usize, u64) {
    let n = adj.len();
    let mut dist = vec![UNVISITED; n];
    let mut stack = Vec::with_capacity(n);
    dist[start] = 0;
    stack.push(start);
    while let Some(v) = stack.pop() {
        let base = dist[v];
        for &(to, w) in &adj[v] {
            if dist[to] == UNVISITED {
                dist[to] = base.saturating_add(w);
                stack.push(to);
            }
        }
    }
    max_dist(&dist, start)
}

fn farthest_chinese(start: usize, adj: &ChineseAdj) -> (usize, u64) {
    let n = adj.head.len();
    let mut dist = vec![UNVISITED; n];
    let mut stack = Vec::with_capacity(n);
    dist[start] = 0;
    stack.push(start);
    while let Some(v) = stack.pop() {
        let base = dist[v];
        let mut e = adj.head[v];
        while e != NONE {
            let to = adj.to[e];
            if dist[to] == UNVISITED {
                dist[to] = base.saturating_add(adj.weight[e]);
                stack.push(to);
            }
            e = adj.next[e];
        }
    }
    max_dist(&dist, start)
}

fn farthest_csr(start: usize, adj: &Csr) -> (usize, u64) {
    let n = adj.offsets.len().saturating_sub(1);
    let mut dist = vec![UNVISITED; n];
    let mut stack = Vec::with_capacity(n);
    dist[start] = 0;
    stack.push(start);
    while let Some(v) = stack.pop() {
        let base = dist[v];
        let begin = adj.offsets[v];
        let end = adj.offsets[v + 1];
        for idx in begin..end {
            let to = adj.to[idx];
            if dist[to] == UNVISITED {
                dist[to] = base.saturating_add(adj.weight[idx]);
                stack.push(to);
            }
        }
    }
    max_dist(&dist, start)
}

fn max_dist(dist: &[u64], default_node: usize) -> (usize, u64) {
    let mut far_node = default_node;
    let mut far_dist = 0_u64;
    for (i, &d) in dist.iter().enumerate() {
        if d != UNVISITED && d > far_dist {
            far_dist = d;
            far_node = i;
        }
    }
    (far_node, far_dist)
}

fn push_best(best1: &mut u64, best2: &mut u64, value: u64) {
    if value > *best1 {
        *best2 = *best1;
        *best1 = value;
    } else if value > *best2 {
        *best2 = value;
    }
}

struct ChineseAdj {
    head: Vec<usize>,
    to: Vec<usize>,
    next: Vec<usize>,
    weight: Vec<u64>,
}

impl ChineseAdj {
    fn new(n: usize, edges: &[(usize, usize, u64)]) -> Self {
        let m = edges.len();
        let mut head = vec![NONE; n];
        let mut to = Vec::with_capacity(m * 2);
        let mut next = Vec::with_capacity(m * 2);
        let mut weight = Vec::with_capacity(m * 2);

        for &(u, v, w) in edges {
            add_edge(u, v, w, &mut head, &mut to, &mut next, &mut weight);
            add_edge(v, u, w, &mut head, &mut to, &mut next, &mut weight);
        }

        Self {
            head,
            to,
            next,
            weight,
        }
    }
}

fn add_edge(
    from: usize,
    to_node: usize,
    weight_val: u64,
    head: &mut [usize],
    to: &mut Vec<usize>,
    next: &mut Vec<usize>,
    weight: &mut Vec<u64>,
) {
    let idx = to.len();
    to.push(to_node);
    weight.push(weight_val);
    next.push(head[from]);
    head[from] = idx;
}

struct Csr {
    offsets: Vec<usize>,
    to: Vec<usize>,
    weight: Vec<u64>,
}

impl Csr {
    fn new(n: usize, edges: &[(usize, usize, u64)]) -> Self {
        let m = edges.len();
        let mut degree = vec![0_usize; n];
        for &(u, v, _) in edges {
            degree[u] += 1;
            degree[v] += 1;
        }
        let mut offsets = vec![0_usize; n + 1];
        for i in 0..n {
            offsets[i + 1] = offsets[i] + degree[i];
        }
        let mut to = vec![0_usize; m * 2];
        let mut weight = vec![0_u64; m * 2];
        let mut cursor = offsets.clone();
        for &(u, v, w) in edges {
            let pos_u = cursor[u];
            to[pos_u] = v;
            weight[pos_u] = w;
            cursor[u] += 1;

            let pos_v = cursor[v];
            to[pos_v] = u;
            weight[pos_v] = w;
            cursor[v] += 1;
        }
        Self {
            offsets,
            to,
            weight,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{diameter_chinese, diameter_csr, diameter_vec, diameter_xor};
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    fn assert_all_equal(n: usize, edges: &[(usize, usize, u64)], expected: u64) {
        assert_eq!(diameter_vec(n, edges), expected);
        assert_eq!(diameter_chinese(n, edges), expected);
        assert_eq!(diameter_csr(n, edges), expected);
        assert_eq!(diameter_xor(n, edges), expected);
    }

    #[test]
    fn trivial_cases() {
        assert_all_equal(0, &[], 0);
        assert_all_equal(1, &[], 0);
    }

    #[test]
    fn line_tree() {
        let edges = vec![(0, 1, 3), (1, 2, 5), (2, 3, 7)];
        assert_all_equal(4, &edges, 15);
    }

    #[test]
    fn star_tree() {
        let edges = vec![(0, 1, 2), (0, 2, 4), (0, 3, 6)];
        assert_all_equal(4, &edges, 10);
    }

    #[test]
    fn random_tree_matches() {
        let n: usize = 32;
        let mut rng = StdRng::seed_from_u64(0xC0FFEE);
        let mut edges = Vec::with_capacity(n.saturating_sub(1));
        for i in 1..n {
            let parent = rng.random_range(0..i);
            let weight = rng.random_range(1..=1_000_000_000_u64);
            edges.push((i, parent, weight));
        }
        let expected = diameter_vec(n, &edges);
        assert_eq!(diameter_chinese(n, &edges), expected);
        assert_eq!(diameter_csr(n, &edges), expected);
        assert_eq!(diameter_xor(n, &edges), expected);
    }
}
