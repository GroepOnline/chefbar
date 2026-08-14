---
name: chefbar-actions-palette
description: ChefBar actions and palette skill for RunSpec, Executor, build_actions, fuzzy ranking, RankContext, aliases, frecency, and harness.rs prefixes. Use when adding a user command, keywords for a room, contains 1000 prefix 700 gappy 500 scoring, pinned boost, or exhaustive Executor match in actions.rs palette.rs aliases.rs frecency.rs. Use when search ranking feels wrong or a sidebar filter hides an action.
---

# ChefBar actions + palette

Actions are data. The GTK thread does not close over HTTP.

## Instructions

1. New user-visible command:
   - Add a `RunSpec` variant in `src/actions.rs`
   - Add a row in `build_actions` with **keywords that include a harness prefix** (`HarnessKind::prefixes` in `src/harness.rs`)
   - Add an exhaustive arm in `Executor::run`
   - If it does HTTP, use `self.vault` / `self.ops` inside `spawn_bg`, never on GTK
2. Ranking (`src/palette.rs`):
   - contains (full needle substring) **1000**
   - prefix (each token prefixes a haystack word) **700**
   - gappy (ordered characters) **500 - gaps**
   - Boosts (`RankContext`, pinned +80, active group +150, running agents, frecency +60 within 24h) are **capped** via `boosted()` so they cannot outrank a higher tier (boost clamp 0..99 added onto the tier base)
3. Aliases: small map in `aliases.rs` / inline fallback in `palette.rs` (`cfg→config`, `dash→dashboard`, …). Bidirectional expand. Not an LLM.
4. Frecency: `frecency.rs` + `apply_frecency_boost`. Keys match title/meta/keywords.
5. Destructive actions set `destructive: true`. Vault secrets use `CopySecretMeta { id }`; `CopyText` is non-secret clipboard. Notify without the payload.
6. Tests: `exact_phrase_wins`, `prefix_words_rank_above_gappy`, harness keyword match, alias expand. Keep that style.

### Executor facts

`Executor` holds `vault`, `ops`, `profile`, `revision`. `OpenUrl` goes through `notify::open_url` (policy still applies at construction time). `FocusAgent` / `CreateTask` / `SwitchAccount` / `SendPrompt` use `spawn_bg`. `CopySecretMeta` copies by id; `CopyText` is GTK-clipboard in panel; executor only toasts.

### Exhaustiveness

When you add `RunSpec::Foo`, rustc should fail `Executor::run` and any `match spec` in tests until you handle it. Do not add `_ => {}`.

## Examples

**Example 1 — action invisible in Fleet**

Input: title “Herstart node” but keywords `"restart"`

Output: add prefixes `fleet` / `nodes` / `ops` so sidebar prefix-match keeps it.

**Example 2 — boost jumps tier**

Input: want pinned gappy match above a contains match

Output: refuse. Tiers are the product: contains always beats prefix always beats gappy. Boost only reorders **inside** a tier (`boosted` adds ≤99).

**Example 3 — new Linear open**

Input: open issue in browser

Output: `RunSpec::OpenLinearIssue(id)` already exists — wire keywords `linear` + issue id; `OpenUrl` to Linear. Do not embed Linear’s web app in GTK.

## Performance Notes

- `rank_actions_with` scores every action per keystroke. Keep `fuzzy_score` allocation modest (haystack format once).
- `build_actions` runs on snapshot rebuild — O(snapshot rows). Do not N+1 HTTP there.
- Alias maps stay tiny; a 5k synonym list would dominate ranking.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| Contains “fl” matches “fleet” | that is prefix, not contains; contains needs full needle |
| Overlay ≠ header results | both must use `rank_actions_with` + same `RankContext` |
| Executor compile after new variant | missing match arm |
| Secret flashed in mako | `CopySecretMeta`; never put the secret in notify |

## Invariants to paste

See [references/invariants.md](references/invariants.md). Short form:

- `RunSpec` + `build_actions` + exhaustive `Executor` arm. Keywords include harness prefixes.
- contains 1000 > prefix 700 > gappy 500. `boosted()` clamp 0..99.
- No HTTP in `build_actions`. No tokio, reqwest, webview, or Electron.

## Next

New bytes for the action → `chefbar-actor`. Button placement → `chefbar-gtk-panel`. IPC verb → `chefbar-tray-ipc`.

Copy [references/invariants.md](references/invariants.md) into worker prompts. No tokio, reqwest, webview, or Electron. Boosts never jump a ranking tier.
