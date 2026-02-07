#[inline(always)]
pub(crate) fn better_index(values: &[i64], a: usize, b: usize) -> usize {
    let va = values[a];
    let vb = values[b];
    if va < vb || (va == vb && a < b) { a } else { b }
}

#[inline(always)]
pub(crate) fn better_index_ordered(values: &[i64], a: usize, b: usize) -> usize {
    debug_assert!(a < b);
    if values[a] <= values[b] { a } else { b }
}

#[inline(always)]
pub(crate) fn is_strictly_less(values: &[i64], a: usize, b: usize) -> bool {
    let va = values[a];
    let vb = values[b];
    va < vb || (va == vb && a < b)
}

#[inline(always)]
pub(crate) fn floor_log2_nonzero(x: usize) -> u32 {
    debug_assert!(x > 0);
    usize::BITS - 1 - x.leading_zeros()
}
