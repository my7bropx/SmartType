# Module 6 — Modules, Crates, Workspaces

Goal: understand Rust's module system and crate boundaries.

## What you learn
- `mod`, `pub`, `use` for visibility and structure
- `lib.rs` vs. `main.rs`
- Crate boundaries and sharing code via workspaces
- Binary crates depending on library crates

## Project: Workspace Tool (`workspace_tool/`)
Workspace with a core library and a CLI binary:
- `corelib` (library): provides helpers like `greet` and `add`.
- `cli` (binary): uses `clap` to expose subcommands that call into `corelib`.

### Usage
```bash
# Run from workspace root
cargo run --manifest-path workspace_tool/cli/Cargo.toml -- greet Ferris
cargo run --manifest-path workspace_tool/cli/Cargo.toml -- add 2 3
```

### Testing
```bash
cargo test --manifest-path workspace_tool/corelib/Cargo.toml
cargo test --manifest-path workspace_tool/cli/Cargo.toml
```

### Notes
- Workspace root `Cargo.toml` pins resolver 3 and lists members.
- Library crate exports functions; CLI crate depends on it via a path dependency.
- Subcommands demonstrate argument parsing and reuse of shared logic.
