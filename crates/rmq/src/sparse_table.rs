use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::Range;

use crate::StaticRmq;
use crate::util::better_index;
use crate::util::better_index_ordered;
use crate::util::floor_log2_nonzero;

#[derive(Clone, Debug)]
enum SparseTable {
    U32(Vec<u32>),
    Usize(Vec<usize>),
}

#[derive(Clone, Debug)]
pub struct SparseTableRmq {
    values: Vec<i64>,
    row_offsets: Vec<usize>,
    table: SparseTable,
}

impl SparseTableRmq {
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl StaticRmq for SparseTableRmq {
    fn new(values: &[i64]) -> Self {
        let n = values.len();
        let values = values.to_vec();

        if n == 0 {
            return Self {
                values,
                row_offsets: Vec::new(),
                table: SparseTable::U32(Vec::new()),
            };
        }

        let levels = (floor_log2_nonzero(n) as usize) + 1;

        let mut total_len = 0_usize;
        for k in 0..levels {
            total_len += n + 1 - (1_usize << k);
        }

        let mut row_offsets = Vec::with_capacity(levels);
        let mut offset = 0_usize;
        for k in 0..levels {
            row_offsets.push(offset);
            offset += n + 1 - (1_usize << k);
        }
        debug_assert_eq!(offset, total_len);

        let table = if n <= (u32::MAX as usize) {
            let mut table = Vec::<MaybeUninit<u32>>::with_capacity(total_len);
            // We fully initialize the buffer before converting it to `Vec<u32>`.
            unsafe {
                table.set_len(total_len);
            }
            let ptr = table.as_mut_ptr();
            for i in 0..n {
                unsafe {
                    ptr.add(i).write(MaybeUninit::new(i as u32));
                }
            }

            for k in 1..levels {
                let span = 1_usize << k;
                let half = span >> 1;
                let len = n + 1 - span;

                let base = row_offsets[k];
                let prev_base = row_offsets[k - 1];

                for i in 0..len {
                    let a = unsafe { (*ptr.add(prev_base + i)).assume_init() as usize };
                    let b = unsafe { (*ptr.add(prev_base + i + half)).assume_init() as usize };
                    let best = better_index_ordered(&values, a, b);
                    unsafe {
                        ptr.add(base + i).write(MaybeUninit::new(best as u32));
                    }
                }
            }

            let mut table = ManuallyDrop::new(table);
            let ptr = table.as_mut_ptr() as *mut u32;
            let len = table.len();
            let cap = table.capacity();
            let table = unsafe { Vec::from_raw_parts(ptr, len, cap) };
            SparseTable::U32(table)
        } else {
            let mut table = Vec::<MaybeUninit<usize>>::with_capacity(total_len);
            // We fully initialize the buffer before converting it to `Vec<usize>`.
            unsafe {
                table.set_len(total_len);
            }
            let ptr = table.as_mut_ptr();
            for i in 0..n {
                unsafe {
                    ptr.add(i).write(MaybeUninit::new(i));
                }
            }

            for k in 1..levels {
                let span = 1_usize << k;
                let half = span >> 1;
                let len = n + 1 - span;

                let base = row_offsets[k];
                let prev_base = row_offsets[k - 1];

                for i in 0..len {
                    let a = unsafe { (*ptr.add(prev_base + i)).assume_init() };
                    let b = unsafe { (*ptr.add(prev_base + i + half)).assume_init() };
                    unsafe {
                        ptr.add(base + i)
                            .write(MaybeUninit::new(better_index_ordered(&values, a, b)));
                    }
                }
            }

            let mut table = ManuallyDrop::new(table);
            let ptr = table.as_mut_ptr() as *mut usize;
            let len = table.len();
            let cap = table.capacity();
            let table = unsafe { Vec::from_raw_parts(ptr, len, cap) };
            SparseTable::Usize(table)
        };

        Self {
            values,
            row_offsets,
            table,
        }
    }

    fn argmin(&self, range: Range<usize>) -> Option<usize> {
        let n = self.values.len();
        if range.start >= range.end || range.end > n {
            return None;
        }
        let len = range.end - range.start;
        if len == 1 {
            return Some(range.start);
        }

        let k = floor_log2_nonzero(len) as usize;
        let span = 1_usize << k;
        let base = self.row_offsets[k];
        match &self.table {
            SparseTable::U32(table) => {
                let a = table[base + range.start] as usize;
                let b = table[base + range.end - span] as usize;
                Some(better_index(&self.values, a, b))
            }
            SparseTable::Usize(table) => {
                let a = table[base + range.start];
                let b = table[base + range.end - span];
                Some(better_index(&self.values, a, b))
            }
        }
    }
}
