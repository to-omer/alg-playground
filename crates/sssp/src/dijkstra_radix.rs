use crate::INF;
use crate::graph::DirectedGraph;

#[derive(Debug)]
struct RadixHeap {
    buckets: [Vec<(u64, usize)>; 65],
    last: u64,
    len: usize,
}

impl RadixHeap {
    fn new() -> Self {
        Self {
            buckets: std::array::from_fn(|_| Vec::new()),
            last: 0,
            len: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn push(&mut self, key: u64, value: usize) {
        debug_assert!(key >= self.last, "radix-heap key must be monotone");
        let b = bucket_index(self.last, key);
        self.buckets[b].push((key, value));
        self.len += 1;
    }

    fn pop(&mut self) -> Option<(u64, usize)> {
        if self.len == 0 {
            return None;
        }

        if self.buckets[0].is_empty() {
            let mut idx = 1;
            while idx < self.buckets.len() && self.buckets[idx].is_empty() {
                idx += 1;
            }

            if idx == self.buckets.len() {
                return None;
            }

            let mut min_key = u64::MAX;
            for &(key, _) in &self.buckets[idx] {
                min_key = min_key.min(key);
            }
            self.last = min_key;

            let entries = std::mem::take(&mut self.buckets[idx]);
            for (key, value) in entries {
                let b = bucket_index(self.last, key);
                self.buckets[b].push((key, value));
            }
        }

        let pair = self.buckets[0].pop();
        if pair.is_some() {
            self.len -= 1;
        }
        pair
    }
}

#[inline]
fn bucket_index(last: u64, key: u64) -> usize {
    if key == last {
        0
    } else {
        (64 - (key ^ last).leading_zeros()) as usize
    }
}

pub fn dijkstra_radix_heap(graph: &DirectedGraph, source: usize) -> Vec<u64> {
    let n = graph.vertex_count();
    let mut dist = vec![INF; n];
    if source >= n {
        return dist;
    }

    let mut heap = RadixHeap::new();
    dist[source] = 0;
    heap.push(0, source);

    while !heap.is_empty() {
        let Some((d, u)) = heap.pop() else {
            break;
        };
        if d != dist[u] {
            continue;
        }

        for edge in graph.out_edges(u) {
            let v = edge.to as usize;
            let cand = d.saturating_add(edge.weight).min(INF);
            if cand < dist[v] {
                dist[v] = cand;
                heap.push(cand, v);
            }
        }
    }

    dist
}
