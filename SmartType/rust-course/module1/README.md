# Module 1 — Rust Basics

Goal: write clean basic programs confidently.

## What you learn
- Variables: `let`, `mut`, shadowing
- Scalar and compound types
- Functions and the `->` return arrow (expressions vs. statements)
- Control flow: `if`, `loop`, `while`, `for`
- Strings (`String` vs `&str`) and `Vec`

## Exercises implemented
- Temperature converter (Celsius/Fahrenheit parsing and conversion)
- Fibonacci (iterative + recursive helpers)
- Word counter (split string; file-based CLI)

## Project: CLI Toolbox
Code lives in `toolbox/` and exposes three subcommands:

```bash
# Fibonacci
cargo run --bin toolbox --manifest-path toolbox/Cargo.toml -- fib 10

# Temperature conversion (case-insensitive unit)
cargo run --bin toolbox --manifest-path toolbox/Cargo.toml -- temp 32F

# Word count from a file
cargo run --bin toolbox --manifest-path toolbox/Cargo.toml -- wc ./some.txt
```

### Code layout
- `src/fib.rs`: iterative and recursive Fibonacci plus input parsing.
- `src/temp.rs`: temperature parsing/conversion with nice error messages.
- `src/wc.rs`: word counting for strings and files.
- `src/main.rs`: lightweight CLI router; prints usage and friendly errors.

### How to test & format
```bash
cargo fmt --manifest-path toolbox/Cargo.toml
cargo test --manifest-path toolbox/Cargo.toml
```

Use this folder for solutions, notes, and further experiments.
