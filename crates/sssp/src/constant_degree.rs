use std::ops::Range;

use crate::INF;
use crate::graph::DirectedGraph;

/// Paper (arXiv:2504.17033v2) constant-degree transformation:
/// - Replace each original vertex v by a directed zero-weight cycle.
/// - Create one cycle vertex for each incident edge (outgoing + incoming). (This is a safe
///   refinement of "one per neighbor" that preserves the in/out-degree <= 2 guarantee even
///   with parallel edges.)
/// - For each original edge (u -> v, w), add an edge from u's outgoing slot to v's incoming
///   slot with weight w.
///
/// Then every transformed vertex has in-degree and out-degree at most 2.
#[derive(Debug)]
pub struct ConstantDegreeGraph {
    graph: DirectedGraph,
    pub source: usize,
    vertex_ranges: Vec<Range<usize>>,
}

impl ConstantDegreeGraph {
    #[inline]
    pub fn graph(&self) -> &DirectedGraph {
        &self.graph
    }

    pub fn project_distances(&self, transformed_dist: &[u64]) -> Vec<u64> {
        let mut dist = vec![INF; self.vertex_ranges.len()];
        for (v, range) in self.vertex_ranges.iter().enumerate() {
            let mut best = INF;
            for idx in range.clone() {
                best = best.min(transformed_dist[idx]);
            }
            dist[v] = best;
        }
        dist
    }
}

pub fn transform_to_constant_degree(graph: &DirectedGraph, source: usize) -> ConstantDegreeGraph {
    let n = graph.vertex_count();

    let mut out_deg = vec![0_usize; n];
    let mut in_deg = vec![0_usize; n];
    for (u, out) in out_deg.iter_mut().enumerate() {
        *out = graph.out_degree(u);
        let (to, _) = graph.out_edge_slices(u);
        for &v in to {
            in_deg[v as usize] += 1;
        }
    }

    let mut vertex_ranges = Vec::with_capacity(n);
    let mut total_nodes = 0_usize;
    for v in 0..n {
        let slots = (out_deg[v] + in_deg[v]).max(1);
        let start = total_nodes;
        total_nodes += slots;
        vertex_ranges.push(start..total_nodes);
    }

    let mut transformed_edges = Vec::with_capacity(
        graph
            .edge_count()
            .saturating_add(total_nodes.saturating_sub(n).max(1)),
    );

    // Zero-weight directed cycles.
    for range in &vertex_ranges {
        let len = range.end - range.start;
        if len <= 1 {
            continue;
        }
        for i in 0..len {
            let from = (range.start + i) as u32;
            let to = (range.start + ((i + 1) % len)) as u32;
            transformed_edges.push((from, to, 0_u64));
        }
    }

    // Map each original edge to a unique outgoing slot of u and a unique incoming slot of v.
    let mut out_seen = vec![0_usize; n];
    let mut in_seen = vec![0_usize; n];
    for u in 0..n {
        let (to, weight) = graph.out_edge_slices(u);
        for i in 0..to.len() {
            let v = to[i] as usize;
            let tail = vertex_ranges[u].start + out_seen[u];
            out_seen[u] += 1;

            let head = vertex_ranges[v].start + out_deg[v] + in_seen[v];
            in_seen[v] += 1;

            transformed_edges.push((tail as u32, head as u32, weight[i]));
        }
    }

    let transformed_graph = DirectedGraph::from_edges(total_nodes, &transformed_edges);
    let source_node = if source < n {
        vertex_ranges[source].start
    } else {
        0
    };

    ConstantDegreeGraph {
        graph: transformed_graph,
        source: source_node,
        vertex_ranges,
    }
}

#[cfg(test)]
mod tests {
    use super::transform_to_constant_degree;
    use crate::dijkstra_binary_heap;
    use crate::graph::DirectedGraph;

    #[test]
    fn transform_preserves_distances_for_small_graph() {
        let g = DirectedGraph::from_edges(
            5,
            &[
                (0, 1, 3),
                (0, 2, 2),
                (2, 1, 1),
                (1, 3, 7),
                (2, 3, 4),
                (3, 4, 5),
            ],
        );
        let original = dijkstra_binary_heap(&g, 0);

        let transformed = transform_to_constant_degree(&g, 0);
        let projected = transformed.project_distances(&dijkstra_binary_heap(
            transformed.graph(),
            transformed.source,
        ));

        assert_eq!(projected, original);
    }

    #[test]
    fn transformed_graph_has_degree_at_most_two() {
        let n = 80;
        let mut edges = Vec::new();
        for u in 0..n {
            for v in 0..n {
                if u != v && (u + v) % 17 == 0 {
                    edges.push((u as u32, v as u32, ((u * 7 + v) % 19) as u64));
                }
            }
        }
        let g = DirectedGraph::from_edges(n, &edges);
        let transformed = transform_to_constant_degree(&g, 0);

        let tg = transformed.graph();
        let mut max_out = 0_usize;
        let mut in_deg = vec![0_usize; tg.vertex_count()];
        for u in 0..tg.vertex_count() {
            let (to, _) = tg.out_edge_slices(u);
            max_out = max_out.max(to.len());
            for &v in to {
                in_deg[v as usize] += 1;
            }
        }
        let max_in = in_deg.into_iter().max().unwrap_or(0);
        assert!(max_out <= 2, "max_out={max_out}");
        assert!(max_in <= 2, "max_in={max_in}");
    }
}
