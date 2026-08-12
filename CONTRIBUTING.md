# Contributing — ChefBar

## Harde regel: geen Rust-builds op de laptop

De laptop `joep` heeft **bewust geen Rust-toolchain** sinds 2026-08-12.
`cargo`, `rustc`, `rustup`, `rustfmt`, `clippy-driver`, `rust-analyzer` en
`cargo-*` zijn fail-fast stubs in `~/.local/bin` die met een foutmelding en
runner-verwijzing stoppen (exit 1). Zie `~/AGENTS.md`.

Dat is geen administratieve regel maar een technische: de stubs bestaan en de
toolchain is verwijderd (6.7G vrijgemaakt). Er is niets om mee te bouwen.

## Waar bouwen

Alles op de runner (`chef-runner-01-1`, user `chef`, cargo aanwezig):

```bash
# checkout bijwerken
ssh chef@chef-runner-01-1 'cd ~/chefbar-check && git pull --rebase'

# testen + build
ssh chef@chef-runner-01-1 'cd ~/chefbar-check && export PATH=$HOME/.cargo/bin:$PATH && cargo test --all-targets && cargo build --release'

# binary ophalen
scp chef@chef-runner-01-1:/home/chef/chefbar-check/target/release/chefbar /tmp/chefbar.bin
./install.sh --systemd /tmp/chefbar.bin
```

CI is notify-first (geen poll-loops) en draait op de self-hosted runner;
release-artifact heet `chefbar-release`.

## Gates

- `cargo fmt --check` en `cargo clippy --all-targets -- -D warnings` draaien in
  CI als harde checks — lokaal voorlopen kan alleen op de runner.
- PR's naar `main` worden squash-gemerged; geen force-push naar `main`.
- Wijzigingen aan `install.sh` of systemd-units: `shellcheck` + dry-run in CI.

## ChefApp 4.0 lanes — file-disjoint

`feat/chefapp-4.0` draait 7 lanes parallel naar één stack. Regels:

- **File-disjoint:** elke lane raakt alleen zijn eigen matrix (zie `docs/plan-full-chefapp.md` §6.2). Lane G raakt alleen `scripts/**`, `.github/workflows/ci.yml`, `docs/**`, `README.md`, `CONTRIBUTING.md`, `Cargo.toml` (dev-deps/scripts).
- **Worktree per lane:** `git worktree add ~/ChefFactory/chefbar-worktrees/chefapp-<x> feat/chefapp-4.0-lane-<x>`.
- **No-local-build blijft:** build/test alleen op `chef@chef-runner-01-1` (zie boven).
- **Merge-train:** `A → F,G → B → C,D,E`, squash naar `feat/chefapp-4.0`, dan squash naar `main`. Lane G (tooling) hoeft niet te wachten — schrijft alleen scripts/docs.

## Conventional commits

`feat:`, `fix:`, `docs:`, `test:`, `chore:` met scope, bijv.
`fix(tray): statuslijn sorteert nieuwste eerst binnen priority-groep`.
