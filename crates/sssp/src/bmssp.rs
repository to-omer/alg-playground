mod find_pivots;
mod partial_ds;

use std::cmp::Ordering;

use crate::INF;
use crate::constant_degree::transform_to_constant_degree;
use crate::graph::DirectedGraph;

use partial_ds::PartialOrderQueue;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ValueKey {
    dist: u64,
    hops: u32,
    vertex: u32,
}

impl ValueKey {
    pub(crate) fn new(dist: u64, hops: u32, vertex: u32) -> Self {
        Self { dist, hops, vertex }
    }

    pub(crate) fn infinity() -> Self {
        Self {
            dist: INF,
            hops: u32::MAX,
            vertex: u32::MAX,
        }
    }
}

impl Ord for ValueKey {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.dist, self.hops, self.vertex).cmp(&(other.dist, other.hops, other.vertex))
    }
}

impl PartialOrd for ValueKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct Label {
    dist: u64,
    hops: u32,
    pred: u32,
}

impl Label {
    fn infinity() -> Self {
        Self {
            dist: INF,
            hops: u32::MAX,
            pred: u32::MAX,
        }
    }
}

pub(crate) struct BmsspEngine<'a> {
    pub(crate) graph: &'a DirectedGraph,
    source: usize,
    pub(crate) labels: Vec<Label>,
    pub(crate) mark_w: Vec<u32>,
    pub(crate) mark_next: Vec<u32>,
    pub(crate) mark_index: Vec<u32>,
    pub(crate) index_pos: Vec<usize>,
    pub(crate) scratch_parent: Vec<usize>,
    pub(crate) scratch_parent_label: Vec<Label>,
    pub(crate) scratch_pending: Vec<usize>,
    pub(crate) scratch_sizes: Vec<usize>,
    pub(crate) scratch_stack: Vec<usize>,
    partial_queues: Vec<Option<PartialOrderQueue>>,
    base_queue: Vec<(ValueKey, usize)>,
    base_u0: Vec<usize>,
    insert_best: Vec<ValueKey>,
    prepend_best: Vec<ValueKey>,
    mark_actual: Vec<u64>,
    epoch: u32,
    pub(crate) k: usize,
    t: usize,
    level_max: usize,
}

impl<'a> BmsspEngine<'a> {
    fn new(graph: &'a DirectedGraph, source: usize) -> Self {
        let n = graph.vertex_count();
        let lg = (n.max(2) as f64).log2();
        let mut k = lg.powf(1.0 / 3.0).floor() as usize;
        let mut t = lg.powf(2.0 / 3.0).floor() as usize;
        k = k.max(1);
        t = t.max(1);
        let level_max = (lg / t as f64).ceil() as usize;

        let mut labels = vec![Label::infinity(); n];
        if source < n {
            labels[source] = Label {
                dist: 0,
                hops: 0,
                pred: source as u32,
            };
        }

        Self {
            graph,
            source,
            labels,
            mark_w: vec![0; n],
            mark_next: vec![0; n],
            mark_index: vec![0; n],
            index_pos: vec![0; n],
            scratch_parent: Vec::new(),
            scratch_parent_label: Vec::new(),
            scratch_pending: Vec::new(),
            scratch_sizes: Vec::new(),
            scratch_stack: Vec::new(),
            partial_queues: (0..=level_max).map(|_| None).collect(),
            base_queue: Vec::with_capacity(k.saturating_add(4)),
            base_u0: Vec::with_capacity(k.saturating_add(1)),
            insert_best: vec![ValueKey::infinity(); n],
            prepend_best: vec![ValueKey::infinity(); n],
            mark_actual: vec![0; n],
            epoch: 0,
            k,
            t,
            level_max,
        }
    }

    fn solve(&mut self) {
        if self.source >= self.graph.vertex_count() {
            return;
        }
        let source_set = [self.source];
        let _ = self.bmssp(self.level_max, ValueKey::infinity(), &source_set, 0);
    }

    fn distances(&self) -> Vec<u64> {
        self.labels.iter().map(|label| label.dist).collect()
    }

    pub(crate) fn value_of(&self, vertex: usize) -> ValueKey {
        let label = self.labels[vertex];
        ValueKey::new(label.dist, label.hops, vertex as u32)
    }

