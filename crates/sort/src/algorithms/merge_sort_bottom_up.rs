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

    let run = TUNED_PARAMS.insertion_threshold.max(8);
    if len <= run {
        common::insertion_sort(data);
        return;
    }

    for chunk in data.chunks_mut(run) {
        common::insertion_sort(chunk);
    }

    let buf = ctx.ensure_scratch(len);
    common::copy_u64_slice(buf, data);

    let mut width = run;
    let mut src_is_buf = true;
    while width < len {
        if src_is_buf {
            merge_pass(&buf[..len], data, width);
        } else {
            merge_pass(data, &mut buf[..len], width);
        }
        src_is_buf = !src_is_buf;
        width <<= 1;
    }

    if src_is_buf {
        common::copy_u64_slice(data, &buf[..len]);
    }
}

fn merge_pass(src: &[u64], dst: &mut [u64], width: usize) {
    let len = src.len();
    let mut left = 0usize;
    while left < len {
        let mid = (left + width).min(len);
        let right = (mid + width).min(len);

        if mid >= right || src[mid - 1] <= src[mid] {
            common::copy_u64_slice(&mut dst[left..right], &src[left..right]);
        } else {
            common::merge_ranges(src, dst, left, mid, right);
        }

        left = right;
    }
}
