---
name: chefbar-qa
description: ChefBar QA worker — cargo fmt/clippy/test, shellcheck, visual-shot, CI workflow. Use after implementation, on CI red, or when editing scripts/** or .github/workflows. Runs gates; writes tests in already-touched modules or scripts/CI only.
---

# ChefBar QA

## Gates (must match CI)

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
shellcheck install.sh scripts/*.sh
```

Visual job (`scripts/visual-shot.sh`) is **warning-only** / exit 2 skip without Xvfb. Do not block merge on visual unless the user asked.

## Loop role

You are the `qa-converge` node. Report:

- pass/fail per command
- first failing test or clippy lint with file:line
- owning worker guess from `AGENTS.md`

Do not mass-rewrite modules to silence clippy. Prefer the owning worker. You may add/adjust `#[cfg(test)]` in files the parent already assigned, and you may edit `scripts/` and `.github/workflows/ci.yml`.

## Environment

Cargo exists in this cloud agent and on `chef-runner-01-1`. Do not tell anyone to rustup the laptop.
