use crate::{SortContext, TUNED_PARAMS};

use super::common;

pub fn sort(data: &mut [u64], _ctx: &mut SortContext) {
    if data.len() < 2 {
        return;
    }
    if common::is_sorted_non_decreasing(data) {
        return;
    }
    let Some((min, max)) = common::min_max(data) else {
        return;
    };
    let diff = min ^ max;
    if diff == 0 {
        return;
    }
    let start_byte = ((63 - diff.leading_zeros()) / 8) as i32;
    american_flag_sort_msd(data, start_byte);
}

fn american_flag_sort_msd(data: &mut [u64], byte: i32) {
    if data.len() <= TUNED_PARAMS.insertion_threshold || byte < 0 {
        common::insertion_sort(data);
        return;
    }

    let shift = (byte as usize) * 8;
    let mut counts = [0usize; 256];
    for &x in data.iter() {
        counts[digit(x, shift)] += 1;
    }

    let non_zero_buckets = counts.iter().filter(|&&c| c > 0).count();
    if non_zero_buckets <= 1 {
        if byte > 0 {
            american_flag_sort_msd(data, byte - 1);
        }
        return;
    }

    let mut starts = [0usize; 256];
    let mut ends = [0usize; 256];
    let mut sum = 0usize;
    for i in 0..256 {
        starts[i] = sum;
        sum += counts[i];
        ends[i] = sum;
    }

    let mut next = starts;
    for bucket in 0..256 {
        while next[bucket] < ends[bucket] {
            let from = next[bucket];
            let mut value = data[from];
            let mut d = digit(value, shift);

            while d != bucket {
                let to = next[d];
                next[d] += 1;
                std::mem::swap(&mut value, &mut data[to]);
                d = digit(value, shift);
            }

            data[from] = value;
            next[bucket] += 1;
        }
    }

    if byte == 0 {
        return;
    }

    for bucket in 0..256 {
        let start = starts[bucket];
        let end = ends[bucket];
        if end - start > 1 {
            american_flag_sort_msd(&mut data[start..end], byte - 1);
        }
    }
}

#[inline]
fn digit(x: u64, shift: usize) -> usize {
    ((x >> shift) & 0xFF) as usize
}
