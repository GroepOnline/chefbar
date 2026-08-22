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

CI is notify-first (geen poll-loops). Full lane = self-hosted GHA
(`.github/workflows/ci.yml`, artifact `chefbar-release`). Optionele snelle
Rust check/fmt/clippy/test = Buildkite `onlinechef/chefbar` (`.buildkite/`;
niet required — zie `.buildkite/README.md`).

### GHA host-deps (aws-chefbar-compat)

PR CI landt op self-hosted labels `pr-isolated` / `heavy` (compat-host
`aws-chefbar-compat`). De Rust GTK-build heeft **host** packages nodig:

- `pkg-config`
- `libgtk-3-dev`
- `libglib2.0-dev`

Zonder die packages faalt `glib-sys` met “The pkg-config command could not be
found” na een nutteloze toolchain-warmup. CI heeft daarom een **Host deps
preflight** vóór `cargo check`. De runner-service draait met
`NoNewPrivileges=true`, dus de workflow mag **geen** `sudo apt` doen — packages
installeren op de host zelf (als root/operator), niet in de job.

## Gates

- `cargo fmt --check` en `cargo clippy --all-targets -- -D warnings` draaien in
  CI als harde checks — lokaal voorlopen kan alleen op de runner.
- PR's naar `main` worden squash-gemerged; geen force-push naar `main`.
- Wijzigingen aan `install.sh`, `.cursor/*.sh` of systemd-units: `shellcheck` + dry-run in CI.

## ChefApp 4.0 lanes — file-disjoint

`feat/chefapp-4.0` draait 7 lanes parallel naar één stack. Regels:

- **File-disjoint:** elke lane raakt alleen zijn eigen matrix (zie `docs/plan-full-chefapp.md` §6.2). Lane G raakt alleen `scripts/**`, `.github/workflows/ci.yml`, `docs/**`, `README.md`, `CONTRIBUTING.md`, `Cargo.toml` (dev-deps/scripts).
- **Worktree per lane:** `git worktree add ~/ChefFactory/chefbar-worktrees/chefapp-<x> feat/chefapp-4.0-lane-<x>`.
- **No-local-build blijft:** build/test alleen op `chef@chef-runner-01-1` (zie boven).
- **Merge-train:** `A → F,G → B → C,D,E`, squash naar `feat/chefapp-4.0`, dan squash naar `main`. Lane G (tooling) hoeft niet te wachten — schrijft alleen scripts/docs.

## Conventional commits

`feat:`, `fix:`, `docs:`, `test:`, `chore:` met scope, bijv.
`fix(tray): statuslijn sorteert nieuwste eerst binnen priority-groep`.

## ChefApp 5.0 lane G — tooling & docs

Lane G blijft file-disjoint: wijzigingen zijn beperkt tot `scripts/**`, `.github/workflows/ci.yml`, `docs/**`, `README.md`, `CONTRIBUTING.md`, `Cargo.toml` (alleen dev-deps/scripts) en `tests/**`. Raak voor deze lane geen `src/**` aan.

De verplichte gate op `chef-runner-01-1` is:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
shellcheck install.sh scripts/*.sh
```

Visual shots mogen in CI warning-only zijn, maar moeten hun artefacten en concrete foutmelding bewaren. De 15-domein-run gebruikt `scripts/visual-shot.sh --mode all-domains`; zie [docs/chefapp-qa.md](docs/chefapp-qa.md).
