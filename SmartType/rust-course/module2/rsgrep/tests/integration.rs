use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rsgrep::{search_file, Config};

fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    path.push(format!("{name}_{nanos}.txt"));
    path
}

fn write_temp_file(contents: &str) -> PathBuf {
    let path = temp_path("rsgrep_integration");
    let mut file = File::create(&path).unwrap();
    write!(file, "{}", contents).unwrap();
    path
}

#[test]
fn finds_case_sensitive_matches_with_line_numbers() {
    let path = write_temp_file("Rust is great\nTrust the process\n");
    let cfg = Config {
        query: "Rust".into(),
        filename: path.to_string_lossy().into_owned(),
        ignore_case: false,
    };

    let results = search_file(&cfg).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].line_number, 1);

    let _ = std::fs::remove_file(path);
}

#[test]
fn finds_ignore_case_matches() {
    let path = write_temp_file("rustacean\nFerris\nRUST rules\n");
    let cfg = Config {
        query: "rust".into(),
        filename: path.to_string_lossy().into_owned(),
        ignore_case: true,
    };

    let results = search_file(&cfg).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[1].line_number, 3);

    let _ = std::fs::remove_file(path);
}

#[test]
fn errors_on_missing_file() {
    let cfg = Config {
        query: "needle".into(),
        filename: "/definitely/not/here.txt".into(),
        ignore_case: false,
    };

    let err = search_file(&cfg).unwrap_err();
    assert!(err
        .to_string()
        .contains("No such file") || err.to_string().contains("not found"));
}
