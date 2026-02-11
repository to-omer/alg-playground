use crate::{SortContext, TUNED_PARAMS};

use super::common;

pub fn sort(data: &mut [u64], ctx: &mut SortContext) {
    let len = data.len();
    if len < 2 {
        return;
    }
    if common::is_sorted_non_decreasing(data) {
        return;
    }

    let buf = ctx.ensure_scratch(len);
    common::copy_u64_slice(buf, data);
    merge_sort_recursive(buf, data, 0, len);
}

fn merge_sort_recursive(src: &mut [u64], dst: &mut [u64], left: usize, right: usize) {
    let len = right - left;
    if len <= TUNED_PARAMS.insertion_threshold {
        common::copy_u64_slice(&mut dst[left..right], &src[left..right]);
        common::insertion_sort(&mut dst[left..right]);
        return;
    }

    let mid = left + (len >> 1);

    merge_sort_recursive(dst, src, left, mid);
    merge_sort_recursive(dst, src, mid, right);

    if src[mid - 1] <= src[mid] {
        common::copy_u64_slice(&mut dst[left..right], &src[left..right]);
        return;
    }

    common::merge_ranges(src, dst, left, mid, right);
}
