# Module 2 — Ownership, Borrowing, Lifetimes

Goal: understand Rust's ownership model and eliminate confusion.

## What you learn
- Move semantics, copy types, and cloning
- Borrowing with `&T` and `&mut T`
- Borrowing rules: one mutable or many immutable references
- Slices: `&[T]` and `&str`
- Lifetimes: when they are needed and how to read them

## Drills
- Longest word finder
- Safe split-at implementation
- Borrow-checker fix practice

## Project: `rsgrep`
A mini ripgrep that searches files with optional case-insensitive matching.

### Usage
```bash
cargo run --manifest-path rsgrep/Cargo.toml -- "needle" ./some.txt
cargo run --manifest-path rsgrep/Cargo.toml -- "needle" ./some.txt --ignore-case
```

### Notes on ownership & borrowing
- `Config::from_args` borrows the iterator lazily and consumes it to build owned `String` fields.
- `search_in_reader` borrows a `BufRead` and returns owned `Match` records so callers can move them freely.
- The matcher logic avoids allocations in the case-sensitive path and only allocates when `--ignore-case` is set.

Run tests:
```bash
cargo test --manifest-path rsgrep/Cargo.toml
```
