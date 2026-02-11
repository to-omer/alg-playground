use crate::{SortContext, TUNED_PARAMS};

use super::common;

pub fn sort(data: &mut [u64], _ctx: &mut SortContext) {
    dual_pivot_quick_sort(data);
}

fn dual_pivot_quick_sort(mut data: &mut [u64]) {
    while data.len() > TUNED_PARAMS.insertion_threshold {
        let len = data.len();
        let i1 = len / 3;
        let mut i2 = (len * 2) / 3;
        if i1 == i2 {
            i2 = (i2 + 1).min(len - 1);
        }

        let mut p = data[i1];
        let mut q = data[i2];
        if p > q {
            std::mem::swap(&mut p, &mut q);
        }

        if p == q {
            let (lt, gt) = common::partition_3way(data, p);
            if lt == 0 && gt == len {
                return;
            }
            let (left, rest) = data.split_at_mut(lt);
            let (_, right) = rest.split_at_mut(gt - lt);
            if left.len() < right.len() {
                dual_pivot_quick_sort(left);
                data = right;
            } else {
                dual_pivot_quick_sort(right);
                data = left;
            }
            continue;
        }

        let mut lt = 0usize;
        let mut i = 0usize;
        let mut gt = len;
        while i < gt {
            let v = data[i];
            if v < p {
                data.swap(i, lt);
                lt += 1;
                i += 1;
            } else if v > q {
                gt -= 1;
                data.swap(i, gt);
            } else {
                i += 1;
            }
        }

        if lt == 0 && gt == len {
            let pivot = common::choose_pivot_ninther(data);
            let (eq_left, eq_right) = common::partition_3way(data, pivot);
            if eq_left == 0 && eq_right == len {
                return;
            }
            let (left, rest) = data.split_at_mut(eq_left);
            let (_, right) = rest.split_at_mut(eq_right - eq_left);
            if left.len() < right.len() {
                dual_pivot_quick_sort(left);
                data = right;
            } else {
                dual_pivot_quick_sort(right);
                data = left;
            }
            continue;
        }

        let (left, rest) = data.split_at_mut(lt);
        let mid_len = gt - lt;
        let (middle, right) = rest.split_at_mut(mid_len);

        if left.len() >= middle.len() && left.len() >= right.len() {
            dual_pivot_quick_sort(middle);
            dual_pivot_quick_sort(right);
            data = left;
        } else if middle.len() >= right.len() {
            dual_pivot_quick_sort(left);
            dual_pivot_quick_sort(right);
            data = middle;
        } else {
            dual_pivot_quick_sort(left);
            dual_pivot_quick_sort(middle);
            data = right;
        }
    }

    common::insertion_sort(data);
}
