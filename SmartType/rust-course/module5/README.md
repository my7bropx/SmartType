# Module 5 — Traits, Generics, Iterators

Goal: unlock Rust's power with reusable abstractions.

## What you learn
- Generics and trait bounds (`T: Display + Clone`)
- Implementing and consuming traits
- Iterator patterns (`map`, `filter`, `fold`, `collect`)
- Writing your own iterators and generic data structures

## Exercises implemented (`stats_tool/`)
- Mean/median/mode helpers for slices of numbers
- Generic `Stack<T>` with push/pop/peek helpers
- `Summarize` trait with implementations for multiple types (`Article`, `Tweet`)

### Usage
```bash
cargo run --manifest-path stats_tool/Cargo.toml
```

### Testing
```bash
cargo test --manifest-path stats_tool/Cargo.toml
```

### Notes
- `Stack<T>` owns its items and exposes references via `peek` to avoid unnecessary moves.
- `Summarize` demonstrates trait objects (`Box<dyn Summarize>`) in the binary and concrete impls in the library.
- Mean/median/mode show collection processing with iteration, sorting, and frequency counting.
