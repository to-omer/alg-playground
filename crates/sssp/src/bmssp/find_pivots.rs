use super::BmsspEngine;
use super::ValueKey;

pub(super) fn find_pivots(
    engine: &mut BmsspEngine<'_>,
    expect_b: ValueKey,
    s: &[usize],
) -> (Vec<usize>, Vec<usize>) {
    if s.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let source_epoch = engine.next_epoch();
    for &x in s {
        engine.mark_index[x] = source_epoch;
    }

    let w_epoch = engine.next_epoch();
    let mut w = Vec::with_capacity(s.len().saturating_mul(2).max(1));
    engine.scratch_parent.clear();
    for &x in s {
        if engine.mark_w[x] != w_epoch {
            engine.mark_w[x] = w_epoch;
            engine.index_pos[x] = w.len();
            w.push(x);
            engine.scratch_parent.push(usize::MAX);
        }
    }

    let mut w_prev = w.clone();
    let mut w_next = Vec::new();
    let limit = engine.k.saturating_mul(s.len());

    for _ in 0..engine.k {
        let next_epoch = engine.next_epoch();
        w_next.clear();
        w_next.reserve(w_prev.len().saturating_mul(2));

        for &u in &w_prev {
            let cur = engine.labels[u];
            let hops = cur.hops.saturating_add(1);
            let (to, weight) = engine.graph.out_edge_slices(u);
            for i in 0..to.len() {
                let v = to[i] as usize;
                let dist = cur.dist.saturating_add(weight[i]).min(crate::INF);
                let candidate = super::Label {
                    dist,
                    hops,
                    pred: u as u32,
                };
                let current = engine.labels[v];
                let relaxed = if candidate < current {
                    engine.labels[v] = candidate;
                    true
                } else {
                    candidate == current
                };
                if relaxed {
                    let candidate_value = ValueKey::new(dist, hops, v as u32);
                    if candidate_value < expect_b && engine.mark_next[v] != next_epoch {
                        engine.mark_next[v] = next_epoch;
                        w_next.push(v);
                    }
                    if candidate_value < expect_b && engine.mark_w[v] != w_epoch {
                        engine.mark_w[v] = w_epoch;
                        engine.index_pos[v] = w.len();
                        w.push(v);
                    }
                }
            }
        }

        if w.len() > limit {
            return (s.to_vec(), w);
        }

        if w_next.is_empty() {
            break;
        }
        std::mem::swap(&mut w_prev, &mut w_next);
    }

    engine.scratch_parent.clear();
    engine.scratch_parent.resize(w.len(), usize::MAX);
    engine.scratch_parent_label.clear();
    engine
        .scratch_parent_label
        .resize(w.len(), super::Label::infinity());

    let mut has_invalid_parent = false;
    for &v in &w {
        let idx = engine.index_pos[v];
        let pred = engine.labels[v].pred as usize;
        if pred >= engine.graph.vertex_count() || engine.mark_w[pred] != w_epoch {
            has_invalid_parent = true;
            break;
        }
        let pred_idx = engine.index_pos[pred];
        engine.scratch_parent[idx] = pred_idx;
        engine.scratch_parent_label[idx] = engine.labels[v];
    }

    let mut pivots = Vec::new();
    if has_invalid_parent {
        pivots.extend_from_slice(s);
    } else {
        engine.scratch_pending.clear();
        engine.scratch_pending.resize(w.len(), 0);
        for &p in &engine.scratch_parent {
            if p != usize::MAX {
                engine.scratch_pending[p] += 1;
            }
        }

        engine.scratch_sizes.clear();
        engine.scratch_sizes.resize(w.len(), 1);
        engine.scratch_stack.clear();
        engine.scratch_stack.reserve(w.len());
        for (i, &cnt) in engine.scratch_pending.iter().enumerate() {
            if cnt == 0 {
                engine.scratch_stack.push(i);
            }
        }

        let mut processed = 0usize;
        while let Some(i) = engine.scratch_stack.pop() {
            processed += 1;
            let p = engine.scratch_parent[i];
            if p != usize::MAX {
                engine.scratch_sizes[p] += engine.scratch_sizes[i];
                engine.scratch_pending[p] -= 1;
                if engine.scratch_pending[p] == 0 {
                    engine.scratch_stack.push(p);
                }
            }
        }

        if processed != w.len() {
            pivots.extend_from_slice(s);
        } else {
            for &u in s {
                if engine.mark_w[u] != w_epoch {
                    continue;
                }
                if engine.mark_index[u] != source_epoch {
                    continue;
                }
                let i = engine.index_pos[u];
                if engine.scratch_parent[i] == usize::MAX
                    && engine.scratch_sizes[i] >= engine.k.max(1)
                {
                    pivots.push(u);
                }
            }
        }
    }

    (pivots, w)
}
