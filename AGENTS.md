# ChefBar — agentinstructies

ChefBar is de native GTK3/Rust mission-control app van ChefGroep. Eén poll-actor, één snapshot, één Unix-socket, één venster. Geen Electron, geen webview, geen tweede daemon.

Dit bestand is de **catalogus**, geen always-on wet. Skills, rules en workers staan in `.cursor/` (ChefBar-eigen) en `.agents/skills/` (ecosysteem via `npx skills`). Uitleg: [`docs/agent-harness.md`](docs/agent-harness.md).

## Stateless — niets auto in context

Cursor laadt skills, rules en worker-playbooks **niet** standaard. Alleen via:

1. **Trigger** — YAML-`description` matcht de taak. Rules hebben `alwaysApply: false` en **geen** `globs`.
2. **Chain** — named graph (`feature` / `bugfix` / `review` / `ci-red` / `kater-ops` / `docs-only`) of `/chefbar-graph`. De orchestrator plakt invariants dan in worker-prompts.

Generic `continuous-agent-loop` / tokio-skills niet voor productcode. Product-skills houden `disable-model-invocation` **uit** (anders sterven triggers). Handmatig: `/chefbar-*`.

Non-negotiables (één poll-actor, sync `ureq`, GTK geen HTTP, last-good, geen Electron, `RunSpec`+`Executor`, `UiCommand`, camelCase origins, CI-gates, file-disjoint) staan in skill `chefbar-architecture`, rule `chefbar-invariants`, en `references/invariants.md` per skill — laden via trigger of chain, niet hier dupliceren.

## Omgeving

Bouwen/testen mag in deze cloud-omgeving en op de CI-runner. Vertel de gebruiker nooit om `rustup` op de laptop `joep` te zetten — daar staan fail-fast stubs. Zie `CONTRIBUTING.md`.

## Skills (laden via description of chain)

### ChefBar-eigen (`.cursor/skills/`)

| Skill | Wanneer |
| --- | --- |
| `chefbar-architecture` | Nieuwe domeinen, snapshot-vorm, harnassen, “waar hoort dit?” |
| `chefbar-actor` | Poll-actor: `state.rs`, `models.rs`, ritme, last-good, `coalesce_toasts` |
| `chefbar-rust` | Rust schrijven/reviewen in *deze* crate (overrides ecosysteem-async) |
| `chefbar-gtk-panel` | Panel, overlay, drawer, CSS, motion, density |
| `chefbar-tray-ipc` | Tray, `chefbar.sock`, doctor, notify/quiet/mutes |
| `chefbar-policy-http` | `policy`, `http`, `auth`, `config`, doctor-probes |
| `chefbar-actions-palette` | `RunSpec`, `Executor`, ranking 1000/700/500, harness-prefixes |
| `chefbar-kater` | Kater MCP-koppeling, chains/adapters, ChefBar↔Kater poll-vorm |
| `chefbar-qa` | fmt/clippy/test, visual-shot, CI |
| `chefbar-bench` | `node scripts/agent-bench.mjs`, routing.json, harness-score |
| `chefbar-graph-loop` | Multi-worker: chains, fan-out, qa-converge tot CI groen |

### Ecosysteem (`.agents/skills/`, lockfile `skills-lock.json`)

| Skill | Bron | Let op |
| --- | --- | --- |
| `find-skills` | vercel-labs/skills | Nieuwe skills zoeken/installeren |
| `skill-creator` | anthropics/skills | Eigen skill schrijven |
| `rust-best-practices` | apollographql/skills | Idiomen; **geen** tokio invoeren |
| `rust-patterns` | affaan-m/ecc | Ownership/Result; async-advies negeren |
| `rust-testing` | affaan-m/ecc | Alleen `#[cfg(test)]` inline; geen mockall/tokio/proptest tenzij in `Cargo.toml` |
| `continuous-agent-loop` | affaan-m/ecc | Generiek; ChefBar gebruikt `chefbar-graph-loop` als SSOT |

Nieuwe ecosysteem-skill: `npx skills find <query>` daarna `npx skills add <owner/repo> --skill <name> -a cursor --copy -y`. Verifieer installs (≥1k), bron, en of het tokio/LSP verzint. Lockfile committen.

## Subagent-workers (`.cursor/agents/`)

File-disjoint, zoals de ChefApp 4.0-lanes. Parallel alleen als de file-sets niet overlappen.

| Worker | Owns (schrijven) |
| --- | --- |
| `chefbar-architect` | geen productcode; plan + invariant-check |
| `chefbar-rust-core` | diff-brede clippy/ownership-review; kleine fixes |
| `chefbar-actor` | `src/state.rs`, `src/models.rs` |
| `chefbar-gtk-panel` | `src/panel/**`, `src/css.rs`, `src/motion.rs`, `src/panel_state.rs` |
| `chefbar-tray-ipc` | `src/tray.rs`, `src/ipc.rs`, `src/notify.rs`, `src/quiet.rs`, `src/mutes.rs`, `src/doctor.rs`, `src/log.rs` |
| `chefbar-policy-http` | `src/policy.rs`, `src/http.rs`, `src/auth.rs`, `src/config.rs` |
| `chefbar-actions-palette` | `src/actions.rs`, `src/palette.rs`, `src/aliases.rs`, `src/frecency.rs`, `src/harness.rs` |
| `chefbar-qa` | `scripts/**`, `.github/workflows/**`, inline tests in geraakte modules |
| `chefbar-kater` | Kater MCP + `src/sessions.rs` (kater-attach), `src/ops_cli.rs` |
| `chefbar-orchestrator` | geen code; dispatch van de graph |

`src/main.rs` en `src/lib.rs` zijn dunne seams — architect keurt splitsing goed, de domain-worker past aan.

## Build & gates

Cloud-agent en CI-runner hebben `cargo`/`rustc`. Laptop `joep` niet.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
shellcheck install.sh scripts/*.sh
node scripts/agent-bench.mjs
```

CI: `.github/workflows/ci.yml` op `[self-hosted, Linux, X64, company-control]`. Visual shots (`scripts/visual-shot.sh`) zijn warning-only. Agent-bench is deterministisch (geen LLM, geen netwerk) en faalt bij structure/invariants of routing < 0.75.

## Commits

Conventional commits met scope, bijv. `feat(panel): drawer onthoudt open-staat`. Geen force-push naar `main`.
