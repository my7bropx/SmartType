use std::num::ParseIntError;

/// Parse a Fibonacci input `n` from string to u64.
pub fn parse_n(input: &str) -> Result<u64, ParseIntError> {
    input.trim().parse::<u64>()
}

/// Iterative Fibonacci using constant space.
pub fn fib_iter(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => {
            let mut prev = 0;
            let mut curr = 1;
            for _ in 2..=n {
                let next = prev + curr;
                prev = curr;
                curr = next;
            }
            curr
        }
    }
}

/// Recursive Fibonacci for learning purposes (exponential time!).
pub fn fib_rec(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fib_rec(n - 1) + fib_rec(n - 2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_n() {
        assert_eq!(parse_n("8").unwrap(), 8);
        assert!(parse_n("not_a_number").is_err());
    }

    #[test]
    fn computes_iterative() {
        assert_eq!(fib_iter(0), 0);
        assert_eq!(fib_iter(1), 1);
        assert_eq!(fib_iter(8), 21);
    }

    #[test]
    fn computes_recursive() {
        assert_eq!(fib_rec(0), 0);
        assert_eq!(fib_rec(1), 1);
        assert_eq!(fib_rec(6), 8);
    }
}
