---
name: chefbar-rust
description: ChefBar-specific Rust skill for this crate. Use when writing, reviewing, refactoring, or testing Rust in chefbar. Overrides generic rust-patterns/rust-testing/rust-best-practices on tokio, async, mockall, extra crates, and laptop toolchain. Use for clippy -D warnings, ownership across Snapshot/GTK, and inline #[cfg(test)] modules.
---

# ChefBar Rust

Generic skills in `.agents/skills/` (`rust-best-practices`, `rust-patterns`, `rust-testing`) are useful for ownership and `Result`. This skill **wins** on stack, tests, and concurrency.

## Stack (do not expand casually)

`gtk 0.18` (v3_24), `ksni 0.2`, `ureq 2` (`json`, redirects 0), `clap 4`, `url 2`, `serde`/`serde_json`, `dirs`, `sha2`, `libc`, `pango`, `gdk`. Edition 2021. Release: `lto = true`, `strip = true`.

Do **not** add: tokio, async-std, reqwest, hyper, thiserror, anyhow, mockall, proptest, rstest, criterion, gtk4, relm4, tauri, wry.

## Concurrency

The process is two worlds:

1. **Actor thread** — `state.rs`, blocking `ureq`, publishes into `Arc<RwLock<Snapshot>>`.
2. **GTK main thread** — panel/tray bridge. Talks to the actor via `mpsc` (`ActorCommand`, `UiCommand`).

GTK types are not `Send`. Do not move widgets onto the actor. Do not take a write-lock on `Snapshot` on the UI thread for long; clone the small bits you need.

`Arc<Mutex<…>>` is already used for tray bits. Prefer existing `Shared` over a new lock.

## Errors and parsing

- Network: `http::ApiError`, never unwrap.
- Config/JSON: degrade to defaults (`models.rs` comment: tolerant parse, never panic).
- CLI: clap + explicit `process::exit` codes (doctor 0/1/2, ipc unknown = 2).

## Tests

Existing tests are **inline** `#[cfg(test)]` in the module under test (`actions`, `config`, `palette`, `models`, `motion`, `harness`, `ipc`, `policy`, `sessions`, `panel_state`, …).

Keep that pattern:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesce_toasts_caps_one_per_cycle() {
        // behavior name, not method name
    }
}
```

Ignore `rust-testing` advice about `tests/`, tokio::test, mockall, proptest, and 80% llvm-cov gates unless the user asks to introduce them.

Helpers: build small `Snapshot` / `EndpointProfile` fixtures in the test module. Do not hit the network.

## Clippy / fmt

CI: `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings`. Match that locally in this cloud environment.

Prefer `#[expect(clippy::lint)]` with a reason over silent `#[allow]`.

## Exhaustiveness

When matching `RunSpec`, `UiCommand`, `HarnessKind`, `HarnessGroup`, `ActorCommand`, `DomainStatus`, list every variant. A `_` arm hides the next domain.

## Toolchain

| Where | cargo |
| --- | --- |
| This cloud agent | yes (`/usr/local/cargo`) |
| CI self-hosted runner | yes |
| Laptop `joep` | **no** — stubs, see `CONTRIBUTING.md` |

Never instruct `rustup` on the laptop.

## Review checklist

- No new thread that polls
- No clone in a hot GTK rebuild unless owned data is required
- No `unwrap` outside tests
- Keywords on new actions include harness prefixes
- Dutch module docs, English identifiers
