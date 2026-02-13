use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::INF;
use crate::graph::DirectedGraph;

pub fn dijkstra_binary_heap(graph: &DirectedGraph, source: usize) -> Vec<u64> {
    let n = graph.vertex_count();
    let mut dist = vec![INF; n];
    if source >= n {
        return dist;
    }

    let mut heap = BinaryHeap::new();
    dist[source] = 0;
    heap.push(Reverse((0_u64, source)));

    while let Some(Reverse((d, u))) = heap.pop() {
        if d != dist[u] {
            continue;
        }

        for edge in graph.out_edges(u) {
            let v = edge.to as usize;
            let cand = d.saturating_add(edge.weight).min(INF);
            if cand < dist[v] {
                dist[v] = cand;
                heap.push(Reverse((cand, v)));
            }
        }
    }

    dist
}
