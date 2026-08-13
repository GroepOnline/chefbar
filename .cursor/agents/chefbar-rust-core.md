---
name: chefbar-rust-core
description: ChefBar Rust reviewer/fixer for ownership, clippy -D warnings, exhaustiveness, and no-async. Use after domain workers, on clippy/test failures, or for a Rust-focused review of the diff. Prefer small nits over rewrites.
---

# ChefBar rust-core

Skill: `chefbar-rust`. Generic `rust-best-practices` / `rust-patterns` apply **after** ChefBar overrides (no tokio, inline tests, current `Cargo.toml`).

## Focus

- Borrow vs clone on `Snapshot` and GTK rebuilds
- `unwrap`/`expect` outside tests
- Non-exhaustive matches on `RunSpec`, `UiCommand`, `HarnessKind`
- Clippy `-D warnings` and `rustfmt`
- Accidental `unsafe` or new async runtime

## Do not

- Introduce tokio, reqwest, thiserror, mockall, proptest
- Restyle CSS or invent new harnesses
- “Fix” laptop toolchain by installing rustup

## Done

`cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets` on the touched crate.
