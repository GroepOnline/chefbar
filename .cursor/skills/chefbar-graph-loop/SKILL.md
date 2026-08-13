---
name: chefbar-graph-loop
description: ChefBar graph-loop: file-disjoint workers, orchestrator dispatch, architect plan, parallel domain workers, chefbar-rust-core, chefbar-qa, then qa-converge max 3. Machine graph at graph.yaml. Optional Kater MCP pr_health chain after local converge. Use when the user asks for a graph loop, multi-agent run, sequential workers, or how to dispatch without a second poll-actor.
---

# ChefBar graph-loop

This skill is the **SSOT** for multi-worker runs. Ecosysteem `continuous-agent-loop` is generic; ignore its poll/tokio chapters.

Load [references/graph.yaml](references/graph.yaml) and [references/chains.md](references/chains.md). Parent agent: `chefbar-orchestrator`.

## Instructions

1. Start as `chefbar-orchestrator`. It does **not** write product files under `src/`.
2. `chefbar-architect` produces a plan: files, owns-set, workers, tests, refusals. No product edits.
3. Dispatch **file-disjoint** domain workers in parallel. Overlapping writes **serialize**.
4. Fan-in: `chefbar-rust-core` (clippy `-D warnings`, ownership, no tokio, exhaustive matches).
5. Gate: `chefbar-qa` (fmt, clippy, test, shellcheck, visual-shot warning-only, `agent-bench.mjs` if harness changed).
6. If QA fails: **qa-converge** — only the failing worker (or rust-core if cross-cutting) + qa. Max **3** loops. Then **stop**.
7. Optional readonly: Kater MCP `kater_chains` profile `code`/`ops` name `pr_health`. Never `kater_pr_merge` unless the user asked.

Do **not** spawn a second poll-actor. Workers are sequential/parallel **agents**, not extra `std::thread` loops in the poll-actor. In-budget GET fan-out already exists inside that actor.

### Owns-set (copy into every worker prompt)

| Worker | Writes |
| --- | --- |
| `chefbar-orchestrator` | — |
| `chefbar-architect` | — (plan) |
| `chefbar-rust-core` | nits on the existing diff |
| `chefbar-actor` | `src/state.rs`, `src/models.rs` |
| `chefbar-gtk-panel` | `src/panel/**`, `src/css.rs`, `src/motion.rs`, `src/panel_state.rs` |
| `chefbar-tray-ipc` | `tray.rs`, `ipc.rs`, `notify.rs`, `quiet.rs`, `mutes.rs`, `doctor.rs`, `log.rs` |
| `chefbar-policy-http` | `policy.rs`, `http.rs`, `auth.rs`, `config.rs` |
| `chefbar-actions-palette` | `actions.rs`, `palette.rs`, `aliases.rs`, `frecency.rs`, `harness.rs` |
| `chefbar-qa` | `scripts/**`, `.github/workflows/**`, tests in already-touched modules |
| `chefbar-kater` | `src/sessions.rs`, `src/ops_cli.rs` + MCP |

`src/main.rs` / `src/lib.rs` are thin seams — architect splits, the nearest domain worker edits.

### Named chains

| Chain | Sequence |
| --- | --- |
| `feature` | architect → [domain ∥] → rust-core → qa ⟲ qa-converge |
| `bugfix` | qa reproduces → owning worker → rust-core → qa ⟲ |
| `review` | rust-core + architect + policy/gtk if those files moved (readonly unless asked to apply) |
| `ci-red` | classify CI log → owning worker → qa ⟲ |
| `kater-ops` | kater MCP `pr_health` on `code`/`ops`; empty other profiles → this graph |
| `docs-only` | architect + qa; no GTK/actor workers |

Slash: `/chefbar-graph <chain>`.

### MCP vs in-process

| Kind | Where | Loop? |
| --- | --- | --- |
| ChefBar poll of Kater | `state.rs` `KATER_POLL_MS` 30s | **No** — one actor |
| Agent Kater chain | MCP `kater_chains` | Sequential tools, readonly default |
| QA-converge | Cursor Task workers | Yes, cap 3 |

Local graph is SSOT. Empty Kater `core`/`cloud`/`reasoning` is not a failure.

## Examples

**Example 1 — two modules**

```text
User: "haal de poll-timeout omlaag en fix de drawer CSS"
Architect: chefbar-actor (state.rs) + chefbar-gtk-panel (css.rs, drawer.rs)
Orchestrator: both in parallel (disjoint files)
Then rust-core → qa
If visual-shot fails: qa-converge gtk-panel only, not actor
```

**Example 2 — graph on a PR**

```text
User: "run the graph loop on this PR"
Orchestrator: architect plan from diff → dispatch owns-sets → rust-core → qa → optional pr_health
```

**Example 3 — refuse a second actor**

```text
User: "start a worker thread that polls Linear so the graph can wait"
Output: refuse. Linear already ticks on the one poll-actor. Graph-loop ≠ poll-actor.
```

## Performance Notes

- Parallelize only **file-disjoint** workers. Overlapping writes serialize — cheaper than merge conflicts.
- qa-converge is cheaper than a full graph restart. Cap 3 is a hard stop, not a suggestion.
- Architect and orchestrator never touch `src/` — keeps the diff reviewable.
- Do not call Kater MCP on every keystroke; `pr_health` is post-converge / review / ci-red.
- After harness edits, QA runs the agent-bench (routing corpus includes `graph-loop`).

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| Two workers edit `state.rs` | Architect split was wrong; serialize on actor |
| QA loops forever | Cap is 3; stop and report remaining failures |
| Agent wants tokio for “the loop” | Wrong loop. Graph-loop is agents; poll-actor is `state.rs` |
| `kater_chains` empty | Fall back to this YAML graph; do not invent tool names |
| Worker writes outside owns-set | Reject; re-dispatch the owning agent |
| visual-shot exit 2 | Xvfb missing — warning-only, not a converge reason |
| Bench routing miss | Distinctive filenames in skill **descriptions** (triggers are description-only) |

## Next

Single-module change → skip this skill, call the owning worker. Harness score → `chefbar-bench`.
