---
name: chefbar-rust
description: ChefBar Rust skill for this crate only. Use when writing, reviewing, refactoring, or testing Rust in chefbar; when clippy -D warnings fail; when ownership crosses Snapshot and GTK; when something suggests tokio, async, reqwest, mockall, or unwrap outside tests. Overrides rust-patterns, rust-testing, and rust-best-practices on stack and concurrency. Covers Cargo.toml gtk 0.18 ksni ureq clap, inline cfg(test) modules, exhaustive RunSpec matches.
---

# ChefBar Rust

Ecosysteem-skills in `.agents/skills/` (`rust-best-practices`, `rust-patterns`, `rust-testing`) mogen ownership en `Result` leren. **Deze skill wint** op stack, tests, en concurrency.

## Instructions

1. Read `Cargo.toml`. If the change needs a crate that is not already there, stop and justify it in the plan — default is **no new crate**.
2. Keep the process two-world: actor thread (`state.rs`, blocking ureq) and GTK thread (widgets, `Rc`/`RefCell`). `Snapshot` crosses via `Arc<RwLock<_>>`. Widgets are not `Send`.
3. Prefer `&str` / `&[T]` / borrowed snapshot fields. Clone at the GTK/snapshot boundary when you must own a row for a widget, not in a ranking loop.
4. Fallible work returns `Result`. Vault/ops JSON is tolerant (`Option`/`unwrap_or_default`), never `unwrap` on parse in production.
5. Match `RunSpec`, `UiCommand`, `HarnessKind`, `HarnessGroup`, `ActorCommand`, `DomainStatus` exhaustively. A `_` arm hides the next domain.
6. Tests go in `#[cfg(test)]` in the same file as the code. Name them after behavior (`coalesce_toasts_caps_one_per_cycle`), not methods.
7. Before handing off: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets` in this cloud agent or CI. Never tell the laptop `joep` to install rustup.

### Allowed stack

`gtk 0.18` (feature `v3_24`), `ksni 0.2`, `ureq 2` (`json`), `clap 4`, `url 2`, `serde`/`serde_json`, `dirs`, `sha2`, `libc`, `pango`, `gdk`. Edition 2021. Release: `lto`, `strip`.

### Forbidden unless the user explicitly changes Cargo.toml

tokio, async-std, reqwest, hyper, thiserror, anyhow, mockall, proptest, rstest, criterion, gtk4, relm4, tauri, wry, WebKit.

`ureq` must keep `redirects(0)` — bearers never follow 302.

### Concurrency that already exists (do not “clean up”)

- `Poller::fetch_all` / `fanout` spawn short GET threads inside an 8s budget. That is not a second actor.
- `Executor::spawn_bg` is one-shot. Do not turn it into `loop { sleep; poll }`.
- Tray has its own ksni thread and only sends `UiCommand`.

## Examples

**Example 1 — clippy redundant_clone**

Input: clone of `String` in `rank_actions_with`

Output: rank by index/`&Action`, clone only the `take(limit)` winners (already the pattern). Do not collect owned `Action` before sort.

**Example 2 — rust-testing says add mockall**

Input: “unit-test Executor::run without HTTP”

Output: extract a tiny function that builds the `json!` body and assert on that; or test `RunSpec` construction. Do not add mockall. Do not hit the network.

**Example 3 — unwrap in doctor**

Input: parse fingerprint

Output: `sha2` digest → hex truncate 12. On missing token, skip with a warn line, do not `expect`.

## Performance Notes

- Palette ranking is O(actions × needle). Keep haystack formatting cheap; do not clone `RunSpec` unless returning winners.
- Snapshot write-lock is held only while swapping the new struct. UI takes a short read-lock and clones the rows it paints.
- Clippy `perf` lints are CI-blocking because `-D warnings`.
- Ignore ecosysteem chapters on llvm-cov 80% gates and tokio::test.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| `cannot Send gtk::Label` | work stayed on GTK thread; send `UiCommand` instead |
| clippy fail in CI, pass local | CI uses `-D warnings`; run the same flags |
| agent added tokio “for linear poll” | point at `LINEAR_POLL_MS`; revert Cargo.toml |
| laptop user asks for rustup | `CONTRIBUTING.md` — runner / this cloud agent only |

## Invariants to paste

See [references/invariants.md](references/invariants.md). Short form:

- `Cargo.toml` allowed stack only. Clippy `-D warnings`. Inline `#[cfg(test)]`.
- Exhaustive `RunSpec` / `UiCommand` / `HarnessKind`. No `unwrap` in production.
- No tokio, reqwest, webview, Electron, mockall, or proptest.

## Next

Actor/snapshot → `chefbar-actor`. GTK types → `chefbar-gtk-panel`. Review diff → agent `chefbar-rust-core`.

Copy [references/invariants.md](references/invariants.md) into worker prompts. No tokio, reqwest, webview, or Electron unless the user explicitly changes `Cargo.toml`.
