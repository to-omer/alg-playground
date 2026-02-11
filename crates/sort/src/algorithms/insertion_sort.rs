use crate::SortContext;

use super::common;

pub fn sort(data: &mut [u64], _ctx: &mut SortContext) {
    common::insertion_sort(data);
}
