use std::env;
use std::process;

use rsgrep::{Config, run};

fn main() {
    let config = Config::from_args(env::args()).unwrap_or_else(|err| {
        eprintln!("Error: {err}");
        print_usage();
        process::exit(1);
    });

    if let Err(err) = run(config) {
        eprintln!("Application error: {err}");
        process::exit(1);
    }
}

fn print_usage() {
    eprintln!("Usage: rsgrep <query> <file> [--ignore-case|-i]");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_usage_on_error() {
        let result = Config::from_args(vec!["rsgrep".into()].into_iter());
        assert!(result.is_err());
    }
}
