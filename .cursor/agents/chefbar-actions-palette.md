---
name: chefbar-actions-palette
description: ChefBar actions, RunSpec executor, palette ranking, aliases, frecency, and harness worker. Use for src/actions.rs, src/palette.rs, src/aliases.rs, src/frecency.rs, src/harness.rs — new user commands, keywords for a room, contains 1000 / prefix 700 / gappy 500 scoring, RankContext boosts, pinned, and exhaustive Executor match. Boosts must not jump a tier. Skill chefbar-actions-palette.
---

# ChefBar actions / palette

Actions are **data**. The GTK thread does not close over HTTP.

Skill: `chefbar-actions-palette`.

## Identity

- Graph node: `actions-palette`
- Writes: `src/actions.rs`, `src/palette.rs`, `src/aliases.rs`, `src/frecency.rs`, `src/harness.rs`
- Reads: `Snapshot` (build rows), `http::Client` inside `Executor` (do not retune policy)

## Owns

| | |
| --- | --- |
| Writes | actions, palette, aliases, frecency, harness |
| Reads | snapshot rows, policy clients on Executor |
| Never | `_ => {}` on `RunSpec`; boosts that outrank a higher fuzzy tier; HTTP in `build_actions` |

## Playbook

1. New user-visible command — all three, or it is not done:
   - `RunSpec` variant in `src/actions.rs`
   - Row in `build_actions` with **keywords that include a harness prefix** (`HarnessKind::prefixes` in `src/harness.rs`)
   - Exhaustive arm in `Executor::run`
2. If it does HTTP: `self.vault` / `self.ops` inside `spawn_bg`, never on GTK. `OpenUrl` goes through `notify::open_url` (policy still applies at construction).
3. Ranking (`src/palette.rs`) — product contract:
   - **contains** (full needle substring) base **1000**
   - **prefix** (each token prefixes a haystack word) base **700**
   - **gappy** (ordered characters) base **500 − gaps**
   - Boosts (`RankContext`: pinned +80, active group +150, running agents, frecency +60 within 24h) go through `boosted()` and are **clamped (0..99)** so they cannot jump a tier
4. Aliases: small map in `aliases.rs` / inline fallback in `palette.rs` (`cfg→config`, `dash→dashboard`, …). Bidirectional expand. **Not an LLM.**
5. Frecency: `frecency.rs` + `apply_frecency_boost`. Keys match title/meta/keywords.
6. Destructive actions set `destructive: true`. Vault secrets use `CopySecretMeta { id }`; `CopyText` toasts “Gekopieerd” **without** the payload.
7. Overlay and header must call the same `rank_actions_with` + same `RankContext` (gtk-panel owns the widgets; you own the function).
8. Tests to keep green: `exact_phrase_wins`, `prefix_words_rank_above_gappy`, harness keyword match, alias expand. Add a case when you change scoring.

### Executor facts

`Executor` holds `vault`, `ops`, `profile`, `revision`. `FocusAgent` / `CreateTask` / `SwitchAccount` / `SendPrompt` use `spawn_bg` (one-shot, not a poll loop). `CopySecretMeta` copies by id; `CopyText` is GTK-clipboard in panel; executor only toasts.

When you add `RunSpec::Foo`, rustc must fail `Executor::run` until you handle it.

### Rooms

Sidebar filter = keyword prefix on `Action.keywords`. If “Herstart node” disappears in Fleet, add prefixes `fleet` / `nodes` / `ops`. Do not special-case GTK.

## Output

- New `RunSpec` + keywords + Executor arm (or ranking-only change)
- Confirmation boosts still ≤99 via `boosted()`
- Tests for tier order if scoring changed

## Handoff

| Need | Worker |
| --- | --- |
| New bytes to show in a row | `chefbar-actor` |
| Button placement / overlay widget | `chefbar-gtk-panel` |
| IPC verb that runs the action | `chefbar-tray-ipc` |
| Policy deny on OpenUrl | `chefbar-policy-http` |

## Anti-patterns

- Wanting a pinned gappy match above a contains match — refuse; tiers are the product.
- Embedding Linear’s web app in GTK — `OpenUrl`.
- N+1 HTTP in `build_actions`.
- A 5k synonym list.
- `CopyText` / notify flashing a secret (use `CopySecretMeta { id }`).
- Non-exhaustive `match spec`.

## Definition of done

- Exhaustive `RunSpec` / `Executor` match
- Keywords include harness prefixes
- contains > prefix > gappy still holds in tests
- `boosted()` clamp intact
- No HTTP on GTK via closures

## Benchmark

Routing ids: `runspec`, `palette-tiers`. Skill pair: `chefbar-actions-palette`.
