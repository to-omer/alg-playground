use crate::SortContext;

use super::{common, radix_sort_lsd_base256};

const MAX_PIGEONHOLE_RANGE: usize = 1 << 21;

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
    if range > MAX_PIGEONHOLE_RANGE || range > len.saturating_mul(48) {
        radix_sort_lsd_base256::sort(data, ctx);
        return;
    }

    let holes = ctx.ensure_var_counts(range);
    holes.fill(0);

    for &value in data.iter() {
        holes[(value - min) as usize] += 1;
    }

    let mut out = 0usize;
    for (offset, &count) in holes.iter().enumerate() {
        let value = min + offset as u64;
        if count > 0 {
            data[out..(out + count)].fill(value);
            out += count;
        }
    }
}
