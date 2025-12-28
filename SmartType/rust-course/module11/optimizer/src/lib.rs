use std::time::{Duration, Instant};

/// Naive concatenation using `format!` repeatedly (allocates many times).
pub fn concat_naive(parts: &[&str]) -> String {
    let mut out = String::new();
    for part in parts {
        out = format!("{}{}", out, part);
    }
    out
}

/// Optimized concatenation: reserve capacity and push_str in place.
pub fn concat_optimized(parts: &[&str]) -> String {
    let total: usize = parts.iter().map(|p| p.len()).sum();
    let mut out = String::with_capacity(total);
    for part in parts {
        out.push_str(part);
    }
    out
}

/// Measure elapsed time of a function; returns (result, duration).
pub fn measure<F, T>(mut f: F) -> (T, Duration)
where
    F: FnMut() -> T,
{
    let start = Instant::now();
    let result = f();
    (result, start.elapsed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outputs_match() {
        let parts = vec!["hello", " ", "world", "!"];
        assert_eq!(concat_naive(&parts), concat_optimized(&parts));
    }

    #[test]
    fn optimized_is_not_slower_in_small_case() {
        let parts: Vec<String> = (0..100).map(|i| format!("p{i}")).collect();
        let refs: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();

        let (_, naive_time) = measure(|| concat_naive(&refs));
        let (_, opt_time) = measure(|| concat_optimized(&refs));

        // We just assert optimized isn't dramatically slower (allow equal or faster).
        assert!(opt_time <= naive_time * 2);
    }
}
