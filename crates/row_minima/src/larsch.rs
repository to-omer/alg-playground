pub struct Larsch {
    base: ReduceRow,
}

impl Larsch {
    pub fn new<F>(rows: usize, cost: F) -> Self
    where
        F: Fn(usize, usize) -> u64 + 'static,
    {
        let mut base = ReduceRow::new(rows);
        base.set_f(std::rc::Rc::new(cost));
        Self { base }
    }

    pub fn get_argmin(&mut self) -> usize {
        self.base.get_argmin()
    }
}

pub fn larsch_shortest_path<F>(n: usize, cost: F) -> Vec<u64>
where
    F: Fn(usize, usize) -> u64 + 'static,
{
    if n == 0 {
        return Vec::new();
    }

    let inf = u64::MAX / 4;
    let dp = std::rc::Rc::new(std::cell::RefCell::new(vec![inf; n]));
    dp.borrow_mut()[0] = 0;

    let dp_for_cost = std::rc::Rc::clone(&dp);
    let cost = std::rc::Rc::new(cost);
    let cost_for_cost = std::rc::Rc::clone(&cost);
    let mut larsch = Larsch::new(n - 1, move |i, col| {
        let row = i + 1;
        if row <= col {
            return inf;
        }
        let dp = dp_for_cost.borrow();
        dp[col] + cost_for_cost(row, col)
    });

    for row in 1..n {
        let col = larsch.get_argmin();
        let mut dp = dp.borrow_mut();
        let value = dp[col] + cost(row, col);
        dp[row] = value;
    }

    dp.take()
}

struct ReduceRow {
    n: usize,
    f: std::rc::Rc<dyn Fn(usize, usize) -> u64>,
    cur_row: usize,
    state: usize,
    rec: Option<Box<ReduceCol>>,
}

impl ReduceRow {
    fn new(n: usize) -> Self {
        let rec = if n / 2 == 0 {
            None
        } else {
            Some(Box::new(ReduceCol::new(n / 2)))
        };
        Self {
            n,
            f: std::rc::Rc::new(|_, _| 0),
            cur_row: 0,
            state: 0,
            rec,
        }
    }

    fn set_f(&mut self, f: std::rc::Rc<dyn Fn(usize, usize) -> u64>) {
        self.f = f.clone();
        if let Some(rec) = &mut self.rec {
            let f2 = std::rc::Rc::new(move |i: usize, j: usize| (f)(2 * i + 1, j));
            rec.set_f(f2);
        }
    }

    fn get_argmin(&mut self) -> usize {
        let f = &self.f;
        let cur_row = self.cur_row;
        self.cur_row += 1;
        if cur_row & 1 == 0 {
            let prev_argmin = self.state;
            let next_argmin = if cur_row + 1 == self.n {
                self.n - 1
            } else {
                self.rec.as_mut().expect("reduce_col missing").get_argmin()
            };
            self.state = next_argmin;
            if prev_argmin == next_argmin {
                return prev_argmin;
            }
            let mut ret = prev_argmin;
            let mut best_val = f(cur_row, ret);
            for col in prev_argmin + 1..=next_argmin {
                let value = f(cur_row, col);
                if value < best_val {
                    best_val = value;
                    ret = col;
                }
            }
            ret
        } else if f(cur_row, self.state) <= f(cur_row, cur_row) {
            self.state
        } else {
            cur_row
        }
    }
}

struct ReduceCol {
    n: usize,
    f: std::rc::Rc<dyn Fn(usize, usize) -> u64>,
    cur_row: usize,
    cols: std::rc::Rc<std::cell::RefCell<Vec<usize>>>,
    rec: Box<ReduceRow>,
}

impl ReduceCol {
    fn new(n: usize) -> Self {
        let cols = std::rc::Rc::new(std::cell::RefCell::new(Vec::with_capacity(n)));
        Self {
            n,
            f: std::rc::Rc::new(|_, _| 0),
            cur_row: 0,
            cols,
            rec: Box::new(ReduceRow::new(n)),
        }
    }

    fn set_f(&mut self, f: std::rc::Rc<dyn Fn(usize, usize) -> u64>) {
        self.f = f.clone();
        let cols = self.cols.clone();
        let f2 = std::rc::Rc::new(move |i: usize, j: usize| {
            let cols = cols.borrow();
            (f)(i, cols[j])
        });
        self.rec.set_f(f2);
    }

    fn get_argmin(&mut self) -> usize {
        let f = &self.f;
        let cur_row = self.cur_row;
        self.cur_row += 1;
        let candidates = if cur_row == 0 {
            [0, 0]
        } else {
            [2 * cur_row - 1, 2 * cur_row]
        };

        {
            let mut cols = self.cols.borrow_mut();
            for &col in candidates.iter().take(if cur_row == 0 { 1 } else { 2 }) {
                loop {
                    let size = cols.len();
                    if size == cur_row {
                        break;
                    }
                    let last_col = cols[size - 1];
                    if f(size - 1, last_col) > f(size - 1, col) {
                        cols.pop();
                    } else {
                        break;
                    }
                }
                if cols.len() != self.n {
                    cols.push(col);
                }
            }
        }

        let index = self.rec.get_argmin();
        let cols = self.cols.borrow();
        cols[index]
    }
}
