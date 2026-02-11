use crate::SortContext;

use super::common;

pub fn sort(data: &mut [u64], ctx: &mut SortContext) {
    if data.len() < 2 {
        return;
    }
    if common::is_sorted_non_decreasing(data) {
        return;
    }

    let passes = radix_passes(data);
    if passes == 0 {
        return;
    }

    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") {
            unsafe {
                radix_sort_impl_avx2(data, ctx, passes);
            }
            return;
        }
    }

    radix_sort_impl_scalar(data, ctx, passes);
}

fn radix_sort_impl_scalar(data: &mut [u64], ctx: &mut SortContext, passes: usize) {
    let len = data.len();
    let SortContext {
        scratch, counts256, ..
    } = ctx;
    if scratch.len() < len {
        scratch.resize(len, 0);
    }

    let mut src_is_data = true;
    for pass in 0..passes {
        let shift = pass * 8;

        if src_is_data {
            count_digits(data, counts256, shift);
            prefix_sum(counts256);
            scatter_scalar(data, &mut scratch[..len], counts256, shift);
        } else {
            count_digits(&scratch[..len], counts256, shift);
            prefix_sum(counts256);
            scatter_scalar(&scratch[..len], data, counts256, shift);
        }

        src_is_data = !src_is_data;
    }

    if !src_is_data {
        common::copy_u64_slice(data, &scratch[..len]);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn radix_sort_impl_avx2(data: &mut [u64], ctx: &mut SortContext, passes: usize) {
    let len = data.len();
    let SortContext {
        scratch, counts256, ..
    } = ctx;
    if scratch.len() < len {
        scratch.resize(len, 0);
    }

    let mut src_is_data = true;
    for pass in 0..passes {
        let shift = pass * 8;

        if src_is_data {
            unsafe {
                count_digits_avx2(data, counts256, shift);
            }
            prefix_sum(counts256);
            scatter_scalar(data, &mut scratch[..len], counts256, shift);
        } else {
            unsafe {
                count_digits_avx2(&scratch[..len], counts256, shift);
            }
            prefix_sum(counts256);
            scatter_scalar(&scratch[..len], data, counts256, shift);
        }

        src_is_data = !src_is_data;
    }

    if !src_is_data {
        common::copy_u64_slice(data, &scratch[..len]);
    }
}

#[inline]
fn radix_passes(data: &[u64]) -> usize {
    let first = data[0];
    let mut diff = 0_u64;
    for &x in data.iter().skip(1) {
        diff |= first ^ x;
    }
    if diff == 0 {
        return 0;
    }
    ((63 - diff.leading_zeros()) as usize / 8) + 1
}

#[inline]
fn count_digits(src: &[u64], counts: &mut [usize; 256], shift: usize) {
    let mut c0 = [0usize; 256];
    let mut c1 = [0usize; 256];
    let mut c2 = [0usize; 256];
    let mut c3 = [0usize; 256];

    let mut i = 0usize;
    while i + 4 <= src.len() {
        let x0 = unsafe { *src.get_unchecked(i) };
        let x1 = unsafe { *src.get_unchecked(i + 1) };
        let x2 = unsafe { *src.get_unchecked(i + 2) };
        let x3 = unsafe { *src.get_unchecked(i + 3) };

        c0[((x0 >> shift) & 0xFF) as usize] += 1;
        c1[((x1 >> shift) & 0xFF) as usize] += 1;
        c2[((x2 >> shift) & 0xFF) as usize] += 1;
        c3[((x3 >> shift) & 0xFF) as usize] += 1;
        i += 4;
    }

    while i < src.len() {
        c0[((src[i] >> shift) & 0xFF) as usize] += 1;
        i += 1;
    }

    for idx in 0..256 {
        counts[idx] = c0[idx] + c1[idx] + c2[idx] + c3[idx];
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn count_digits_avx2(src: &[u64], counts: &mut [usize; 256], shift: usize) {
    use std::arch::x86_64::{__m256i, _mm256_loadu_si256};

    let mut c0 = [0usize; 256];
    let mut c1 = [0usize; 256];
    let mut c2 = [0usize; 256];
    let mut c3 = [0usize; 256];

    let mut i = 0usize;
    while i + 4 <= src.len() {
        let packed = unsafe { _mm256_loadu_si256(src.as_ptr().add(i).cast::<__m256i>()) };
        let lanes: [u64; 4] = unsafe { std::mem::transmute(packed) };

        c0[((lanes[0] >> shift) & 0xFF) as usize] += 1;
        c1[((lanes[1] >> shift) & 0xFF) as usize] += 1;
        c2[((lanes[2] >> shift) & 0xFF) as usize] += 1;
        c3[((lanes[3] >> shift) & 0xFF) as usize] += 1;

        i += 4;
    }

    while i < src.len() {
        c0[((src[i] >> shift) & 0xFF) as usize] += 1;
        i += 1;
    }

    for idx in 0..256 {
        counts[idx] = c0[idx] + c1[idx] + c2[idx] + c3[idx];
    }
}

#[inline]
fn prefix_sum(counts: &mut [usize; 256]) {
    let mut sum = 0usize;
    for c in counts.iter_mut() {
        let old = *c;
        *c = sum;
        sum += old;
    }
}

#[inline]
fn scatter_scalar(src: &[u64], dst: &mut [u64], offsets: &mut [usize; 256], shift: usize) {
    let dst_ptr = dst.as_mut_ptr();
    unsafe {
        for &x in src {
            let digit = ((x >> shift) & 0xFF) as usize;
            let pos = *offsets.get_unchecked(digit);
            *dst_ptr.add(pos) = x;
            *offsets.get_unchecked_mut(digit) = pos + 1;
        }
    }
}
