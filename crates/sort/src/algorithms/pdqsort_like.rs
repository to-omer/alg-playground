use crate::{SortContext, TUNED_PARAMS};

use super::{common, heap_sort};

pub fn sort(data: &mut [u64], _ctx: &mut SortContext) {
    if data.len() < 2 {
        return;
    }
    let bad_allowed = common::floor_log2(data.len()) + 2;
    pdqsort_like(data, bad_allowed, true);
}

fn pdqsort_like(mut data: &mut [u64], mut bad_allowed: usize, mut was_balanced: bool) {
    while data.len() > TUNED_PARAMS.insertion_threshold {
        if bad_allowed == 0 {
            heap_sort::heap_sort(data);
            return;
        }

        if !was_balanced {
            break_patterns(data);
        }

        let len = data.len();
        let pivot = common::choose_pivot_ninther(data);
        let (lt, gt) = common::partition_3way(data, pivot);
        if lt == 0 && gt == len {
            return;
        }

        let left_len = lt;
        let right_len = len - gt;
        let unbalanced = left_len < (len / 8) || right_len < (len / 8);
        if unbalanced {
            bad_allowed = bad_allowed.saturating_sub(1);
        }
        was_balanced = !unbalanced;

        let (left, rest) = data.split_at_mut(lt);
        let (_, right) = rest.split_at_mut(gt - lt);

        if left.len() < right.len() {
            pdqsort_like(left, bad_allowed, was_balanced);
            data = right;
        } else {
            pdqsort_like(right, bad_allowed, was_balanced);
            data = left;
        }
    }

    common::insertion_sort(data);
}

fn break_patterns(data: &mut [u64]) {
    if data.len() < 8 {
        return;
    }

    let len = data.len();
    let mid = len / 2;
    let a = len / 4;
    let b = (len * 3) / 4;

    data.swap(0, mid);
    data.swap(a, len - 1);
    data.swap(b, (mid + 1).min(len - 1));
}
