use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::Range;

use crate::StaticRmq;
use crate::util::better_index_ordered;
use crate::util::floor_log2_nonzero;

#[derive(Clone, Debug)]
enum DstTable {
    U16(Vec<u16>),
    U32(Vec<u32>),
    Usize(Vec<usize>),
}

#[derive(Clone, Debug)]
pub struct DisjointSparseTableRmq {
    values: Vec<i64>,
    n: usize,
    levels: usize,
    table: DstTable,
}

impl DisjointSparseTableRmq {
    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }
}

impl StaticRmq for DisjointSparseTableRmq {
    fn new(values: &[i64]) -> Self {
        let n = values.len();
        let values = values.to_vec();
        if n == 0 {
            return Self {
                values,
                n,
                levels: 0,
                table: DstTable::U32(Vec::new()),
            };
        }
        if n == 1 {
            return Self {
                values,
                n,
                levels: 0,
                table: DstTable::U32(Vec::new()),
            };
        }

        let levels = (floor_log2_nonzero(n - 1) as usize) + 1;
        let table = if n <= (u16::MAX as usize) + 1 {
            let mut table = Vec::<MaybeUninit<u16>>::with_capacity(levels * n);
            unsafe {
                table.set_len(levels * n);
            }
            for level in 0..levels {
                let base = level * n;
                let level_table = &mut table[base..(base + n)];
                let seg_len = 1_usize << (level + 1);
                let half = seg_len >> 1;

                for block_start in (0..n).step_by(seg_len) {
                    let block_end = (block_start + seg_len).min(n);
                    let mid = (block_start + half).min(block_end);

                    if mid > block_start {
                        level_table[mid - 1].write((mid - 1) as u16);
                        for i in (block_start..(mid - 1)).rev() {
                            let right = unsafe { level_table[i + 1].assume_init() } as usize;
                            let best = better_index_ordered(&values, i, right);
                            level_table[i].write(best as u16);
                        }
                    }
                    if mid < block_end {
                        level_table[mid].write(mid as u16);
                        for i in (mid + 1)..block_end {
                            let left = unsafe { level_table[i - 1].assume_init() } as usize;
                            let best = better_index_ordered(&values, left, i);
                            level_table[i].write(best as u16);
                        }
                    }
                }
            }
            let mut table = ManuallyDrop::new(table);
            let ptr = table.as_mut_ptr() as *mut u16;
            let len = table.len();
            let cap = table.capacity();
            let table = unsafe { Vec::from_raw_parts(ptr, len, cap) };
            DstTable::U16(table)
        } else if n <= (u32::MAX as usize) {
            let mut table = Vec::<MaybeUninit<u32>>::with_capacity(levels * n);
            unsafe {
                table.set_len(levels * n);
            }
            for level in 0..levels {
                let base = level * n;
                let level_table = &mut table[base..(base + n)];
                let seg_len = 1_usize << (level + 1);
                let half = seg_len >> 1;

                for block_start in (0..n).step_by(seg_len) {
                    let block_end = (block_start + seg_len).min(n);
                    let mid = (block_start + half).min(block_end);

                    if mid > block_start {
                        level_table[mid - 1].write((mid - 1) as u32);
                        for i in (block_start..(mid - 1)).rev() {
                            let right = unsafe { level_table[i + 1].assume_init() } as usize;
                            let best = better_index_ordered(&values, i, right);
                            level_table[i].write(best as u32);
                        }
                    }
                    if mid < block_end {
                        level_table[mid].write(mid as u32);
                        for i in (mid + 1)..block_end {
                            let left = unsafe { level_table[i - 1].assume_init() } as usize;
                            let best = better_index_ordered(&values, left, i);
                            level_table[i].write(best as u32);
                        }
                    }
                }
            }
            let mut table = ManuallyDrop::new(table);
            let ptr = table.as_mut_ptr() as *mut u32;
            let len = table.len();
            let cap = table.capacity();
            let table = unsafe { Vec::from_raw_parts(ptr, len, cap) };
            DstTable::U32(table)
        } else {
            let mut table = Vec::<MaybeUninit<usize>>::with_capacity(levels * n);
            unsafe {
                table.set_len(levels * n);
            }
            for level in 0..levels {
                let base = level * n;
                let level_table = &mut table[base..(base + n)];
                let seg_len = 1_usize << (level + 1);
                let half = seg_len >> 1;

                for block_start in (0..n).step_by(seg_len) {
                    let block_end = (block_start + seg_len).min(n);
                    let mid = (block_start + half).min(block_end);

                    if mid > block_start {
                        level_table[mid - 1].write(mid - 1);
                        for i in (block_start..(mid - 1)).rev() {
                            let right = unsafe { level_table[i + 1].assume_init() };
                            level_table[i].write(better_index_ordered(&values, i, right));
                        }
                    }
                    if mid < block_end {
                        level_table[mid].write(mid);
                        for i in (mid + 1)..block_end {
                            let left = unsafe { level_table[i - 1].assume_init() };
                            level_table[i].write(better_index_ordered(&values, left, i));
                        }
                    }
                }
            }
            let mut table = ManuallyDrop::new(table);
            let ptr = table.as_mut_ptr() as *mut usize;
            let len = table.len();
            let cap = table.capacity();
            let table = unsafe { Vec::from_raw_parts(ptr, len, cap) };
            DstTable::Usize(table)
        };

        Self {
            values,
            n,
            levels,
            table,
        }
    }

    fn argmin(&self, range: Range<usize>) -> Option<usize> {
        let n = self.n;
        if range.start >= range.end || range.end > n {
            return None;
        }

        let l = range.start;
        let r = range.end - 1;
        if l == r {
            return Some(l);
        }

        let x = l ^ r;
        let level = (usize::BITS - 1 - x.leading_zeros()) as usize;
        debug_assert!(level < self.levels);
        let base = level * n;
        match &self.table {
            DstTable::U16(table) => {
                let a = table[base + l] as usize;
                let b = table[base + r] as usize;
                Some(better_index_ordered(&self.values, a, b))
            }
            DstTable::U32(table) => {
                let a = table[base + l] as usize;
                let b = table[base + r] as usize;
                Some(better_index_ordered(&self.values, a, b))
            }
            DstTable::Usize(table) => {
                let a = table[base + l];
                let b = table[base + r];
                Some(better_index_ordered(&self.values, a, b))
            }
        }
    }
}
