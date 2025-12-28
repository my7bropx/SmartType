# Module 10 — Async Rust (Foundations)

Goal: build intuition for async-style concurrency using lightweight tasks.

## What you learn
- Structuring work as tasks with limited depth and queues
- Using scoped threads as a stand-in for async executors when external crates are unavailable
- Handling task results and errors via channels

## Project: Async-Style File Crawler (`async_crawler/`)
- Crawls text files, following `link:<path>` markers up to a depth limit
- Spawns worker threads to process files concurrently and reports discovered links

### Usage
```bash
cargo run --manifest-path async_crawler/Cargo.toml -- ./seed.txt 2
```

### Testing
```bash
cargo test --manifest-path async_crawler/Cargo.toml
```

### Notes
- Uses channels and scoped threads to mimic async task scheduling without external dependencies.
- Depth limiting prevents runaway traversals; visited-set avoids duplicate work.
- Swap in a real async runtime (Tokio) later by replacing the worker loop with spawned async tasks.
