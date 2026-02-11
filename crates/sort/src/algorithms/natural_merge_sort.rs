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
    if len <= TUNED_PARAMS.insertion_threshold {
        common::insertion_sort(data);
        return;
    }

    let scratch = ctx.ensure_scratch(len);
    let mut runs = Vec::with_capacity((len / TUNED_PARAMS.timsort_min_run).max(1));

    loop {
        runs.clear();
        collect_runs(data, &mut runs);
        if runs.len() <= 1 {
            break;
        }

        let mut write = 0usize;
        let mut idx = 0usize;
        while idx < runs.len() {
            if idx + 1 < runs.len() {
                let (l0, l1) = runs[idx];
                let (r0, r1) = runs[idx + 1];
                let total = (l1 - l0) + (r1 - r0);
                merge_two_runs(
                    &data[l0..l1],
                    &data[r0..r1],
                    &mut scratch[write..(write + total)],
                );
                write += total;
                idx += 2;
            } else {
                let (start, end) = runs[idx];
                let run_len = end - start;
                common::copy_u64_slice(&mut scratch[write..(write + run_len)], &data[start..end]);
                write += run_len;
                idx += 1;
            }
        }

        common::copy_u64_slice(data, &scratch[..len]);
    }
}

fn collect_runs(data: &mut [u64], runs: &mut Vec<(usize, usize)>) {
    let n = data.len();
    let mut i = 0usize;

    while i < n {
        let start = i;
        i += 1;
        if i == n {
            runs.push((start, i));
            break;
        }

        if data[i - 1] <= data[i] {
            while i < n && data[i - 1] <= data[i] {
                i += 1;
            }
        } else {
            while i < n && data[i - 1] > data[i] {
                i += 1;
            }
            data[start..i].reverse();
        }

        runs.push((start, i));
    }
}

#[inline]
fn merge_two_runs(left: &[u64], right: &[u64], dst: &mut [u64]) {
    let mut i = 0usize;
    let mut j = 0usize;
    let mut k = 0usize;

    while i < left.len() && j < right.len() {
        if left[i] <= right[j] {
            dst[k] = left[i];
            i += 1;
        } else {
            dst[k] = right[j];
            j += 1;
        }
        k += 1;
    }

    if i < left.len() {
        common::copy_u64_slice(&mut dst[k..(k + (left.len() - i))], &left[i..]);
    } else if j < right.len() {
        common::copy_u64_slice(&mut dst[k..(k + (right.len() - j))], &right[j..]);
    }
}
