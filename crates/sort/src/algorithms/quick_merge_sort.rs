use crate::{SortContext, TUNED_PARAMS};

use super::{common, merge_sort_top_down};

pub fn sort(data: &mut [u64], ctx: &mut SortContext) {
    if data.len() < 2 {
        return;
    }
    let depth_limit = common::introsort_depth_limit(data.len()) + 1;
    quick_merge_sort(data, ctx, depth_limit);
}

fn quick_merge_sort(mut data: &mut [u64], ctx: &mut SortContext, mut depth_limit: usize) {
    while data.len() > TUNED_PARAMS.insertion_threshold {
        if depth_limit == 0 {
            merge_sort_top_down::sort(data, ctx);
            return;
        }

        depth_limit -= 1;
        let len = data.len();
        let pivot = common::choose_pivot_ninther(data);
        let cut = common::partition_hoare(data, pivot) + 1;

        if cut == 0 || cut == len {
            let (lt, gt) = common::partition_3way(data, pivot);
            if lt == 0 && gt == len {
                return;
            }
            let (left, rest) = data.split_at_mut(lt);
            let (_, right) = rest.split_at_mut(gt - lt);
            if left.len() < right.len() {
                quick_merge_sort(left, ctx, depth_limit);
                data = right;
            } else {
                quick_merge_sort(right, ctx, depth_limit);
                data = left;
            }
            continue;
        }

        let (left, right) = data.split_at_mut(cut);
        let unbalanced = left.len() < (len / 8) || right.len() < (len / 8);
        if unbalanced {
            if left.len() < right.len() {
                quick_merge_sort(left, ctx, depth_limit);
                merge_sort_top_down::sort(right, ctx);
            } else {
                quick_merge_sort(right, ctx, depth_limit);
                merge_sort_top_down::sort(left, ctx);
            }
            return;
        }

        if left.len() < right.len() {
            quick_merge_sort(left, ctx, depth_limit);
            data = right;
        } else {
            quick_merge_sort(right, ctx, depth_limit);
            data = left;
        }
    }

    common::insertion_sort(data);
}
