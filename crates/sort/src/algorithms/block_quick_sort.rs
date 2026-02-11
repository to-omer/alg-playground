use std::ptr;

use crate::{SortContext, TUNED_PARAMS};

use super::common;

const BLOCK: usize = crate::TUNED_PARAMS.block_partition_size;

pub fn sort(data: &mut [u64], _ctx: &mut SortContext) {
    block_quick_sort(data);
}

fn block_quick_sort(mut data: &mut [u64]) {
    while data.len() > TUNED_PARAMS.insertion_threshold {
        let pivot = common::choose_pivot_ninther(data);
        let split = block_partition(data, pivot);

        if split == 0 || split == data.len() {
            let (lt, gt) = common::partition_3way(data, pivot);
            if lt == 0 && gt == data.len() {
                return;
            }

            let (left, rest) = data.split_at_mut(lt);
            let (_, right) = rest.split_at_mut(gt - lt);
            if left.len() < right.len() {
                block_quick_sort(left);
                data = right;
            } else {
                block_quick_sort(right);
                data = left;
            }
            continue;
        }

        let (left, right) = data.split_at_mut(split);
        if left.len() < right.len() {
            block_quick_sort(left);
            data = right;
        } else {
            block_quick_sort(right);
            data = left;
        }
    }

    common::insertion_sort(data);
}

fn block_partition(data: &mut [u64], pivot: u64) -> usize {
    let len = data.len();
    if len <= 1 {
        return len;
    }

    let ptr = data.as_mut_ptr();
    let mut left = 0usize;
    let mut right = len;

    let mut left_offsets = [0usize; BLOCK];
    let mut right_offsets = [0usize; BLOCK];
    let mut left_count = 0usize;
    let mut right_count = 0usize;
    let mut left_pos = 0usize;
    let mut right_pos = 0usize;

    unsafe {
        while right - left > BLOCK * 2 {
            if left_pos == left_count {
                left_pos = 0;
                left_count = 0;
                for i in 0..BLOCK {
                    if *ptr.add(left + i) >= pivot {
                        left_offsets[left_count] = i;
                        left_count += 1;
                    }
                }
                if left_count == 0 {
                    left += BLOCK;
                    continue;
                }
            }

            if right_pos == right_count {
                right_pos = 0;
                right_count = 0;
                for i in 0..BLOCK {
                    let idx = right - 1 - i;
                    if *ptr.add(idx) < pivot {
                        right_offsets[right_count] = i;
                        right_count += 1;
                    }
                }
                if right_count == 0 {
                    right -= BLOCK;
                    continue;
                }
            }

            let swaps = (left_count - left_pos).min(right_count - right_pos);
            for _ in 0..swaps {
                let li = left + left_offsets[left_pos];
                let ri = right - 1 - right_offsets[right_pos];
                ptr::swap(ptr.add(li), ptr.add(ri));
                left_pos += 1;
                right_pos += 1;
            }

            if left_pos == left_count {
                left += BLOCK;
            }
            if right_pos == right_count {
                right -= BLOCK;
            }
        }

        let mut i = left;
        let mut j = right;
        while i < j {
            while i < j && *ptr.add(i) < pivot {
                i += 1;
            }
            while i < j && *ptr.add(j - 1) >= pivot {
                j -= 1;
            }
            if i < j {
                ptr::swap(ptr.add(i), ptr.add(j - 1));
                i += 1;
                j -= 1;
            }
        }

        i
    }
}
