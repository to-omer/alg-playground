mod larsch;
mod monotone_minima;
mod simple_larsch;
mod smawk;

pub use larsch::{Larsch, larsch_shortest_path};
pub use monotone_minima::monotone_minima;
pub use simple_larsch::simple_larsch_shortest_path;
pub use smawk::smawk;

#[cfg(test)]
mod tests {
    use super::{larsch_shortest_path, monotone_minima, simple_larsch_shortest_path, smawk};

    fn assert_row_minima<F>(rows: usize, cols: usize, cost: &F, argmins: &[usize])
    where
        F: Fn(usize, usize) -> u64,
    {
        for (row, &argmin) in argmins.iter().enumerate().take(rows) {
            let mut best_val = cost(row, 0);
            for col in 1..cols {
                let value = cost(row, col);
                if value < best_val {
                    best_val = value;
                }
            }
            let got_val = cost(row, argmin);
            assert_eq!(got_val, best_val, "row {row} argmin mismatch");
        }
    }

    fn brute_force_dp<F>(n: usize, cost: &F) -> Vec<u64>
    where
        F: Fn(usize, usize) -> u64,
    {
        let inf = u64::MAX / 4;
        let mut dp = vec![inf; n];
        if n == 0 {
            return dp;
        }
        dp[0] = 0;
        for row in 1..n {
            let mut best = inf;
            for (col, value) in dp.iter().enumerate().take(row) {
                let candidate = value + cost(row, col);
                if candidate < best {
                    best = candidate;
                }
            }
            dp[row] = best;
        }
        dp
    }

    #[test]
    fn monotone_minima_matches_bruteforce() {
        let rows = 16;
        let cols = 20;
        let cost = |i: usize, k: usize| {
            let diff = i.abs_diff(k) as u64;
            diff * diff
        };
        let got = monotone_minima(rows, cols, &cost);
        assert_row_minima(rows, cols, &cost, &got);
    }

    #[test]
    fn smawk_matches_bruteforce() {
        let rows = 18;
        let cols = 23;
        let cost = |i: usize, k: usize| {
            let diff = i.abs_diff(k) as u64;
            diff * diff
        };
        let got = smawk(rows, cols, &cost);
        assert_row_minima(rows, cols, &cost, &got);
    }

    #[test]
    fn simple_larsch_matches_bruteforce() {
        let n = 20;
        let cost = |i: usize, k: usize| {
            if k >= i {
                return u64::MAX / 4;
            }
            let diff = i.abs_diff(k) as u64;
            diff * diff
        };
        let expected = brute_force_dp(n, &cost);
        let got = simple_larsch_shortest_path(n, &cost);
        assert_eq!(got, expected);
    }

    #[test]
    fn simple_larsch_handles_two_nodes() {
        let n = 2;
        let cost = |row: usize, col: usize| {
            if col >= row {
                return u64::MAX / 4;
            }
            let diff = row.abs_diff(col) as u64;
            diff * diff
        };
        let expected = brute_force_dp(n, &cost);
        let got = simple_larsch_shortest_path(n, &cost);
        assert_eq!(got, expected);
    }

    #[test]
    fn larsch_matches_bruteforce() {
        let n = 22;
        let cost = move |i: usize, k: usize| {
            if k >= i {
                return u64::MAX / 4;
            }
            let diff = i.abs_diff(k) as u64;
            diff * diff
        };
        let expected = brute_force_dp(n, &cost);
        let got = larsch_shortest_path(n, cost);
        assert_eq!(got, expected);
    }
}
