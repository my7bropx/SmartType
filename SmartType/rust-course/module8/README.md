# Module 8 — Smart Pointers & Interior Mutability

Goal: handle shared ownership safely.

## What you learn
- `Arc<T>` for shared ownership across threads
- `Mutex<T>` and `RwLock<T>` for interior mutability and contention trade-offs
- When to clone Arcs vs. move data

## Project: In-Memory Database (`mem_db/`)
Simple shared key-value store using `Arc<RwLock<...>>` plus `Mutex` for stats.

### Usage
```bash
cargo run --manifest-path mem_db/Cargo.toml
```

### Testing
```bash
cargo test --manifest-path mem_db/Cargo.toml
```

### Notes
- Data guarded by `RwLock` for read-heavy workloads; stats use a `Mutex` for simplicity.
- `Database` is `Clone` (shares Arcs) so threads can operate concurrently.
- Tests include concurrent access to prove thread-safety patterns.