    #[inline]
    pub(crate) fn next_epoch(&mut self) -> u32 {
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.mark_w.fill(0);
            self.mark_next.fill(0);
            self.mark_index.fill(0);
            self.epoch = 1;
        }
        self.epoch
    }

    fn pull_size(&self, level: usize) -> usize {
        if level == 0 {
            return 1;
        }
        let exp = (level - 1).saturating_mul(self.t);
        pow2_saturating(exp)
            .max(1)
            .min(self.graph.vertex_count().max(1))
    }

    fn partial_threshold(&self, level: usize) -> usize {
        let exp = level.saturating_mul(self.t);
        self.k
            .saturating_mul(pow2_saturating(exp))
            .max(1)
            .min(self.graph.vertex_count().max(1))
    }

    fn base_case(&mut self, expect_b: ValueKey, s: &[usize]) -> (ValueKey, Vec<usize>) {
        if s.is_empty() {
            return (expect_b, Vec::new());
        }
        debug_assert_eq!(s.len(), 1, "base_case expects singleton source set");

        let x = s[0];
        let mut queue = std::mem::take(&mut self.base_queue);
        queue.clear();
        let mut u0 = std::mem::take(&mut self.base_u0);
        u0.clear();
        let seen_epoch = self.next_epoch();
        let queued_epoch = self.next_epoch();
        let source_value = self.value_of(x);
        self.mark_next[x] = queued_epoch;
        self.insert_best[x] = source_value;
        queue.push((source_value, x));

        while let Some((value, u)) = pop_min_value_key(&mut queue) {
            if value != self.value_of(u) {
                continue;
            }
            if self.mark_next[u] == queued_epoch {
                self.mark_next[u] = 0;
            }
            if self.mark_w[u] == seen_epoch {
                continue;
            }
            self.mark_w[u] = seen_epoch;
            u0.push(u);

            let cur = self.labels[u];
            let hops = cur.hops.saturating_add(1);
            let (to, weight) = self.graph.out_edge_slices(u);
            for i in 0..to.len() {
                let v = to[i] as usize;
                let dist = cur.dist.saturating_add(weight[i]).min(INF);
                let candidate = Label {
                    dist,
                    hops,
                    pred: u as u32,
                };
                let candidate_value = ValueKey::new(dist, hops, v as u32);
                // Paper (Alg. 2, line 10): relax only when the new value is strictly below B.
                if candidate_value >= expect_b {
                    continue;
                }
                let current = self.labels[v];
                let relaxed = if candidate < current {
                    self.labels[v] = candidate;
                    true
                } else {
                    candidate == current
                };
                if relaxed
                    && self.mark_w[v] != seen_epoch
                    && (self.mark_next[v] != queued_epoch || candidate_value < self.insert_best[v])
                {
                    self.mark_next[v] = queued_epoch;
                    self.insert_best[v] = candidate_value;
                    queue.push((candidate_value, v));
                }
            }

            if u0.len() > self.k {
                break;
            }
        }

        if u0.len() <= self.k {
            let ret_u = u0.clone();
            self.base_queue = queue;
            self.base_u0 = u0;
            return (expect_b, ret_u);
        }

        let actual_b = u0
            .iter()
            .map(|&v| self.value_of(v))
            .max()
            .unwrap_or(expect_b);

        let mut actual_u = Vec::with_capacity(u0.len());
        for &v in &u0 {
            if self.value_of(v) < actual_b {
                actual_u.push(v);
            }
        }
        self.base_queue = queue;
        self.base_u0 = u0;
        (actual_b, actual_u)
    }

    fn bmssp(
        &mut self,
        level: usize,
        expect_b: ValueKey,
        s: &[usize],
        depth: usize,
    ) -> (ValueKey, Vec<usize>) {
        if s.is_empty() {
            return (expect_b, Vec::new());
        }

        if level == 0 {
            return self.base_case(expect_b, s);
        }
        debug_assert!(depth < 64, "bmssp recursion depth overflow");

        let (pivots, visited) = find_pivots::find_pivots(self, expect_b, s);
        if pivots.is_empty() {
            let mut actual_u = Vec::with_capacity(visited.len());
            for &x in &visited {
                if self.value_of(x) < expect_b {
                    actual_u.push(x);
                }
            }
            return (expect_b, actual_u);
        }

        let threshold = self.partial_threshold(level);
        let pull_limit = self.pull_size(level);
        let key_space_hint = self.graph.vertex_count().max(1);
        let mut ds = self.partial_queues[level].take().unwrap_or_else(|| {
            PartialOrderQueue::with_capacity(pull_limit, expect_b, key_space_hint)
        });
        ds.reset(expect_b);

        for &x in &pivots {
            ds.insert(x, self.value_of(x));
        }

        let dedup_bit = 1_u64 << depth;
        let mut actual_u = Vec::new();
        let mut actual_b = expect_b;
        let mut prepend_records = Vec::new();
        let mut insert_keys = Vec::new();
        let mut prepend_keys = Vec::new();
        let mut pulled_keys = Vec::with_capacity(pull_limit);
        let mut newly_completed = Vec::new();

        while actual_u.len() < threshold && !ds.is_empty() {
            let Some(expect_b_i) = ds.pull_into(&mut pulled_keys) else {
                break;
            };
            if pulled_keys.is_empty() {
                break;
            }

            let (actual_b_i, u_i) =
                self.bmssp(level - 1, expect_b_i, pulled_keys.as_slice(), depth + 1);
            actual_b = actual_b_i;

            newly_completed.clear();
            newly_completed.reserve(u_i.len());
            for &u in &u_i {
                if self.mark_actual[u] & dedup_bit == 0 {
                    self.mark_actual[u] |= dedup_bit;
                    actual_u.push(u);
                    newly_completed.push(u);
                }
            }

            let insert_epoch = self.next_epoch();
            let prepend_epoch = self.next_epoch();
            insert_keys.clear();
            prepend_keys.clear();
            for &u in &newly_completed {
                let cur = self.labels[u];
                let hops = cur.hops.saturating_add(1);
                let (to, weight) = self.graph.out_edge_slices(u);
                for i in 0..to.len() {
                    let v = to[i] as usize;
                    let dist = cur.dist.saturating_add(weight[i]).min(INF);
                    let candidate = Label {
                        dist,
                        hops,
                        pred: u as u32,
                    };
                    let current = self.labels[v];
                    let relaxed = if candidate < current {
                        self.labels[v] = candidate;
                        true
                    } else {
                        candidate == current
                    };
                    if relaxed {
                        let candidate_value = ValueKey::new(dist, hops, v as u32);
                        if candidate_value >= expect_b_i && candidate_value < expect_b {
                            if self.mark_w[v] != insert_epoch {
                                self.mark_w[v] = insert_epoch;
                                self.insert_best[v] = candidate_value;
                                insert_keys.push(v);
                            } else if candidate_value < self.insert_best[v] {
                                self.insert_best[v] = candidate_value;
                            }
                        } else if candidate_value >= actual_b_i && candidate_value < expect_b_i {
                            if self.mark_next[v] != prepend_epoch {
                                self.mark_next[v] = prepend_epoch;
                                self.prepend_best[v] = candidate_value;
                                prepend_keys.push(v);
                            } else if candidate_value < self.prepend_best[v] {
                                self.prepend_best[v] = candidate_value;
                            }
                        }
                    }
                }
            }

            for &x in &pulled_keys {
                let value = self.value_of(x);
                if value >= actual_b_i && value < expect_b_i {
                    if self.mark_next[x] != prepend_epoch {
                        self.mark_next[x] = prepend_epoch;
                        self.prepend_best[x] = value;
                        prepend_keys.push(x);
                    } else if value < self.prepend_best[x] {
                        self.prepend_best[x] = value;
                    }
                }
            }

            for &v in &insert_keys {
                ds.insert(v, self.insert_best[v]);
            }

            prepend_records.clear();
            prepend_records.reserve(prepend_keys.len());
            for &v in &prepend_keys {
                prepend_records.push((v, self.prepend_best[v]));
            }
            ds.batch_prepend_unique(&prepend_records);
        }

        if ds.is_empty() {
            actual_b = expect_b;
        }
        for &x in &visited {
            if self.value_of(x) < actual_b && self.mark_actual[x] & dedup_bit == 0 {
                self.mark_actual[x] |= dedup_bit;
                actual_u.push(x);
            }
        }

        for &u in &actual_u {
            self.mark_actual[u] &= !dedup_bit;
        }

        self.partial_queues[level] = Some(ds);
        (actual_b, actual_u)
    }
}

#[inline]
fn pow2_saturating(exp: usize) -> usize {
    if exp >= usize::BITS as usize {
        usize::MAX
    } else {
        1_usize << exp
    }
}

#[inline]
fn pop_min_value_key(queue: &mut Vec<(ValueKey, usize)>) -> Option<(ValueKey, usize)> {
    if queue.is_empty() {
        return None;
    }
    let mut min_idx = 0usize;
    for i in 1..queue.len() {
        if queue[i].0 < queue[min_idx].0 {
            min_idx = i;
        }
    }
    Some(queue.swap_remove(min_idx))
}

pub fn bmssp_paper(graph: &DirectedGraph, source: usize) -> Vec<u64> {
    if graph.vertex_count() == 0 {
        return Vec::new();
    }
    if source >= graph.vertex_count() {
        return vec![INF; graph.vertex_count()];
    }
    if graph.out_degree(source) == 0 {
        let mut dist = vec![INF; graph.vertex_count()];
        dist[source] = 0;
        return dist;
    }

    let transformed = transform_to_constant_degree(graph, source);
    let mut engine = BmsspEngine::new(transformed.graph(), transformed.source);
    engine.solve();
    transformed.project_distances(&engine.distances())
}
