use crate::{SortContext, TUNED_PARAMS};

use super::{common, heap_sort};

pub fn sort(data: &mut [u64], _ctx: &mut SortContext) {
    if data.len() < 2 {
        return;
    }
    let depth_limit = common::introsort_depth_limit(data.len()) + 1;
    introsort_recursive(data, depth_limit);
}

fn introsort_recursive(mut data: &mut [u64], mut depth_limit: usize) {
    while data.len() > TUNED_PARAMS.insertion_threshold {
        if depth_limit == 0 {
            heap_sort::heap_sort(data);
            return;
        }
        depth_limit -= 1;

        let pivot = common::choose_pivot_ninther(data);
        let split = common::partition_hoare(data, pivot);
        if split == 0 || split + 1 == data.len() {
            let (lt, gt) = common::partition_3way(data, pivot);
            if lt == 0 && gt == data.len() {
                return;
            }
            let (left, rest) = data.split_at_mut(lt);
            let (_, right) = rest.split_at_mut(gt - lt);
            if left.len() < right.len() {
                introsort_recursive(left, depth_limit);
                data = right;
            } else {
                introsort_recursive(right, depth_limit);
                data = left;
            }
            continue;
        }
        let (left, right) = data.split_at_mut(split + 1);

        if left.len() < right.len() {
            introsort_recursive(left, depth_limit);
            data = right;
        } else {
            introsort_recursive(right, depth_limit);
            data = left;
        }
    }

    common::insertion_sort(data);
}
