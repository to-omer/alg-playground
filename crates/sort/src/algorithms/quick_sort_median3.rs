use crate::{SortContext, TUNED_PARAMS};

use super::common;

pub fn sort(data: &mut [u64], _ctx: &mut SortContext) {
    quick_sort(data);
}

pub fn quick_sort(data: &mut [u64]) {
    quick_sort_recursive(data);
}

fn quick_sort_recursive(mut data: &mut [u64]) {
    while data.len() > TUNED_PARAMS.insertion_threshold {
        let len = data.len();
        let pivot = if data.len() >= 2048 {
            common::choose_pivot_ninther(data)
        } else {
            common::choose_pivot_median3(data)
        };
        let split = common::partition_hoare(data, pivot) + 1;
        if split == 0 || split == len {
            let (lt, gt) = common::partition_3way(data, pivot);
            if lt == 0 && gt == len {
                return;
            }
            let (left, rest) = data.split_at_mut(lt);
            let (_, right) = rest.split_at_mut(gt - lt);
            if left.len() < right.len() {
                quick_sort_recursive(left);
                data = right;
            } else {
                quick_sort_recursive(right);
                data = left;
            }
            continue;
        }

        let (left, right) = data.split_at_mut(split);

        if left.len() < right.len() {
            quick_sort_recursive(left);
            data = right;
        } else {
            quick_sort_recursive(right);
            data = left;
        }
    }

    common::insertion_sort(data);
}
