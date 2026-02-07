use std::ops::Range;

use crate::StaticRmq;
use crate::util::better_index;
use crate::util::better_index_ordered;
use crate::util::floor_log2_nonzero;
use crate::util::is_strictly_less;

#[derive(Clone, Debug)]
struct IndexSparseTable {
    n: usize,
    log2: Vec<u8>,
    row_offsets: Vec<usize>,
    table: Vec<usize>,
}

impl IndexSparseTable {
    fn new(values: &[i64], indices: Vec<usize>) -> Self {
        let n = indices.len();

        if n == 0 {
            return Self {
                n,
                log2: vec![0_u8],
                row_offsets: Vec::new(),
                table: Vec::new(),
            };
        }

        let mut log2 = vec![0_u8; n + 1];
        for i in 2..=n {
            log2[i] = log2[i / 2] + 1;
        }

        let levels = (floor_log2_nonzero(n) as usize) + 1;

        let mut total_len = 0_usize;
        for k in 0..levels {
            total_len += n + 1 - (1_usize << k);
        }

        let mut row_offsets = Vec::with_capacity(levels);
        let mut table = Vec::with_capacity(total_len);

        row_offsets.push(0);
        table.extend(indices);

        for k in 1..levels {
            let span = 1_usize << k;
            let half = span >> 1;
            let len = n + 1 - span;

            row_offsets.push(table.len());
            let prev_start = row_offsets[k - 1];
            let prev_ptr = unsafe { table.as_ptr().add(prev_start) };

            for i in 0..len {
                let a = unsafe { *prev_ptr.add(i) };
                let b = unsafe { *prev_ptr.add(i + half) };
                table.push(better_index_ordered(values, a, b));
            }
        }

        Self {
            n,
            log2,
            row_offsets,
            table,
        }
    }

    #[inline(always)]
    fn argmin_assume_valid(&self, values: &[i64], start: usize, end: usize) -> usize {
        debug_assert!(start < end);
        debug_assert!(end <= self.n);

        let len = end - start;
        let k = self.log2[len] as usize;
        let span = 1_usize << k;
        let base = self.row_offsets[k];
        let a = self.table[base + start];
        let b = self.table[base + end - span];
        better_index(values, a, b)
    }
}

#[derive(Clone, Debug)]
pub struct AlstrupRmq {
    values: Vec<i64>,
    block_size: usize,
    l_masks: Vec<u64>,
    block_mins_st: IndexSparseTable,
}

impl AlstrupRmq {
    fn choose_block_size(n: usize) -> usize {
        if n <= 1 {
            return 1;
        }
        let lg = floor_log2_nonzero(n) as usize;
        let m = (lg / 2).max(1);
        m.min(63)
    }

    #[inline(always)]
    fn block_argmin(
        block_start: usize,
        block_len: usize,
        l_masks: &[u64],
        local_l: usize,
        local_r: usize,
    ) -> usize {
        debug_assert!(local_l <= local_r);
        debug_assert!(local_r < block_len);

        let w = l_masks[block_start + local_r] >> local_l;
        let pos = if w == 0 {
            local_r
        } else {
            local_l + (w.trailing_zeros() as usize)
        };

        block_start + pos
    }
}

impl StaticRmq for AlstrupRmq {
    fn new(values: &[i64]) -> Self {
        let n = values.len();
        let values = values.to_vec();
        let block_size = Self::choose_block_size(n);

        if n == 0 {
            return Self {
                values,
                block_size,
                l_masks: Vec::new(),
                block_mins_st: IndexSparseTable::new(&[], Vec::new()),
            };
        }

        let blocks_len = n.div_ceil(block_size);
        let mut l_masks = vec![0_u64; n];
        let mut block_mins = Vec::with_capacity(blocks_len);
        let mut stack: Vec<usize> = Vec::with_capacity(block_size);

        for block_id in 0..blocks_len {
            let start = block_id * block_size;
            let end = (start + block_size).min(n);
            let len = end - start;

            stack.clear();

            for q in 0..len {
                let g = {
                    let global_q = start + q;
                    while let Some(&top) = stack.last() {
                        let global_top = start + top;
                        if is_strictly_less(&values, global_top, global_q) {
                            break;
                        }
                        stack.pop();
                    }
                    stack.last().copied()
                };

                if q > 0
                    && let Some(p) = g
                {
                    let global_q = start + q;
                    let global_p = start + p;
                    l_masks[global_q] = l_masks[global_p] | (1_u64 << p);
                }

                stack.push(q);
            }

            let slice = &values[start..end];
            let mut best_local = 0_usize;
            let mut best_val = slice[0];
            for (i, &v) in slice.iter().enumerate().skip(1) {
                if v < best_val {
                    best_local = i;
                    best_val = v;
                }
            }
            block_mins.push(start + best_local);
        }

        let block_mins_st = IndexSparseTable::new(&values, block_mins);

        Self {
            values,
            block_size,
            l_masks,
            block_mins_st,
        }
    }

    fn argmin(&self, range: Range<usize>) -> Option<usize> {
        let n = self.values.len();
        if range.start >= range.end || range.end > n {
            return None;
        }
        if n == 0 {
            return None;
        }

        let l = range.start;
        let r = range.end - 1;
        let bl = l / self.block_size;
        let br = r / self.block_size;

        if bl == br {
            let block_start = bl * self.block_size;
            let block_end = ((bl + 1) * self.block_size).min(n);
            let block_len = block_end - block_start;
            let local_l = l - block_start;
            let local_r = r - block_start;
            return Some(Self::block_argmin(
                block_start,
                block_len,
                &self.l_masks,
                local_l,
                local_r,
            ));
        }

        let left_start = bl * self.block_size;
        let left_end = ((bl + 1) * self.block_size).min(n);
        let left_len = left_end - left_start;
        let left_local_l = l - left_start;
        let left = Self::block_argmin(
            left_start,
            left_len,
            &self.l_masks,
            left_local_l,
            left_len - 1,
        );

        let right_start = br * self.block_size;
        let right_end = ((br + 1) * self.block_size).min(n);
        let right_len = right_end - right_start;
        let right_local_r = r - right_start;
        let right = Self::block_argmin(right_start, right_len, &self.l_masks, 0, right_local_r);

        let mut ans = better_index_ordered(&self.values, left, right);

        if bl + 1 < br {
            let mid = self
                .block_mins_st
                .argmin_assume_valid(&self.values, bl + 1, br);
            ans = better_index(&self.values, ans, mid);
        }

        Some(ans)
    }
}
