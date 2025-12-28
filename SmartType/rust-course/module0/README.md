# Module 0 — Setup & Mindset

Goal: get tooling right and build confidence with the compiler.

## Checklist
- Install Rust toolchain: `rustup`, `cargo`, `rust-analyzer`, `clippy`, and `rustfmt`.
- Practice essential commands: `cargo new`, `cargo run`, `cargo test`, `cargo fmt`, `cargo clippy`, `cargo doc --open`, `rustup update`.
- Create a small program to confirm everything works.

## Mini Task: `hello_rust`
- Prints your name, the current operating system, and the arguments passed to the program.
- Includes `RunInfo` helpers so the logic is tested and easy to extend.

Run it with custom args:
```bash
cargo run --manifest-path hello_rust/Cargo.toml -- first second
```

Tests keep the formatter/output in check:
```bash
cargo test --manifest-path hello_rust/Cargo.toml
```

Use this folder for notes about installation steps and any friction you encounter with the tooling.
