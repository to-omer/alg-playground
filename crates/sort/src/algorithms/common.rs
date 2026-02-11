use std::ptr;

use crate::TUNED_PARAMS;

#[inline]
pub fn insertion_sort(data: &mut [u64]) {
    let len = data.len();
    if len < 2 {
        return;
    }

    for i in 1..len {
        let key = data[i];
        let mut j = i;
        // Hot loop: unchecked accesses remove repeated bounds checks.
        unsafe {
            while j > 0 {
                let prev = *data.get_unchecked(j - 1);
                if prev <= key {
                    break;
                }
                *data.get_unchecked_mut(j) = prev;
                j -= 1;
            }
            *data.get_unchecked_mut(j) = key;
        }
    }
}

#[inline]
pub fn binary_insertion_sort(data: &mut [u64]) {
    let len = data.len();
    if len < 2 {
        return;
    }

    let ptr = data.as_mut_ptr();
    for i in 1..len {
        let key = unsafe { *ptr.add(i) };
        let mut left = 0usize;
        let mut right = i;
        while left < right {
            let mid = left + ((right - left) >> 1);
            unsafe {
                if *ptr.add(mid) <= key {
                    left = mid + 1;
                } else {
                    right = mid;
                }
            }
        }

        let pos = left;
        unsafe {
            // Shift by memmove semantics for overlapping regions.
            ptr::copy(ptr.add(pos), ptr.add(pos + 1), i - pos);
            *ptr.add(pos) = key;
        }
    }
}

#[inline]
pub fn is_sorted_non_decreasing(data: &[u64]) -> bool {
    if data.len() < 2 {
        return true;
    }
    let ptr = data.as_ptr();
    unsafe {
        for i in 1..data.len() {
            if *ptr.add(i - 1) > *ptr.add(i) {
                return false;
            }
        }
    }
    true
}

#[inline]
pub fn floor_log2(n: usize) -> usize {
    if n <= 1 {
        0
    } else {
        usize::BITS as usize - 1 - n.leading_zeros() as usize
    }
}

#[inline]
pub fn introsort_depth_limit(n: usize) -> usize {
    let log = floor_log2(n);
    (log * TUNED_PARAMS.introsort_depth_factor_num) / TUNED_PARAMS.introsort_depth_factor_den
}

#[inline]
pub fn median3(a: u64, b: u64, c: u64) -> u64 {
    if a < b {
        if b < c {
            b
        } else if a < c {
            c
        } else {
            a
        }
    } else if a < c {
        a
    } else if b < c {
        c
    } else {
        b
    }
}

#[inline]
pub fn choose_pivot_median3(data: &[u64]) -> u64 {
    let len = data.len();
    let a = data[0];
    let b = data[len >> 1];
    let c = data[len - 1];
    median3(a, b, c)
}

#[inline]
pub fn choose_pivot_ninther(data: &[u64]) -> u64 {
    if data.len() < 64 {
        return choose_pivot_median3(data);
    }

    let step = data.len() / 8;
    let m1 = median3(data[0], data[step], data[step * 2]);
    let mid = data.len() / 2;
    let m2 = median3(data[mid - step], data[mid], data[mid + step]);
    let r = data.len() - 1;
    let m3 = median3(data[r - step * 2], data[r - step], data[r]);
    median3(m1, m2, m3)
}

#[inline]
pub fn min_max(data: &[u64]) -> Option<(u64, u64)> {
    let (&first, rest) = data.split_first()?;
    let mut min = first;
    let mut max = first;
    for &x in rest {
        if x < min {
            min = x;
        }
        if x > max {
            max = x;
        }
    }
    Some((min, max))
}

#[inline]
pub fn partition_hoare(data: &mut [u64], pivot: u64) -> usize {
    debug_assert!(!data.is_empty());

    let ptr = data.as_mut_ptr();
    let mut i = 0usize;
    let mut j = data.len() - 1;

    unsafe {
        loop {
            while *ptr.add(i) < pivot {
                i += 1;
            }

            while *ptr.add(j) > pivot {
                j -= 1;
            }

            if i >= j {
                return j;
            }

            ptr::swap(ptr.add(i), ptr.add(j));
            i += 1;
            j -= 1;
        }
    }
}

#[inline]
pub fn partition_3way(data: &mut [u64], pivot: u64) -> (usize, usize) {
    let ptr = data.as_mut_ptr();
    let mut lt = 0usize;
    let mut i = 0usize;
    let mut gt = data.len();

    unsafe {
        while i < gt {
            let v = *ptr.add(i);
            if v < pivot {
                ptr::swap(ptr.add(i), ptr.add(lt));
                i += 1;
                lt += 1;
            } else if v > pivot {
                gt -= 1;
                ptr::swap(ptr.add(i), ptr.add(gt));
            } else {
                i += 1;
            }
        }
    }

    (lt, gt)
}

#[inline]
pub fn merge_ranges(src: &[u64], dst: &mut [u64], left: usize, mid: usize, right: usize) {
    let mut i = left;
    let mut j = mid;
    let mut k = left;

    while i < mid && j < right {
        if src[i] <= src[j] {
            dst[k] = src[i];
            i += 1;
        } else {
            dst[k] = src[j];
            j += 1;
        }
        k += 1;
    }

    if i < mid {
        copy_u64_slice(&mut dst[k..(k + (mid - i))], &src[i..mid]);
    } else if j < right {
        copy_u64_slice(&mut dst[k..(k + (right - j))], &src[j..right]);
    }
}

#[inline]
pub fn copy_u64_slice(dst: &mut [u64], src: &[u64]) {
    debug_assert_eq!(dst.len(), src.len());
    unsafe {
        copy_u64_ptr(dst.as_mut_ptr(), src.as_ptr(), dst.len());
    }
}

#[inline]
pub unsafe fn copy_u64_ptr(dst: *mut u64, src: *const u64, len: usize) {
    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") {
            unsafe {
                copy_u64_ptr_avx2(dst, src, len);
            }
            return;
        }
    }

    unsafe {
        ptr::copy_nonoverlapping(src, dst, len);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn copy_u64_ptr_avx2(dst: *mut u64, src: *const u64, len: usize) {
    use std::arch::x86_64::{__m256i, _mm256_loadu_si256, _mm256_storeu_si256};

    let mut i = 0usize;
    while i + 4 <= len {
        unsafe {
            let v = _mm256_loadu_si256(src.add(i).cast::<__m256i>());
            _mm256_storeu_si256(dst.add(i).cast::<__m256i>(), v);
        }
        i += 4;
    }

    if i < len {
        unsafe {
            ptr::copy_nonoverlapping(src.add(i), dst.add(i), len - i);
        }
    }
}
