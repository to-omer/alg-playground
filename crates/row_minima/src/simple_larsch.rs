pub fn simple_larsch_shortest_path<F>(n: usize, cost: &F) -> Vec<u64>
where
    F: Fn(usize, usize) -> u64,
{
    if n == 0 {
        return Vec::new();
    }

    let inf = u64::MAX / 4;
    let mut dist = vec![inf; n];
    dist[0] = 0;
    let mut argmins = vec![0; n];

    fn update<F>(row: usize, col: usize, cost: &F, dist: &mut [u64], argmins: &mut [usize])
    where
        F: Fn(usize, usize) -> u64,
    {
        if row <= col {
            return;
        }
        let candidate = dist[col] + cost(row, col);
        if candidate < dist[row] {
            dist[row] = candidate;
            argmins[row] = col;
        }
    }

    fn update_unchecked<F>(
        row: usize,
        col: usize,
        cost: &F,
        dist: &mut [u64],
        argmins: &mut [usize],
    ) where
        F: Fn(usize, usize) -> u64,
    {
        let candidate = dist[col] + cost(row, col);
        if candidate < dist[row] {
            dist[row] = candidate;
            argmins[row] = col;
        }
    }

    fn dfs<F>(l: usize, r: usize, cost: &F, dist: &mut [u64], argmins: &mut [usize])
    where
        F: Fn(usize, usize) -> u64,
    {
        if r == l + 1 {
            update_unchecked(r, l, cost, dist, argmins);
            return;
        }
        if r <= l + 1 {
            return;
        }
        let m = (l + r) / 2;
        let left = argmins[l];
        let right = argmins[r];
        for col in left..=right {
            update(m, col, cost, dist, argmins);
        }
        dfs(l, m, cost, dist, argmins);
        for col in l + 1..=m {
            update_unchecked(r, col, cost, dist, argmins);
        }
        dfs(m, r, cost, dist, argmins);
    }

    if n > 1 {
        dfs(0, n - 1, cost, &mut dist, &mut argmins);
    }
    dist
}
