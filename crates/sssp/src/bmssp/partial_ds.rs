use std::collections::BTreeSet;
use std::collections::VecDeque;

use super::ValueKey;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Entry {
    key: usize,
    value: ValueKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BlockKind {
    D0,
    D1,
}

#[derive(Debug)]
struct Block {
    kind: BlockKind,
    // Lemma 3.3: D1 blocks have a fixed "upper bound" used for search. D0 blocks do not use it.
    upper_bound: ValueKey,
    entries: Vec<Entry>,
}

impl Block {
    fn new(kind: BlockKind, upper_bound: ValueKey, entries: Vec<Entry>) -> Self {
        // D1 is initialized with a single empty block (sentinel). Other blocks should be non-empty.
        debug_assert!(
            kind == BlockKind::D1 || !entries.is_empty(),
            "D0 blocks must be non-empty"
        );
        Self {
            kind,
            upper_bound,
            entries,
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn min_value(&self) -> Option<ValueKey> {
        self.entries.iter().map(|e| e.value).min()
    }
}

#[derive(Clone, Copy, Debug)]
struct KeyLoc {
    block_id: usize,
    idx: usize,
    value: ValueKey,
    kind: BlockKind,
}

impl KeyLoc {
    #[inline]
    fn invalid() -> Self {
        Self {
            block_id: usize::MAX,
            idx: 0,
            value: ValueKey::infinity(),
            kind: BlockKind::D0,
        }
    }
}

/// Data structure from Lemma 3.3 (paper).
///
/// Important paper-faithful details:
/// - D1 maintains *fixed* upper bounds for blocks; deletion does NOT update upper bounds.
/// - `reset()` must be O(1) w.r.t. the key universe size; we use an epoch array for key presence.
/// - `pull_into()` must be O(|S'|) (amortized); do not scan the entire key space.
#[derive(Debug)]
pub(super) struct PartialOrderQueue {
    pull_limit: usize,     // M
    upper_bound: ValueKey, // B
    total_len: usize,
    d0: VecDeque<usize>,
    // (upper_bound, block_id) in increasing upper_bound order.
    d1: BTreeSet<(ValueKey, usize)>,
    blocks: Vec<Block>,

    // Key universe bookkeeping: key is present iff key_epoch[key] == epoch.
    key_epoch: Vec<u32>,
    key_locs: Vec<KeyLoc>,
    epoch: u32,
}

impl PartialOrderQueue {
    pub fn with_capacity(pull_limit: usize, upper_bound: ValueKey, key_space: usize) -> Self {
        let pull_limit = pull_limit.max(1);
        let key_cap = key_space.max(1);
        let mut this = Self {
            pull_limit,
            upper_bound,
            total_len: 0,
            d0: VecDeque::new(),
            d1: BTreeSet::new(),
            blocks: Vec::new(),
            key_epoch: vec![0; key_cap],
            key_locs: vec![KeyLoc::invalid(); key_cap],
            epoch: 1,
        };
        this.reset(upper_bound);
        this
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.total_len == 0
    }

    #[inline]
    fn bump_epoch(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.key_epoch.fill(0);
            self.epoch = 1;
        }
    }

    pub fn reset(&mut self, upper_bound: ValueKey) {
        self.upper_bound = upper_bound;
        self.total_len = 0;
        self.d0.clear();
        self.d1.clear();
        self.blocks.clear();
        self.bump_epoch();

        // Lemma 3.3 Initialize(M,B): D1 starts with one empty sentinel block with upper bound B.
        let sentinel_id = self.blocks.len();
        self.blocks
            .push(Block::new(BlockKind::D1, self.upper_bound, Vec::new()));
        let inserted = self.d1.insert((self.upper_bound, sentinel_id));
        debug_assert!(inserted);
    }

    #[inline]
    fn ensure_key_space(&mut self, key: usize) {
        if key < self.key_epoch.len() {
            return;
        }
        let needed = key + 1;
        let mut new_len = self.key_epoch.len().max(1);
        while new_len < needed {
            new_len = new_len.saturating_mul(2);
        }
        self.key_epoch.resize(new_len, 0);
        self.key_locs.resize(new_len, KeyLoc::invalid());
    }

    #[inline]
    fn is_key_present(&self, key: usize) -> bool {
        key < self.key_epoch.len() && self.key_epoch[key] == self.epoch
    }

    pub fn insert(&mut self, key: usize, value: ValueKey) {
        self.ensure_key_space(key);
        debug_assert!(
            value <= self.upper_bound,
            "Insert value must be <= upper bound"
        );

        if self.is_key_present(key) {
            let loc = self.key_locs[key];
            if value >= loc.value {
                return;
            }
            self.delete_loc(key, loc);
        }

        // Locate the D1 block with the smallest upper bound >= value.
        let chosen = self.d1.range((value, 0)..).next().copied();
        let Some((_, block_id)) = chosen else {
            debug_assert!(false, "D1 must contain a sentinel block with upper bound B");
            return;
        };

        let idx = {
            let block = &mut self.blocks[block_id];
            debug_assert_eq!(block.kind, BlockKind::D1);
            let idx = block.entries.len();
            block.entries.push(Entry { key, value });
            idx
        };

        self.key_epoch[key] = self.epoch;
        self.key_locs[key] = KeyLoc {
            block_id,
            idx,
            value,
            kind: BlockKind::D1,
        };
        self.total_len += 1;

        if self.blocks[block_id].len() > self.pull_limit {
            self.split_d1_block(block_id);
        }
    }

    pub fn batch_prepend_unique(&mut self, values: &[(usize, ValueKey)]) {
        if values.is_empty() {
            return;
        }

        // Lemma 3.3 precondition: each prepended value is smaller than any value currently stored.
        debug_assert!(
            self.current_min_value()
                .map(|m| values.iter().all(|&(_, v)| v < m))
                .unwrap_or(true),
            "BatchPrepend values must be < current minimum"
        );

        for &(key, _) in values {
            self.ensure_key_space(key);
        }

        let mut accepted: Vec<Entry> = Vec::with_capacity(values.len());
        for &(key, value) in values {
            debug_assert!(
                value <= self.upper_bound,
                "BatchPrepend value must be <= upper bound"
            );

            if self.is_key_present(key) {
                let loc = self.key_locs[key];
                // The algorithm only calls BatchPrepend with strictly smaller values, but keep this
                // guard to preserve the lemma contract.
                if value >= loc.value {
                    continue;
                }
                self.delete_loc(key, loc);
            }
            accepted.push(Entry { key, value });
        }
        if accepted.is_empty() {
            return;
        }

        let l = accepted.len();
        if l <= self.pull_limit {
            let block_id = self.allocate_block(BlockKind::D0, ValueKey::infinity(), accepted);
            self.rebuild_key_locs(block_id);
            self.d0.push_front(block_id);
            self.total_len += l;
            return;
        }

        // When L > M, create O(L/M) blocks each of size at most ceil(M/2) via repeated medians.
        let block_cap = self.pull_limit.div_ceil(2);
        let mut segments: Vec<(usize, usize)> = Vec::new();
        partition_by_medians(&mut accepted, block_cap, &mut segments);
        segments.sort_unstable_by_key(|&(l, _)| l);

        for (l, r) in segments.into_iter().rev() {
            let chunk = accepted[l..r].to_vec();
            let block_id = self.allocate_block(BlockKind::D0, ValueKey::infinity(), chunk);
            self.rebuild_key_locs(block_id);
            self.d0.push_front(block_id);
        }
        self.total_len += l;
    }

    pub fn pull_into(&mut self, keys: &mut Vec<usize>) -> Option<ValueKey> {
        keys.clear();
        if self.total_len == 0 {
            return None;
        }

        let take = self.pull_limit.min(self.total_len);
        if take == self.total_len {
            keys.reserve(take);
            for &block_id in &self.d0 {
                let block = &self.blocks[block_id];
                for e in &block.entries {
                    keys.push(e.key);
                }
            }
            for &(_, block_id) in &self.d1 {
                let block = &self.blocks[block_id];
                for e in &block.entries {
                    keys.push(e.key);
                }
            }
            debug_assert_eq!(keys.len(), take);
            self.reset(self.upper_bound);
            return Some(self.upper_bound);
        }

        if take == 1 {
            // A hot-path in BMSSP: level=1 implies M=1, so Pull happens very frequently.
            // Avoid `select_nth_unstable_by` overhead by extracting the minimum directly.
            self.cleanup_d0_front();

            let key = if let Some(&block_id) = self.d0.front() {
                let block = &self.blocks[block_id];
                block
                    .entries
                    .iter()
                    .min_by(|a, b| a.value.cmp(&b.value).then_with(|| a.key.cmp(&b.key)))
                    .expect("non-empty D0 front block")
                    .key
            } else {
                let mut found = None;
                for &(_, block_id) in &self.d1 {
                    let block = &self.blocks[block_id];
                    if block.entries.is_empty() {
                        continue;
                    }
                    found = Some(
                        block
                            .entries
                            .iter()
                            .min_by(|a, b| a.value.cmp(&b.value).then_with(|| a.key.cmp(&b.key)))
                            .expect("non-empty D1 block")
                            .key,
                    );
                    break;
                }
                found.expect("non-empty D1 when total_len > 0")
            };

            keys.push(key);
            debug_assert!(self.is_key_present(key));
            let loc = self.key_locs[key];
            self.delete_loc(key, loc);

            let boundary = self
                .current_min_value()
                .expect("must have remaining values after partial pull");
            return Some(boundary);
        }

        let mut candidates: Vec<Entry> = Vec::with_capacity(take.saturating_mul(4).max(8));

        self.cleanup_d0_front();
        let mut cnt0 = 0usize;
        for &block_id in &self.d0 {
            if cnt0 >= take {
                break;
            }
            let block = &self.blocks[block_id];
            if block.is_empty() {
                continue;
            }
            candidates.extend_from_slice(&block.entries);
            cnt0 += block.len();
        }

        let mut cnt1 = 0usize;
        for &(_, block_id) in &self.d1 {
            if cnt1 >= take {
                break;
            }
            let block = &self.blocks[block_id];
            if block.is_empty() {
                continue;
            }
            candidates.extend_from_slice(&block.entries);
            cnt1 += block.len();
        }

        debug_assert!(
            candidates.len() >= take,
            "prefix union must cover pull size"
        );

        // Identify the smallest `take` values among candidates in O(|candidates|) time.
        let nth = take - 1;
        candidates.select_nth_unstable_by(nth, |a, b| {
            a.value.cmp(&b.value).then_with(|| a.key.cmp(&b.key))
        });

        keys.reserve(take);
        for e in &candidates[..take] {
            keys.push(e.key);
        }

        // Delete pulled keys.
        for &key in keys.iter() {
            if self.is_key_present(key) {
                let loc = self.key_locs[key];
                self.delete_loc(key, loc);
            }
        }

        let boundary = self
            .current_min_value()
            .expect("must have remaining values after partial pull");
        Some(boundary)
    }

    fn cleanup_d0_front(&mut self) {
        while let Some(&block_id) = self.d0.front() {
            if !self.blocks[block_id].is_empty() {
                break;
            }
            self.d0.pop_front();
        }
    }

    fn current_min_value(&mut self) -> Option<ValueKey> {
        if self.total_len == 0 {
            return None;
        }

        self.cleanup_d0_front();
        let d0_min = self.d0.front().and_then(|&id| self.blocks[id].min_value());

        let d1_min = self
            .d1
            .iter()
            .find_map(|&(_, id)| self.blocks[id].min_value());

        match (d0_min, d1_min) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        }
    }

    fn split_d1_block(&mut self, block_id: usize) {
        // Remove old upper bound from the BST and split the block around the median.
        let ub_old = self.blocks[block_id].upper_bound;
        let removed = self.d1.remove(&(ub_old, block_id));
        debug_assert!(removed);

        let mut entries = std::mem::take(&mut self.blocks[block_id].entries);
        let mid = entries.len() / 2;
        entries.select_nth_unstable_by(mid, |a, b| a.value.cmp(&b.value));
        let right_entries = entries.split_off(mid);
        let left_entries = entries;

        // Update left block's upper bound to a tight upper bound so that it is <= any value in the
        // next block.
        let ub_left = left_entries
            .iter()
            .map(|e| e.value)
            .max()
            .expect("left block must be non-empty");

        {
            let block = &mut self.blocks[block_id];
            block.entries = left_entries;
            block.upper_bound = ub_left;
        }
        self.rebuild_key_locs(block_id);

        let right_id = self.allocate_block(BlockKind::D1, ub_old, right_entries);
        self.rebuild_key_locs(right_id);

        let inserted_left = self.d1.insert((ub_left, block_id));
        debug_assert!(inserted_left);
        let inserted_right = self.d1.insert((ub_old, right_id));
        debug_assert!(inserted_right);
    }

    fn delete_loc(&mut self, key: usize, loc: KeyLoc) {
        self.key_epoch[key] = 0;
        self.key_locs[key] = KeyLoc::invalid();
        self.total_len = self.total_len.saturating_sub(1);

        let block_id = loc.block_id;
        let block = &mut self.blocks[block_id];
        debug_assert_eq!(block.kind, loc.kind);

        let removed = block.entries.swap_remove(loc.idx);
        debug_assert_eq!(removed.key, key);
        if loc.idx < block.entries.len() {
            let moved = block.entries[loc.idx];
            self.key_epoch[moved.key] = self.epoch;
            self.key_locs[moved.key] = KeyLoc {
                block_id,
                idx: loc.idx,
                value: moved.value,
                kind: loc.kind,
            };
        }

        if block.kind == BlockKind::D1
            && block.entries.is_empty()
            && block.upper_bound != self.upper_bound
        {
            let _ = self.d1.remove(&(block.upper_bound, block_id));
        }
    }

    fn rebuild_key_locs(&mut self, block_id: usize) {
        let kind = self.blocks[block_id].kind;
        for idx in 0..self.blocks[block_id].entries.len() {
            let e = self.blocks[block_id].entries[idx];
            self.ensure_key_space(e.key);
            self.key_epoch[e.key] = self.epoch;
            self.key_locs[e.key] = KeyLoc {
                block_id,
                idx,
                value: e.value,
                kind,
            };
        }
    }

    fn allocate_block(
        &mut self,
        kind: BlockKind,
        upper_bound: ValueKey,
        entries: Vec<Entry>,
    ) -> usize {
        let id = self.blocks.len();
        self.blocks.push(Block::new(kind, upper_bound, entries));
        id
    }
}

fn partition_by_medians(values: &mut [Entry], block_cap: usize, out: &mut Vec<(usize, usize)>) {
    out.clear();
    let mut stack: Vec<(usize, usize)> = vec![(0, values.len())];
    while let Some((l, r)) = stack.pop() {
        let len = r - l;
        if len <= block_cap {
            out.push((l, r));
            continue;
        }
        let mid = l + len / 2;
        values[l..r].select_nth_unstable_by(mid - l, |a, b| a.value.cmp(&b.value));
        // After partition: [l, mid) <= [mid, r).
        stack.push((mid, r));
        stack.push((l, mid));
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use super::PartialOrderQueue;
    use crate::bmssp::ValueKey;

    #[test]
    fn insert_updates_to_smaller_value() {
        let ub = ValueKey::infinity();
        let mut ds = PartialOrderQueue::with_capacity(4, ub, 16);
        ds.reset(ub);
        ds.insert(3, ValueKey::new(10, 0, 3));
        ds.insert(3, ValueKey::new(9, 0, 3));
        ds.insert(3, ValueKey::new(11, 0, 3)); // ignored
        let mut out = Vec::new();
        let x = ds.pull_into(&mut out).unwrap();
        assert_eq!(x, ub);
        assert_eq!(out, vec![3]);
        assert!(ds.is_empty());
    }

    #[test]
    fn pull_returns_boundary_as_next_min() {
        let ub = ValueKey::infinity();
        let mut ds = PartialOrderQueue::with_capacity(2, ub, 64);
        ds.reset(ub);

        ds.insert(10, ValueKey::new(10, 0, 10));
        ds.insert(11, ValueKey::new(11, 0, 11));
        ds.insert(12, ValueKey::new(12, 0, 12));

        let mut out = Vec::new();
        let x = ds.pull_into(&mut out).unwrap();
        assert_eq!(out.len(), 2);
        // Remaining min is key=12.
        assert_eq!(x, ValueKey::new(12, 0, 12));
    }

    #[test]
    fn batch_prepend_pulls_smallest() {
        let ub = ValueKey::infinity();
        let mut ds = PartialOrderQueue::with_capacity(3, ub, 64);
        ds.reset(ub);

        ds.insert(10, ValueKey::new(100, 0, 10));
        ds.insert(11, ValueKey::new(120, 0, 11));
        ds.insert(12, ValueKey::new(140, 0, 12));

        ds.batch_prepend_unique(&[
            (1, ValueKey::new(1, 0, 1)),
            (2, ValueKey::new(2, 0, 2)),
            (3, ValueKey::new(3, 0, 3)),
        ]);

        let mut out = Vec::new();
        let x = ds.pull_into(&mut out).unwrap();
        assert_eq!(out.len(), 3);
        // Next min among remaining is (100,0,10).
        assert_eq!(x, ValueKey::new(100, 0, 10));
    }

    #[test]
    fn randomized_agrees_with_reference_model() {
        let ub = ValueKey::infinity();
        let m = 8;
        let mut ds = PartialOrderQueue::with_capacity(m, ub, 512);
        ds.reset(ub);

        let mut model: std::collections::BTreeMap<usize, ValueKey> =
            std::collections::BTreeMap::new();
        let mut rng = StdRng::seed_from_u64(0xC0FFEE);

        for _ in 0..5000 {
            let op = rng.random_range(0..3);
            match op {
                0 => {
                    let k = rng.random_range(0..256);
                    let v =
                        ValueKey::new(rng.random_range(0..5000), rng.random_range(0..8), k as u32);
                    ds.insert(k, v);
                    model
                        .entry(k)
                        .and_modify(|cur| *cur = (*cur).min(v))
                        .or_insert(v);
                }
                1 => {
                    let l = rng.random_range(0..=16);
                    if l == 0 {
                        continue;
                    }
                    // Enforce the BatchPrepend precondition: values must be smaller than current minimum.
                    if let Some(cur_min) = model.values().copied().min() {
                        if cur_min.dist == 0 {
                            continue;
                        }
                        let mut batch = Vec::new();
                        for _ in 0..l {
                            let k = rng.random_range(0..256);
                            let v = ValueKey::new(cur_min.dist - 1, 0, k as u32);
                            batch.push((k, v));
                        }
                        // Make unique by key for this test.
                        batch.sort_by_key(|x| x.0);
                        batch.dedup_by_key(|x| x.0);
                        ds.batch_prepend_unique(&batch);
                        for (k, v) in batch {
                            model
                                .entry(k)
                                .and_modify(|cur| *cur = (*cur).min(v))
                                .or_insert(v);
                        }
                    } else {
                        // Empty model: precondition holds vacuously.
                        let mut batch = Vec::new();
                        for _ in 0..l {
                            let k = rng.random_range(0..256);
                            let v = ValueKey::new(rng.random_range(0..50), 0, k as u32);
                            batch.push((k, v));
                        }
                        batch.sort_by_key(|x| x.0);
                        batch.dedup_by_key(|x| x.0);
                        ds.batch_prepend_unique(&batch);
                        for (k, v) in batch {
                            model
                                .entry(k)
                                .and_modify(|cur| *cur = (*cur).min(v))
                                .or_insert(v);
                        }
                    }
                }
                _ => {
                    let mut out = Vec::new();
                    let got = ds.pull_into(&mut out);
                    if model.is_empty() {
                        assert!(got.is_none());
                        continue;
                    }

                    let take = m.min(model.len());
                    let mut items: Vec<(ValueKey, usize)> =
                        model.iter().map(|(&k, &v)| (v, k)).collect();
                    items.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
                    let expected_keys: std::collections::BTreeSet<usize> =
                        items.iter().take(take).map(|&(_, k)| k).collect();
                    for &k in &expected_keys {
                        model.remove(&k);
                    }
                    let expected_x = if model.is_empty() {
                        ub
                    } else {
                        *model.values().min().unwrap()
                    };

                    let got_x = got.unwrap();
                    assert_eq!(got_x, expected_x);
                    let got_keys: std::collections::BTreeSet<usize> = out.into_iter().collect();
                    assert_eq!(got_keys, expected_keys);
                    assert_eq!(ds.is_empty(), model.is_empty());
                }
            }
        }
    }
}
