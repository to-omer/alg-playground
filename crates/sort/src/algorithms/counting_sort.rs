use crate::SortContext;

use super::{common, radix_sort_lsd_base256};

const MAX_COUNTING_RANGE: usize = 1 << 20;

pub fn sort(data: &mut [u64], ctx: &mut SortContext) {
    let len = data.len();
    if len < 2 {
        return;
    }

    let Some((min, max)) = common::min_max(data) else {
        return;
    };
    if min == max {
        return;
    }

    let range_u128 = (max as u128) - (min as u128) + 1;
    if range_u128 > usize::MAX as u128 {
        radix_sort_lsd_base256::sort(data, ctx);
        return;
    }

    let range = range_u128 as usize;
    if range > MAX_COUNTING_RANGE || range > len.saturating_mul(24) {
        radix_sort_lsd_base256::sort(data, ctx);
        return;
    }

    let counts = ctx.ensure_var_counts(range);
    counts.fill(0);

    for &x in data.iter() {
        let idx = (x - min) as usize;
        counts[idx] += 1;
    }

    let mut out = 0usize;
    for (i, &count) in counts.iter().enumerate() {
        if count == 0 {
            continue;
        }
        let value = min + i as u64;
        data[out..(out + count)].fill(value);
        out += count;
    }
}
