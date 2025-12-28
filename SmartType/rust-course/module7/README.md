# Module 7 — Testing, Benchmarking, Tooling

Goal: write stable code that survives change.

## Focus areas covered
- Unit tests and integration tests
- Designing code for testability (pure helpers, deterministic paths)
- Capturing CLI behavior through library-level contracts

## Hardened so far
- Added integration tests for `module2/rsgrep` to exercise case-sensitive/insensitive searches, line numbers, and missing-file errors.

## How to run
```bash
cargo test --manifest-path ../module2/rsgrep/Cargo.toml
```

Add more integration tests for other modules as you extend the toolbox.
