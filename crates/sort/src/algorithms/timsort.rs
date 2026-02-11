use crate::{SortContext, TUNED_PARAMS};

use super::common;

#[derive(Clone, Copy)]
struct Run {
    start: usize,
    len: usize,
}

pub fn sort(data: &mut [u64], ctx: &mut SortContext) {
    let n = data.len();
    if n < 2 {
        return;
    }
    if common::is_sorted_non_decreasing(data) {
        return;
    }

    let min_run = min_run_length(n);
    let mut runs: Vec<Run> = Vec::with_capacity(64);

    let mut i = 0usize;
    while i < n {
        let mut run_len = count_run_and_make_ascending(data, i);
        let remaining = n - i;
        if run_len < min_run {
            let force = remaining.min(min_run);
            binary_insertion_sort_range(data, i, i + force);
            run_len = force;
        }

        runs.push(Run {
            start: i,
            len: run_len,
        });
        merge_collapse(data, &mut runs, ctx);
        i += run_len;
    }

    merge_force_collapse(data, &mut runs, ctx);
}

fn min_run_length(mut n: usize) -> usize {
    let mut r = 0usize;
    let limit = TUNED_PARAMS.timsort_min_run.max(2);
    while n >= limit * 2 {
        r |= n & 1;
        n >>= 1;
    }
    n + r
}

fn count_run_and_make_ascending(data: &mut [u64], start: usize) -> usize {
    let n = data.len();
    let mut end = start + 1;
    if end >= n {
        return 1;
    }

    if data[end] < data[start] {
        while end < n && data[end] < data[end - 1] {
            end += 1;
        }
        data[start..end].reverse();
    } else {
        while end < n && data[end] >= data[end - 1] {
            end += 1;
        }
    }

    end - start
}

fn binary_insertion_sort_range(data: &mut [u64], start: usize, end: usize) {
    for i in (start + 1)..end {
        let key = data[i];
        let mut left = start;
        let mut right = i;
        while left < right {
            let mid = left + ((right - left) >> 1);
            if data[mid] <= key {
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        let pos = left;
        unsafe {
            let ptr = data.as_mut_ptr();
            std::ptr::copy(ptr.add(pos), ptr.add(pos + 1), i - pos);
            *ptr.add(pos) = key;
        }
    }
}

fn merge_collapse(data: &mut [u64], runs: &mut Vec<Run>, ctx: &mut SortContext) {
    while runs.len() > 1 {
        let n = runs.len();

        let cond_a = n >= 3 && runs[n - 3].len <= runs[n - 2].len + runs[n - 1].len;
        let cond_b = n >= 4 && runs[n - 4].len <= runs[n - 3].len + runs[n - 2].len;
        if cond_a || cond_b {
            if n >= 3 && runs[n - 3].len < runs[n - 1].len {
                merge_at(data, runs, n - 3, ctx);
            } else {
                merge_at(data, runs, n - 2, ctx);
            }
            continue;
        }

        if runs[n - 2].len <= runs[n - 1].len {
            merge_at(data, runs, n - 2, ctx);
            continue;
        }

        break;
    }
}

fn merge_force_collapse(data: &mut [u64], runs: &mut Vec<Run>, ctx: &mut SortContext) {
    while runs.len() > 1 {
        let n = runs.len();
        if n >= 3 && runs[n - 3].len < runs[n - 1].len {
            merge_at(data, runs, n - 3, ctx);
        } else {
            merge_at(data, runs, n - 2, ctx);
        }
    }
}

fn merge_at(data: &mut [u64], runs: &mut Vec<Run>, idx: usize, ctx: &mut SortContext) {
    let left = runs[idx];
    let right = runs[idx + 1];

    debug_assert_eq!(left.start + left.len, right.start);

    let aux = ctx.ensure_aux(left.len);
    common::copy_u64_slice(aux, &data[left.start..(left.start + left.len)]);

    let mut i = 0usize;
    let mut j = right.start;
    let mut out = left.start;
    let right_end = right.start + right.len;

    while i < left.len && j < right_end {
        if aux[i] <= data[j] {
            data[out] = aux[i];
            i += 1;
        } else {
            data[out] = data[j];
            j += 1;
        }
        out += 1;
    }

    if i < left.len {
        common::copy_u64_slice(&mut data[out..(out + (left.len - i))], &aux[i..left.len]);
    }

    runs[idx] = Run {
        start: left.start,
        len: left.len + right.len,
    };
    let tail_from = idx + 2;
    if tail_from < runs.len() {
        runs.copy_within(tail_from.., idx + 1);
    }
    runs.pop();
}
