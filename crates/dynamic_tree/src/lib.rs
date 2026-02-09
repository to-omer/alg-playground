pub mod policy;
pub mod traits;

mod ett;
mod lct;
mod top_tree;

pub use ett::EulerTourTree;
pub use lct::LinkCutTree;
pub use top_tree::TopTree;

pub use traits::{ComponentOps, DynamicForest, PathOps, SubtreeOps, VertexOps};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{Affine, LazyMapMonoid, PathComposite, VertexAffineSum, VertexSumAdd};
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use std::collections::VecDeque;

    fn add_undirected_edge(g: &mut [Vec<usize>], u: usize, v: usize) {
        g[u].push(v);
        g[v].push(u);
    }

    fn remove_undirected_edge(g: &mut [Vec<usize>], u: usize, v: usize) {
        fn erase_one(vec: &mut Vec<usize>, x: usize) {
            if let Some(pos) = vec.iter().position(|&y| y == x) {
                vec.swap_remove(pos);
            } else {
                panic!("edge not found");
            }
        }
        erase_one(&mut g[u], v);
        erase_one(&mut g[v], u);
    }

    fn bfs_connected(g: &[Vec<usize>], s: usize, t: usize) -> bool {
        if s == t {
            return true;
        }
        let n = g.len();
        let mut q = VecDeque::new();
        let mut vis = vec![false; n];
        vis[s] = true;
        q.push_back(s);
        while let Some(v) = q.pop_front() {
            for &to in &g[v] {
                if vis[to] {
                    continue;
                }
                if to == t {
                    return true;
                }
                vis[to] = true;
                q.push_back(to);
            }
        }
        false
    }

    fn bfs_path(g: &[Vec<usize>], s: usize, t: usize) -> Option<Vec<usize>> {
        let n = g.len();
        let mut par = vec![usize::MAX; n];
        let mut q = VecDeque::new();
        par[s] = s;
        q.push_back(s);
        while let Some(v) = q.pop_front() {
            if v == t {
                break;
            }
            for &to in &g[v] {
                if par[to] != usize::MAX {
                    continue;
                }
                par[to] = v;
                q.push_back(to);
            }
        }
        if par[t] == usize::MAX {
            return None;
        }
        let mut path = Vec::new();
        let mut cur = t;
        while cur != s {
            path.push(cur);
            cur = par[cur];
        }
        path.push(s);
        path.reverse();
        Some(path)
    }

    fn bfs_component_vertices(g: &[Vec<usize>], s: usize) -> Vec<usize> {
        let n = g.len();
        let mut q = VecDeque::new();
        let mut vis = vec![false; n];
        vis[s] = true;
        q.push_back(s);
        let mut verts = Vec::new();
        while let Some(v) = q.pop_front() {
            verts.push(v);
            for &to in &g[v] {
                if vis[to] {
                    continue;
                }
                vis[to] = true;
                q.push_back(to);
            }
        }
        verts
    }

    fn bfs_component_sum(g: &[Vec<usize>], values: &[i64], s: usize) -> i64 {
        bfs_component_vertices(g, s)
            .into_iter()
            .map(|v| values[v])
            .sum()
    }

    fn edge_key(u: usize, v: usize) -> (usize, usize) {
        if u < v { (u, v) } else { (v, u) }
    }

    fn policy_apply_agg_naive<P: LazyMapMonoid<Key = i64, Agg = i64>>(
        xs: &[i64],
        act: P::Act,
    ) -> i64 {
        xs.iter().map(|&x| P::act_apply_key(&x, &act)).sum::<i64>()
    }

    #[test]
    fn policy_apply_agg_matches_naive_sum_add_and_affine() {
        let mut rng = StdRng::seed_from_u64(0xBADC0FFE_u64);
        for len in 0..=10 {
            let xs = (0..len)
                .map(|_| rng.random_range(-50_i64..=50))
                .collect::<Vec<_>>();
            let sum = xs.iter().copied().sum::<i64>();

            // SumAdd
            for _ in 0..50 {
                let delta = rng.random_range(-20_i64..=20);
                let got = VertexSumAdd::act_apply_agg(&sum, &delta, len);
                let expected = policy_apply_agg_naive::<VertexSumAdd>(&xs, delta);
                assert_eq!(got, expected);
            }

            // Affine
            for _ in 0..50 {
                let a = rng.random_range(-3_i64..=3);
                let b = rng.random_range(-10_i64..=10);
                let act = Affine { a, b };
                let got = VertexAffineSum::act_apply_agg(&sum, &act, len);
                let expected = policy_apply_agg_naive::<VertexAffineSum>(&xs, act);
                assert_eq!(got, expected);
            }
        }
    }

    #[test]
    fn lct_random_against_bfs_with_extra_ops() {
        let mut rng = StdRng::seed_from_u64(0xC0FFEE_u64);
        let n = 30_usize;
        let steps = 20_000_usize;

        let mut values = (0..n)
            .map(|_| rng.random_range(-500_i64..=500))
            .collect::<Vec<_>>();
        let mut lct = LinkCutTree::<VertexSumAdd>::new(&values);
        let mut g = vec![Vec::<usize>::new(); n];
        let mut edges = Vec::<(usize, usize)>::new();

        for it in 0..steps {
            let op = rng.random_range(0..8);
            match op {
                0 => {
                    // link
                    let u = rng.random_range(0..n);
                    let v = rng.random_range(0..n);
                    if u == v || bfs_connected(&g, u, v) {
                        continue;
                    }
                    assert!(lct.link(u, v));
                    add_undirected_edge(&mut g, u, v);
                    edges.push(edge_key(u, v));
                }
                1 => {
                    // cut
                    if edges.is_empty() {
                        continue;
                    }
                    let idx = rng.random_range(0..edges.len());
                    let (u, v) = edges.swap_remove(idx);
                    assert!(lct.cut(u, v));
                    remove_undirected_edge(&mut g, u, v);
                }
                2 => {
                    // path fold
                    let u = rng.random_range(0..n);
                    let v = rng.random_range(0..n);
                    let Some(path) = bfs_path(&g, u, v) else {
                        continue;
                    };
                    let expected = path.into_iter().map(|x| values[x]).sum::<i64>();
                    let got = lct.path_fold(u, v).unwrap();
                    assert_eq!(got, expected, "it={it} path_fold({u},{v})");
                }
                3 => {
                    // path apply
                    let u = rng.random_range(0..n);
                    let v = rng.random_range(0..n);
                    let Some(path) = bfs_path(&g, u, v) else {
                        continue;
                    };
                    let delta = rng.random_range(-10_i64..=10);
                    assert!(lct.path_apply(u, v, delta));
                    for x in path {
                        values[x] += delta;
                    }
                }
                4 => {
                    // vertex set
                    let v = rng.random_range(0..n);
                    let key = rng.random_range(-500_i64..=500);
                    lct.vertex_set(v, key);
                    values[v] = key;
                }
                5 => {
                    // vertex apply (add)
                    let v = rng.random_range(0..n);
                    let delta = rng.random_range(-10_i64..=10);
                    lct.vertex_apply(v, delta);
                    values[v] += delta;
                }
                6 => {
                    // path kth
                    let u = rng.random_range(0..n);
                    let v = rng.random_range(0..n);
                    let Some(path) = bfs_path(&g, u, v) else {
                        continue;
                    };
                    let k = rng.random_range(0..path.len());
                    let got = lct.path_kth(u, v, k).unwrap();
                    assert_eq!(got, path[k], "it={it} path_kth({u},{v},{k})");
                }
                _ => {
                    // connected + makeroot/find_root sanity
                    let u = rng.random_range(0..n);
                    let v = rng.random_range(0..n);
                    let expected = bfs_connected(&g, u, v);
                    assert_eq!(lct.connected(u, v), expected, "it={it} connected({u},{v})");

                    let r = rng.random_range(0..n);
                    lct.makeroot(r);
                    assert_eq!(lct.find_root(r), r);
                }
            }
        }
    }

    #[test]
    fn ett_random_against_bfs_with_component_and_subtree_ops() {
        let mut rng = StdRng::seed_from_u64(0xE771_2026_u64);
        let n = 50_usize;
        let steps = 8_000_usize;

        let mut values = (0..n)
            .map(|_| rng.random_range(-1_000_i64..=1_000))
            .collect::<Vec<_>>();
        let mut ett = EulerTourTree::<VertexSumAdd>::new(&values);
        let mut g = vec![Vec::<usize>::new(); n];
        let mut edges = Vec::<(usize, usize)>::new();

        for it in 0..steps {
            let op = rng.random_range(0..9);
            match op {
                0 => {
                    let u = rng.random_range(0..n);
                    let v = rng.random_range(0..n);
                    if u == v || bfs_connected(&g, u, v) {
                        continue;
                    }
                    assert!(ett.link(u, v));
                    add_undirected_edge(&mut g, u, v);
                    edges.push(edge_key(u, v));
                }
                1 => {
                    if edges.is_empty() {
                        continue;
                    }
                    let idx = rng.random_range(0..edges.len());
                    let (u, v) = edges.swap_remove(idx);
                    assert!(ett.cut(u, v));
                    remove_undirected_edge(&mut g, u, v);
                }
                2 => {
                    let v = rng.random_range(0..n);
                    let expected = bfs_component_sum(&g, &values, v);
                    let got = ett.component_fold(v);
                    assert_eq!(got, expected, "it={it} component_fold({v})");
                }
                3 => {
                    let v = rng.random_range(0..n);
                    let delta = rng.random_range(-10_i64..=10);
                    ett.component_apply(v, delta);
                    for x in bfs_component_vertices(&g, v) {
                        values[x] += delta;
                    }
                }
                4 => {
                    let v = rng.random_range(0..n);
                    let key = rng.random_range(-1_000_i64..=1_000);
                    ett.vertex_set(v, key);
                    values[v] = key;
                }
                5 => {
                    let v = rng.random_range(0..n);
                    let delta = rng.random_range(-10_i64..=10);
                    ett.vertex_apply(v, delta);
                    values[v] += delta;
                }
                6 => {
                    // component_size
                    let v = rng.random_range(0..n);
                    let expected = bfs_component_vertices(&g, v).len();
                    let got = ett.component_size(v);
                    assert_eq!(got, expected, "it={it} component_size({v})");
                }
                7 => {
                    // subtree_fold
                    if edges.is_empty() {
                        continue;
                    }
                    let &(a, b) = &edges[rng.random_range(0..edges.len())];
                    let (child, parent) = if rng.random_bool(0.5) { (a, b) } else { (b, a) };

                    remove_undirected_edge(&mut g, child, parent);
                    let expected = bfs_component_sum(&g, &values, child);
                    add_undirected_edge(&mut g, child, parent);

                    let got = ett.subtree_fold(child, parent);
                    assert_eq!(got, expected, "it={it} subtree_fold({child},{parent})");
                }
                _ => {
                    // subtree_apply + connected check
                    if edges.is_empty() {
                        continue;
                    }
                    let &(a, b) = &edges[rng.random_range(0..edges.len())];
                    let (child, parent) = if rng.random_bool(0.5) { (a, b) } else { (b, a) };
                    let delta = rng.random_range(-10_i64..=10);

                    remove_undirected_edge(&mut g, child, parent);
                    for x in bfs_component_vertices(&g, child) {
                        values[x] += delta;
                    }
                    add_undirected_edge(&mut g, child, parent);

                    ett.subtree_apply(child, parent, delta);

                    let u = rng.random_range(0..n);
                    let v = rng.random_range(0..n);
                    assert_eq!(ett.connected(u, v), bfs_connected(&g, u, v));
                }
            }
        }
    }

    #[test]
    fn top_tree_random_against_bfs_with_ops() {
        let mut rng = StdRng::seed_from_u64(0x7A7A_2026_u64);
        let n = 30_usize;
        let steps = 8_000_usize;

        let mut values = (0..n)
            .map(|_| rng.random_range(-500_i64..=500))
            .collect::<Vec<_>>();
        let mut tt = TopTree::<VertexSumAdd>::new(&values);
        let mut g = vec![Vec::<usize>::new(); n];
        let mut edges = Vec::<(usize, usize)>::new();

        for it in 0..steps {
            let op = rng.random_range(0..11);
            match op {
                0 => {
                    let u = rng.random_range(0..n);
                    let v = rng.random_range(0..n);
                    if u == v || bfs_connected(&g, u, v) {
                        continue;
                    }
                    assert!(tt.link(u, v));
                    add_undirected_edge(&mut g, u, v);
                    edges.push(edge_key(u, v));
                }
                1 => {
                    if edges.is_empty() {
                        continue;
                    }
                    let idx = rng.random_range(0..edges.len());
                    let (u, v) = edges.swap_remove(idx);
                    assert!(tt.cut(u, v));
                    remove_undirected_edge(&mut g, u, v);
                }
                2 => {
                    let u = rng.random_range(0..n);
                    let v = rng.random_range(0..n);
                    let Some(path) = bfs_path(&g, u, v) else {
                        continue;
                    };
                    let expected = path.into_iter().map(|x| values[x]).sum::<i64>();
                    let got = tt.path_fold(u, v).unwrap();
                    assert_eq!(got, expected, "it={it} path_fold({u},{v})");
                }
                3 => {
                    let v = rng.random_range(0..n);
                    let expected = bfs_component_sum(&g, &values, v);
                    let got = tt.component_fold(v);
                    assert_eq!(got, expected, "it={it} component_fold({v})");
                }
                4 => {
                    let u = rng.random_range(0..n);
                    let v = rng.random_range(0..n);
                    let Some(path) = bfs_path(&g, u, v) else {
                        continue;
                    };
                    let delta = rng.random_range(-10_i64..=10);
                    assert!(tt.path_apply(u, v, delta));
                    for x in path {
                        values[x] += delta;
                    }
                }
                5 => {
                    let v = rng.random_range(0..n);
                    let delta = rng.random_range(-10_i64..=10);
                    tt.component_apply(v, delta);
                    for x in bfs_component_vertices(&g, v) {
                        values[x] += delta;
                    }
                }
                6 => {
                    let v = rng.random_range(0..n);
                    let key = rng.random_range(-500_i64..=500);
                    tt.vertex_set(v, key);
                    values[v] = key;
                }
                7 => {
                    let v = rng.random_range(0..n);
                    let delta = rng.random_range(-10_i64..=10);
                    tt.vertex_apply(v, delta);
                    values[v] += delta;
                }
                8 => {
                    let v = rng.random_range(0..n);
                    let expected = bfs_component_vertices(&g, v).len();
                    let got = tt.component_size(v);
                    assert_eq!(got, expected, "it={it} component_size({v})");
                }
                9 => {
                    // subtree_fold
                    if edges.is_empty() {
                        continue;
                    }
                    let &(a, b) = &edges[rng.random_range(0..edges.len())];
                    let (child, parent) = if rng.random_bool(0.5) { (a, b) } else { (b, a) };

                    remove_undirected_edge(&mut g, child, parent);
                    let expected = bfs_component_sum(&g, &values, child);
                    add_undirected_edge(&mut g, child, parent);

                    let got = tt.subtree_fold(child, parent);
                    assert_eq!(got, expected, "it={it} subtree_fold({child},{parent})");
                }
                _ => {
                    // subtree_apply + path_kth + connected
                    if !edges.is_empty() && rng.random_bool(0.5) {
                        let &(a, b) = &edges[rng.random_range(0..edges.len())];
                        let (child, parent) = if rng.random_bool(0.5) { (a, b) } else { (b, a) };
                        let delta = rng.random_range(-10_i64..=10);

                        remove_undirected_edge(&mut g, child, parent);
                        for x in bfs_component_vertices(&g, child) {
                            values[x] += delta;
                        }
                        add_undirected_edge(&mut g, child, parent);

                        tt.subtree_apply(child, parent, delta);
                    } else {
                        let u = rng.random_range(0..n);
                        let v = rng.random_range(0..n);
                        if let Some(path) = bfs_path(&g, u, v) {
                            let k = rng.random_range(0..path.len());
                            let got = tt.path_kth(u, v, k).unwrap();
                            assert_eq!(got, path[k], "it={it} path_kth({u},{v},{k})");
                        }
                    }

                    let u = rng.random_range(0..n);
                    let v = rng.random_range(0..n);
                    assert_eq!(tt.connected(u, v), bfs_connected(&g, u, v));
                }
            }
        }
    }

    fn compose_affine(f: (i64, i64), g: (i64, i64)) -> (i64, i64) {
        (
            f.0.wrapping_mul(g.0),
            f.0.wrapping_mul(g.1).wrapping_add(f.1),
        )
    }

    #[test]
    fn path_composite_agrees_between_lct_and_top_tree() {
        let values = vec![(1_i64, 1_i64), (2, 0), (1, -3)]; // x+1, 2x, x-3
        let mut lct = LinkCutTree::<PathComposite>::new(&values);
        let mut tt = TopTree::<PathComposite>::new(&values);
        assert!(lct.link(0, 1));
        assert!(lct.link(1, 2));
        assert!(tt.link(0, 1));
        assert!(tt.link(1, 2));

        let got_lct = lct.path_fold(0, 2).unwrap();
        let got_tt = tt.path_fold(0, 2).unwrap();
        let expected = compose_affine(values[2], compose_affine(values[1], values[0]));
        assert_eq!(got_lct, expected);
        assert_eq!(got_tt, expected);
    }

    #[test]
    fn top_tree_edge_ops_affect_path_and_component_fold() {
        let values = vec![5_i64, 7];
        let mut tt = TopTree::<VertexSumAdd>::new(&values);
        assert!(tt.link_with_edge(0, 1, 3));
        assert_eq!(tt.path_sum(0, 1), Some(15));
        assert_eq!(tt.component_sum(0), 15);

        assert_eq!(tt.edge_get(0, 1), Some(3));
        assert!(tt.edge_set(0, 1, 10));
        assert_eq!(tt.path_sum(0, 1), Some(22));

        assert!(tt.edge_apply(0, 1, -2));
        assert_eq!(tt.edge_get(0, 1), Some(8));
        assert_eq!(tt.component_sum(0), 20);
    }

    #[test]
    fn lct_path_sum_disconnected_returns_none() {
        let values = vec![1_i64, 2, 3];
        let mut lct = LinkCutTree::<VertexSumAdd>::new(&values);
        assert!(lct.link(0, 1));
        assert_eq!(lct.path_sum(0, 2), None);
        assert_eq!(lct.path_sum(1, 2), None);
        assert_eq!(lct.path_sum(2, 2), Some(3));
    }
}
