mod bmssp;
mod constant_degree;
mod dijkstra_binary;
mod dijkstra_radix;
pub mod generator;
pub mod graph;

pub use bmssp::bmssp_paper;
pub use dijkstra_binary::dijkstra_binary_heap;
pub use dijkstra_radix::dijkstra_radix_heap;
pub use graph::DirectedGraph;
pub use graph::Edge;

pub const INF: u64 = u64::MAX / 4;

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rand::Rng;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use crate::bmssp_paper;
    use crate::constant_degree::transform_to_constant_degree;
    use crate::dijkstra_binary_heap;
    use crate::dijkstra_radix_heap;
    use crate::generator::GraphCase;
    use crate::generator::generate_case;
    use crate::graph::DirectedGraph;

    fn random_graph(n: usize, m: usize, seed: u64) -> DirectedGraph {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut used = HashSet::new();
        let mut edges = Vec::with_capacity(m);

        while edges.len() < m {
            let u = rng.random_range(0..n);
            let v = rng.random_range(0..n);
            if u == v {
                continue;
            }
            let key = ((u as u64) << 32) | v as u64;
            if used.insert(key) {
                edges.push((u as u32, v as u32, rng.random_range(0..=1_000_000_u64)));
            }
        }

        DirectedGraph::from_edges(n, &edges)
    }

    #[test]
    fn dijkstra_radix_matches_binary_random() {
        for seed in 0..20_u64 {
            let n = 64;
            let m = 512;
            let g = random_graph(n, m, 0xD1A1_0000 + seed);
            let src = (seed as usize) % n;
            let d0 = dijkstra_binary_heap(&g, src);
            let d1 = dijkstra_radix_heap(&g, src);
            assert_eq!(d0, d1, "seed={seed}");
        }
    }

    #[test]
    fn bmssp_matches_binary_random_small() {
        for seed in 0..30_u64 {
            let n = 40;
            let m = 220;
            let g = random_graph(n, m, 0xB0A5_0000 + seed);
            let src = (seed as usize) % n;
            let expected = dijkstra_binary_heap(&g, src);
            let got = bmssp_paper(&g, src);
            assert_eq!(got, expected, "seed={seed}");
        }
    }

    #[test]
    fn zero_and_unreachable_cases() {
        let g = DirectedGraph::from_edges(6, &[(0, 1, 0), (1, 2, 0), (2, 3, 0), (4, 5, 7)]);
        let expected = dijkstra_binary_heap(&g, 0);
        let got = bmssp_paper(&g, 0);
        assert_eq!(got, expected);
    }

    #[test]
    fn constant_degree_transform_preserves_distances() {
        for seed in 0..12_u64 {
            let g = random_graph(50, 300, 0xC0DE_0000 + seed);
            let src = (seed as usize) % 50;
            let transformed = transform_to_constant_degree(&g, src);
            let transformed_dist = dijkstra_binary_heap(transformed.graph(), transformed.source);
            let projected = transformed.project_distances(&transformed_dist);
            assert_eq!(projected, dijkstra_binary_heap(&g, src));
        }
    }

    #[test]
    fn constant_degree_transform_seed_23_regression() {
        let seed = 23_u64;
        let n = 40;
        let m = 220;
        let g = random_graph(n, m, 0xB0A5_0000 + seed);
        let src = (seed as usize) % n;
        let transformed = transform_to_constant_degree(&g, src);
        let transformed_dist = dijkstra_binary_heap(transformed.graph(), transformed.source);
        let projected = transformed.project_distances(&transformed_dist);
        assert_eq!(projected, dijkstra_binary_heap(&g, src));
    }

    #[test]
    fn generator_smoke_and_agreement() {
        let cases = [
            GraphCase::SparseRandom,
            GraphCase::MaxSparseRandom,
            GraphCase::MaxDenseRandom,
            GraphCase::MaxDenseLong,
            GraphCase::MaxDenseZero,
            GraphCase::AlmostLine,
            GraphCase::GridRandom,
            GraphCase::GridSwirl,
            GraphCase::WrongDijkstraKiller,
            GraphCase::SpfaKiller,
        ];

        for (i, case) in cases.iter().enumerate() {
            let input = generate_case(*case, 1_024, 0x5EED_0000 + i as u64);
            assert!(input.graph.vertex_count() >= 2, "case={:?}", case);
            let d0 = dijkstra_binary_heap(&input.graph, input.source);
            let d1 = dijkstra_radix_heap(&input.graph, input.source);
            let d2 = bmssp_paper(&input.graph, input.source);
            assert_eq!(d0, d1, "case={:?}", case);
            assert_eq!(d0, d2, "case={:?}", case);
        }
    }
}
