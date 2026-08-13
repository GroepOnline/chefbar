# CI / local gates

Must match `.github/workflows/ci.yml` job `build` plus the harness step.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
shellcheck install.sh scripts/*.sh
node scripts/agent-bench.mjs
```

Visual job `visual` is warning-only (`continue-on-error`). `scripts/visual-shot.sh` exit 2 = no Xvfb.

Doctor (not always in CI): exit 0 ok, 1 warn, 2 error. Fingerprints `sha256[:12]` only.

Laptop `joep`: no Rust toolchain. Cloud + `chef-runner-01-1`: cargo present.
