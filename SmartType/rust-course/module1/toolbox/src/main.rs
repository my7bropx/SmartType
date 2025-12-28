mod fib;
mod temp;
mod wc;

use std::env;
use std::io;

use fib::{fib_iter, fib_rec, parse_n};
use temp::Temperature;
use wc::count_words_in_file;

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  toolbox fib <n>");
    eprintln!("  toolbox temp <value><C|F>");
    eprintln!("  toolbox wc <file>");
}

fn run_fib(args: &[String]) -> Result<(), String> {
    let n_arg = args.get(0).ok_or_else(|| "missing n for fib".to_string())?;
    let n = parse_n(n_arg).map_err(|e| format!("invalid n: {e}"))?;

    if n > 93 {
        return Err("n too large for u64 fibonacci (max 93)".to_string());
    }

    let iterative = fib_iter(n);
    let recursive = if n <= 40 { Some(fib_rec(n)) } else { None };

    println!("n = {n}");
    println!("iterative: {iterative}");
    if let Some(rec_val) = recursive {
        println!("recursive: {rec_val}");
    } else {
        println!("recursive: skipped (n too large for demo)");
    }

    Ok(())
}

fn run_temp(args: &[String]) -> Result<(), String> {
    let input = args
        .get(0)
        .ok_or_else(|| "missing temperature value".to_string())?;
    let parsed = Temperature::parse(input).map_err(|e| e.to_string())?;

    let c = parsed.to_celsius();
    let f = parsed.to_fahrenheit();

    println!("Input: {parsed}");
    println!("As Celsius: {c}");
    println!("As Fahrenheit: {f}");

    Ok(())
}

fn run_wc(args: &[String]) -> Result<(), String> {
    let path = args.get(0).ok_or_else(|| "missing file path".to_string())?;
    let count = count_words_in_file(path).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => format!("file not found: {path}"),
        _ => format!("failed to read {path}: {e}"),
    })?;

    println!("Word count: {count}");
    Ok(())
}

fn main() {
    let mut args: Vec<String> = env::args().collect();
    // Drop binary name
    if !args.is_empty() {
        args.remove(0);
    }

    let Some(cmd) = args.get(0).cloned() else {
        print_usage();
        return;
    };
    // Drop the command name
    args.remove(0);

    let result = match cmd.as_str() {
        "fib" => run_fib(&args),
        "temp" => run_temp(&args),
        "wc" => run_wc(&args),
        _ => {
            print_usage();
            Err(format!("unknown command: {cmd}"))
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod integration_like_tests {
    use super::*;

    #[test]
    fn run_fib_works() {
        run_fib(&["10".to_string()]).unwrap();
        // Should skip recursive for large n but within safe numeric bounds
        run_fib(&["60".to_string()]).unwrap();
    }

    #[test]
    fn run_fib_rejects_too_large_input() {
        let err = run_fib(&["100".to_string()]).unwrap_err();
        assert!(err.contains("too large"));
    }

    #[test]
    fn run_temp_works() {
        run_temp(&["32F".to_string()]).unwrap();
        run_temp(&["0C".to_string()]).unwrap();
    }

    #[test]
    fn run_wc_errors_on_missing() {
        let err = run_wc(&[]).unwrap_err();
        assert!(err.contains("missing file path"));
    }
}
