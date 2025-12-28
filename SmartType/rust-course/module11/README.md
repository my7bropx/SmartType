# Module 11 — Performance & Memory

Goal: write fast Rust without guesswork.

## Topics
- Allocations and growth of `String`/`Vec`.
- Borrowing to avoid unnecessary clones.
- Using `Cow` effectively.
- `serde` costs.
- When to use `bytes`, `smallvec`, and similar crates (optional).

## Project: Optimize Earlier Work
- Tuned the Module 9 parallel scanner: deterministic FNV-1a hashing and buffered reads to avoid loading whole files.
- Parallel hashing uses scoped threads without external crates; worker count capped by CPU availability.
- Next steps: profile with `cargo bench` or `perf` when available; compare with streaming SHA-256 if crypto is required.

Record benchmarks, profiles, and lessons here.

## Additional demo: `optimizer/`
- Shows naive vs optimized string concatenation with capacity pre-allocation.
- Includes a tiny timing helper to compare approaches (for educational baselines only).
- CLI prints naive vs optimized durations on a larger input to illustrate the impact of allocations.
