# Paste into chefbar-rust prompts

- Read `Cargo.toml` first. Default: no new crate.
- Forbidden: tokio, async-std, reqwest, hyper, mockall, proptest, gtk4, webview, Electron.
- `ureq` stays `redirects(0)`.
- Exhaustive match on `RunSpec`, `UiCommand`, `HarnessKind`.
- Tests: `#[cfg(test)]` in the same file. Cloud/CI have cargo; laptop `joep` does not.
- Clippy: `cargo clippy --all-targets -- -D warnings`.
