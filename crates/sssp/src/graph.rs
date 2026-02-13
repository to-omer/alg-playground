#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Edge {
    pub to: u32,
    pub weight: u64,
}

#[derive(Clone, Debug)]
pub struct DirectedGraph {
    vertex_count: usize,
    offsets: Vec<usize>,
    to: Vec<u32>,
    weight: Vec<u64>,
}

impl DirectedGraph {
    pub fn new(vertex_count: usize) -> Self {
        Self {
            vertex_count,
            offsets: vec![0; vertex_count + 1],
            to: Vec::new(),
            weight: Vec::new(),
        }
    }

    pub fn from_edges(vertex_count: usize, edges: &[(u32, u32, u64)]) -> Self {
        let mut out_deg = vec![0_usize; vertex_count];
        for &(from, to, _) in edges {
            assert!((from as usize) < vertex_count, "from vertex out of range");
            assert!((to as usize) < vertex_count, "to vertex out of range");
            out_deg[from as usize] += 1;
        }

        let mut offsets = vec![0_usize; vertex_count + 1];
        for v in 0..vertex_count {
            offsets[v + 1] = offsets[v] + out_deg[v];
        }

        let mut to = vec![0_u32; edges.len()];
        let mut weight = vec![0_u64; edges.len()];
        let mut cursor = offsets[..vertex_count].to_vec();

        for &(from, dst, w) in edges {
            let idx = cursor[from as usize];
            cursor[from as usize] += 1;
            to[idx] = dst;
            weight[idx] = w;
        }

        Self {
            vertex_count,
            offsets,
            to,
            weight,
        }
    }

    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    #[inline]
    pub fn edge_count(&self) -> usize {
        self.to.len()
    }

    #[inline]
    pub fn out_degree(&self, v: usize) -> usize {
        let start = self.offsets[v];
        let end = self.offsets[v + 1];
        end - start
    }

    #[inline]
    pub fn out_edges(&self, v: usize) -> OutEdges<'_> {
        let start = self.offsets[v];
        let end = self.offsets[v + 1];
        OutEdges {
            to: &self.to[start..end],
            weight: &self.weight[start..end],
            idx: 0,
        }
    }

    #[inline]
    pub fn out_edge_slices(&self, v: usize) -> (&[u32], &[u64]) {
        let start = self.offsets[v];
        let end = self.offsets[v + 1];
        (&self.to[start..end], &self.weight[start..end])
    }

    pub fn edges_vec(&self) -> Vec<(u32, u32, u64)> {
        let mut edges = Vec::with_capacity(self.edge_count());
        for u in 0..self.vertex_count {
            for edge in self.out_edges(u) {
                edges.push((u as u32, edge.to, edge.weight));
            }
        }
        edges
    }
}

pub struct OutEdges<'a> {
    to: &'a [u32],
    weight: &'a [u64],
    idx: usize,
}

impl<'a> Iterator for OutEdges<'a> {
    type Item = Edge;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.to.len() {
            return None;
        }
        let edge = Edge {
            to: self.to[self.idx],
            weight: self.weight[self.idx],
        };
        self.idx += 1;
        Some(edge)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remain = self.to.len() - self.idx;
        (remain, Some(remain))
    }
}

impl ExactSizeIterator for OutEdges<'_> {}
