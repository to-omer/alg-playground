pub fn monotone_minima<F>(rows: usize, cols: usize, cost: &F) -> Vec<usize>
where
    F: Fn(usize, usize) -> u64,
{
    if rows == 0 {
        return Vec::new();
    }
    assert!(cols > 0, "cols must be positive");

    let mut argmins = vec![0; rows];
    monotone_minima_recursive_range(0, rows, 0, cols, cost, &mut argmins);
    argmins
}

#[inline]
fn monotone_minima_recursive_range<F>(
    row_start: usize,
    row_end: usize,
    col_start: usize,
    col_end: usize,
    cost: &F,
    argmins: &mut [usize],
) where
    F: Fn(usize, usize) -> u64,
{
    if row_start >= row_end || col_start >= col_end {
        return;
    }
    if col_end - col_start == 1 {
        argmins[row_start..row_end].fill(col_start);
        return;
    }
    let mid = (row_start + row_end) / 2;
    let mut best_col = col_start;
    let mut best_val = cost(mid, best_col);
    for col in col_start + 1..col_end {
        let value = cost(mid, col);
        if value < best_val {
            best_val = value;
            best_col = col;
        }
    }
    argmins[mid] = best_col;

    if row_end - row_start == 1 {
        return;
    }

    monotone_minima_recursive_range(row_start, mid, col_start, best_col + 1, cost, argmins);
    monotone_minima_recursive_range(mid + 1, row_end, best_col, col_end, cost, argmins);
}
