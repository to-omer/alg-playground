pub fn smawk<F>(rows: usize, cols: usize, cost: &F) -> Vec<usize>
where
    F: Fn(usize, usize) -> u64,
{
    if rows == 0 {
        return Vec::new();
    }
    assert!(cols > 0, "cols must be positive");

    let mut solver = SmawkFast::new();
    solver.solve(rows, cols, cost)
}

struct SmawkFast {
    cols: Vec<usize>,
    row_argmin: Vec<usize>,
    stack: Vec<u64>,
}

impl SmawkFast {
    fn new() -> Self {
        Self {
            cols: Vec::new(),
            row_argmin: Vec::new(),
            stack: Vec::new(),
        }
    }

    fn solve<F>(&mut self, rows: usize, cols: usize, cost: &F) -> Vec<usize>
    where
        F: Fn(usize, usize) -> u64,
    {
        self.cols.clear();
        self.cols.reserve(cols + rows + rows);
        self.cols.extend(0..cols);
        self.row_argmin.resize(rows, 0);
        self.stack.resize(rows, 0);
        self.recur(rows, cols, 0, 0, cols, cost);
        std::mem::take(&mut self.row_argmin)
    }

    fn recur<F>(
        &mut self,
        rows: usize,
        cols: usize,
        level: usize,
        begin: usize,
        end: usize,
        cost: &F,
    ) where
        F: Fn(usize, usize) -> u64,
    {
        if rows < (2usize << level) {
            let row = (1usize << level) - 1;
            if end - begin == 1 {
                self.row_argmin[row] = self.cols[begin];
                return;
            }
            let mut best_col = self.cols[begin];
            let mut best_val = cost(row, best_col);
            for idx in begin + 1..end {
                let col = self.cols[idx];
                let value = cost(row, col);
                if value < best_val {
                    best_val = value;
                    best_col = col;
                }
            }
            self.row_argmin[row] = best_col;
            return;
        }

        let mut top = 0usize;
        for idx in begin..end {
            let col = self.cols[idx];
            while top > 0 {
                let row = (top << level) - 1;
                if self.stack[top - 1] > cost(row, col) {
                    top -= 1;
                } else {
                    break;
                }
            }
            let row = ((top + 1) << level) - 1;
            if row < rows {
                if end + top == self.cols.len() {
                    self.cols.push(col);
                } else {
                    self.cols[end + top] = col;
                }
                self.stack[top] = cost(row, col);
                top += 1;
            }
        }

        let next_begin = end;
        let next_end = end + top;
        self.recur(rows, cols, level + 1, next_begin, next_end, cost);

        let offset = 1usize << level;
        let mut row = offset - 1;
        let mut pos = next_begin;
        while row < rows {
            let high = if row + offset < rows {
                self.row_argmin[row + offset] + 1
            } else {
                cols
            };
            let mut best_col = self.cols[pos];
            let mut best_val = cost(row, best_col);
            while pos + 1 < next_end && self.cols[pos + 1] < high {
                pos += 1;
                let col = self.cols[pos];
                let value = cost(row, col);
                if value < best_val {
                    best_val = value;
                    best_col = col;
                }
            }
            self.row_argmin[row] = best_col;
            row += offset << 1;
        }
    }
}
