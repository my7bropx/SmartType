# Module 9 — Concurrency

Goal: use threads without pain.

## What you learn
- `std::thread` and scoped spawning (covered conceptually here)
- Work stealing with `rayon` for data parallelism
- Channels and shared state patterns (to add later)

## Project: Parallel File Scanner (`parallel_scanner/`)
- Walks a directory tree and hashes files in parallel (SHA-256).
- Prints `<hash>  <path>` for each file.

### Usage
```bash
cargo run --manifest-path parallel_scanner/Cargo.toml -- ./some_dir
```

### Testing
```bash
cargo test --manifest-path parallel_scanner/Cargo.toml
```

### Notes
- Manual directory walk (stack-based) and parallel hashing via scoped threads with the number of workers capped by available CPUs.
- Uses a simple `DefaultHasher`-based hash to avoid external crypto crates (swap to SHA-256 later if desired).
- Errors propagate with `io::Result`, stopping on first failure to keep behavior predictable.
- Swap in channels or progress reporting later to explore coordination patterns.
