# Module 12 — Unsafe Rust & FFI

Goal: understand unsafe Rust well enough to avoid pitfalls.

## What you learn
- Calling C functions from Rust with `extern "C"`
- Handling raw pointers and ownership boundaries
- When to wrap unsafe code in safe abstractions

## Project: Minimal FFI Wrapper (`minimal_ffi/`)
- Wraps C's `strlen` and demonstrates copying Rust data into C-allocated memory with explicit free.
- Shows where `unsafe` is required and how to provide safe wrappers.

### Usage
```bash
cargo run --manifest-path minimal_ffi/Cargo.toml
```

### Testing
```bash
cargo test --manifest-path minimal_ffi/Cargo.toml
```

### Notes
- `to_c_buffer` returns a raw pointer and length; the caller must free via `c_free`.
- `unsafe` is contained within small helpers; public functions remain safe to call with valid inputs.
- Avoid interior null bytes when using `c_strlen` (uses `CString::new`).
