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

    let Some((min, max)) = common::min_max(data) else {
        return;
    };
    if min == max {
        return;
    }

    let bucket_count =
        ((len / TUNED_PARAMS.bucket_size_divisor).clamp(16, 4096)).next_power_of_two();
    let range = (max as u128) - (min as u128) + 1;

    let crate::SortContext {
        scratch,
        var_counts,
        ..
    } = ctx;

    if var_counts.len() < bucket_count * 2 {
        var_counts.resize(bucket_count * 2, 0);
    }
    if scratch.len() < len {
        scratch.resize(len, 0);
    }

    let counts_and_heads = &mut var_counts[..(bucket_count * 2)];
    let (starts, heads) = counts_and_heads.split_at_mut(bucket_count);
    starts.fill(0);
    heads.fill(0);

    for &x in data.iter() {
        let idx = bucket_index(x, min, range, bucket_count);
        starts[idx] += 1;
    }

    let mut prefix = 0usize;
    for i in 0..bucket_count {
        let c = starts[i];
        starts[i] = prefix;
        heads[i] = prefix;
        prefix += c;
    }

    for &x in data.iter() {
        let idx = bucket_index(x, min, range, bucket_count);
        let pos = heads[idx];
        scratch[pos] = x;
        heads[idx] += 1;
    }

    for i in 0..bucket_count {
        let start = starts[i];
        let end = if i + 1 < bucket_count {
            starts[i + 1]
        } else {
            len
        };
        if end - start <= 1 {
            continue;
        }
        if end - start <= TUNED_PARAMS.insertion_threshold {
            common::insertion_sort(&mut scratch[start..end]);
        } else {
            scratch[start..end].sort_unstable();
        }
    }

    common::copy_u64_slice(data, &scratch[..len]);
}

#[inline]
fn bucket_index(value: u64, min: u64, range: u128, bucket_count: usize) -> usize {
    let offset = (value - min) as u128;
    let idx = (offset * bucket_count as u128 / range) as usize;
    idx.min(bucket_count - 1)
}
