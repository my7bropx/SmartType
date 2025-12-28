# Module 3 — Structs, Enums, Pattern Matching

Goal: model real problems with Rust's type system.

## What you learn
- Structs with fields and method organization via `impl` blocks
- Enums and `match`/pattern matching
- `Option<T>` and `Result<T, E>` control flow
- Destructuring with `if let` and `while let`
- Common derives like `Debug`, `Clone`, `Copy`, `Eq`, `Hash`

## Project: Password Strength Checker (`password_checker/`)
- Rules: length >= 12, lower, upper, digit, symbol
- Scoring: one point per rule; Strong (all 5), Medium (3-4), Weak (0-2)
- Exit codes: Strong = 0, Medium = 1, Weak = 2

### Usage
```bash
cargo run --manifest-path password_checker/Cargo.toml -- "GoodPass123!"
```

### Testing
```bash
cargo test --manifest-path password_checker/Cargo.toml
```

### Notes on design
- `Assessment` and `RuleResult` structs capture per-rule outcomes so you can explain results to users.
- The `Strength` enum drives both the report and the exit code, making it easy to wire into CI pipelines.
- Pattern matching on `Strength` in `exit_code` keeps the mapping explicit and testable.
