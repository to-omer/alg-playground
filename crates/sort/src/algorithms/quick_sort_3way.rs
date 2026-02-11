use crate::{SortContext, TUNED_PARAMS};

use super::common;

pub fn sort(data: &mut [u64], _ctx: &mut SortContext) {
    quick_sort_3way(data);
}

fn quick_sort_3way(mut data: &mut [u64]) {
    while data.len() > TUNED_PARAMS.insertion_threshold {
        let pivot = common::choose_pivot_ninther(data);
        let (lt, gt) = common::partition_3way(data, pivot);

        if lt == 0 && gt == data.len() {
            return;
        }

        let (left, rest) = data.split_at_mut(lt);
        let (_, right) = rest.split_at_mut(gt - lt);

        if left.len() < right.len() {
            quick_sort_3way(left);
            data = right;
        } else {
            quick_sort_3way(right);
            data = left;
        }
    }

    common::insertion_sort(data);
}
