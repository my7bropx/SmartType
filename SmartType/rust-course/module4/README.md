# Module 4 — Error Handling & API Design

Goal: write professional Rust with predictable failures and clean APIs.

## What you learn
- `Result` flow with `?`, `map`, and `and_then`
- Custom error types (manual Display + Error impl here; `thiserror` later)
- When to panic vs. return structured errors
- Friendly CLI error handling with exit codes

## Project: Config Loader (`config_loader/`)
Reads a simple `key=value` config file, applies environment overrides, validates fields, and prints the final config.

### Usage
```bash
cargo run --manifest-path config_loader/Cargo.toml -- ./example.conf
# Overrides via environment (optional):
APP_HOST=example.com APP_PORT=9090 APP_DEBUG=yes cargo run --manifest-path config_loader/Cargo.toml -- ./example.conf
```

### Exit codes
- 0: success
- 1: I/O error
- 2: validation or missing-key error

### Testing
```bash
cargo test --manifest-path config_loader/Cargo.toml
```

### Notes on design
- `ConfigError` keeps validation concerns explicit (missing key, bad port, bad bool, I/O).
- `exit_code_for_error` maps errors to exit codes so the CLI can communicate failures clearly.
- The loader returns owned `AppConfig` data, making it easy to pass across threads or store.
