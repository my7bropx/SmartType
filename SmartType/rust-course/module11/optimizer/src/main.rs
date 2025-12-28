use optimizer::{concat_naive, concat_optimized, measure};

fn main() {
    let parts: Vec<String> = (0..10_000).map(|i| format!("piece{i}")).collect();
    let refs: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();

    let (_, naive_time) = measure(|| concat_naive(&refs));
    let (_, opt_time) = measure(|| concat_optimized(&refs));

    println!(
        "Naive: {:?}\nOptimized: {:?}\nSpeedup ~{:.2}x",
        naive_time,
        opt_time,
        naive_time.as_secs_f64() / opt_time.as_secs_f64()
    );
}
