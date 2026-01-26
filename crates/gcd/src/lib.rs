pub fn gcd_euclid(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

pub fn gcd_binary(mut a: u64, mut b: u64) -> u64 {
    if a == 0 {
        return b;
    }
    if b == 0 {
        return a;
    }

    let shift = (a | b).trailing_zeros();
    a >>= a.trailing_zeros();

    loop {
        b >>= b.trailing_zeros();
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        b -= a;
        if b == 0 {
            return a << shift;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcd_known_cases() {
        let cases = [
            (0_u64, 0_u64, 0_u64),
            (0, 18, 18),
            (18, 0, 18),
            (54, 24, 6),
            (48, 180, 12),
            (17, 13, 1),
            (4096, 256, 256),
        ];

        for (a, b, expected) in cases {
            assert_eq!(gcd_euclid(a, b), expected);
            assert_eq!(gcd_binary(a, b), expected);
        }
    }

    #[test]
    fn gcd_impls_agree() {
        let pairs = [
            (1_u64, 1_u64),
            (2, 3),
            (7, 14),
            (25, 100),
            (81, 153),
            (9_699, 3_231),
        ];

        for (a, b) in pairs {
            let euclid = gcd_euclid(a, b);
            assert_eq!(gcd_binary(a, b), euclid);
        }
    }
}
