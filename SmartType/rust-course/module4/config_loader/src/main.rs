use std::env;
use std::process;

use config_loader::{exit_code_for_error, load_config};

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: config_loader <config_file>");
        process::exit(2);
    });

    match load_config(&path) {
        Ok(cfg) => {
            println!("Loaded config:");
            println!("- host: {}", cfg.host);
            println!("- port: {}", cfg.port);
            println!("- debug: {}", cfg.debug);
            process::exit(0);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(exit_code_for_error(&e));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    fn write_temp_file(content: &str) -> PathBuf {
        let mut path = env::temp_dir();
        path.push("config_cli_test.txt");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "{}", content).unwrap();
        path
    }

    #[test]
    fn loads_config_and_exits_zero() {
        let path = write_temp_file("host=localhost\nport=8080\n");
        let cfg = load_config(path.to_str().unwrap()).unwrap();
        assert_eq!(cfg.port, 8080);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn returns_error_code_on_missing() {
        let path = write_temp_file("host=localhost\n");
        let err = load_config(path.to_str().unwrap()).unwrap_err();
        assert_eq!(exit_code_for_error(&err), 2);
        let _ = std::fs::remove_file(path);
    }
}
