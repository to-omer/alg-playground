use crate::SortContext;

pub fn sort(data: &mut [u64], _ctx: &mut SortContext) {
    heap_sort(data);
}

pub fn heap_sort(data: &mut [u64]) {
    let len = data.len();
    if len < 2 {
        return;
    }

    let mut start = (len - 2) / 2;
    loop {
        sift_down(data, start, len);
        if start == 0 {
            break;
        }
        start -= 1;
    }

    let mut end = len - 1;
    while end > 0 {
        data.swap(0, end);
        sift_down(data, 0, end);
        end -= 1;
    }
}

#[inline]
fn sift_down(data: &mut [u64], mut root: usize, end: usize) {
    let ptr = data.as_mut_ptr();
    unsafe {
        loop {
            let child = root * 2 + 1;
            if child >= end {
                break;
            }

            let mut swap_idx = child;
            if child + 1 < end && *ptr.add(child) < *ptr.add(child + 1) {
                swap_idx = child + 1;
            }

            if *ptr.add(root) >= *ptr.add(swap_idx) {
                break;
            }

            std::ptr::swap(ptr.add(root), ptr.add(swap_idx));
            root = swap_idx;
        }
    }
}
