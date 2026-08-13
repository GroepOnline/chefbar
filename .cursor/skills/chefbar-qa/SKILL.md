---
name: chefbar-qa
description: >-
  ChefBar QA gate: cargo fmt --check, cargo clippy --all-targets -- -D warnings,
  cargo test, bash scripts/visual-shot.sh, node scripts/agent-bench.mjs.
  CI workflow .github/workflows/ci.yml. Doctor fingerprints sha256[:12].
  Use when verifying a change, fixing CI, running visual-shot, or checking
  whether the agent-bench routing still passes.
---

# ChefBar QA

You are the merge gate for product *and* harness. CI: `.github/workflows/ci.yml` on `[self-hosted, Linux, X64, company-control]`.

Gates cheat-sheet: [references/gates.md](references/gates.md). Bench details: skill `chefbar-bench`.

## Instructions

Run in this order (cheap → expensive):

1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test --all-targets`
4. `shellcheck install.sh scripts/*.sh` (if those files changed, still cheap — CI always runs it)
5. `node scripts/agent-bench.mjs` — skills/agents structure, `graph.yaml` pairing, `.cursor/evals/routing.json`, Cargo/CSS invariants. No LLM, no network, Node stdlib only.
6. `bash scripts/visual-shot.sh` — PNG under `docs/` or the path the script documents. Fail if git sees a **new unignored PNG at repo root**. Dark panel accent `#5C97FF` when Xvfb exists. Exit **2** = no X-stack → skip, **not** a product bug. CI visual job is `continue-on-error` / warning-only.
7. Optional: `bash scripts/doctor.sh` — exit 0/1/2; fingerprints `sha256[:12]` only.

Do **not** add mockall, proptest, tokio, npm deps for the bench, or extra CI jobs that need a display beyond xvfb.

Cloud agent and CI runner have cargo (`rustc` 1.97+). Laptop `joep` does **not** — never tell the user to rustup (`CONTRIBUTING.md`).

### qa-converge

You are the graph node `qa`. Report:

- pass/fail per command
- first failing test or clippy lint with **file:line**
- owning worker guess from `AGENTS.md`

Prefer the owning worker over a QA rewrite. You may add/adjust `#[cfg(test)]` in files the parent already assigned, and you may edit `scripts/` and `.github/workflows/ci.yml`. Cap is **3** (orchestrator stops).

### Owner guess

| Failure | Owner |
| --- | --- |
| `state.rs` / `models.rs` | `chefbar-actor` |
| `panel/` / `css.rs` / `motion.rs` / `panel_state.rs` | `chefbar-gtk-panel` |
| `policy.rs` / `http.rs` / `auth.rs` / `config.rs` | `chefbar-policy-http` |
| `actions.rs` / `palette.rs` / `harness.rs` | `chefbar-actions-palette` |
| `ipc.rs` / `tray.rs` / `doctor.rs` | `chefbar-tray-ipc` |
| `sessions.rs` / `ops_cli.rs` | `chefbar-kater` |
| clippy unused/clone across modules | `chefbar-rust-core` |
| `agent-bench.mjs` / routing / skills | harness + `chefbar-bench` |

## Examples

**Example 1 — CI red**

Input: workflow “Clippy (harde gate)” failed `unused_mut` in `src/state.rs`

Output: do not `#[allow]`. Dispatch `chefbar-actor` (owns `state.rs`) or rust-core if it is a one-line nit the parent already scoped. Re-run clippy.

**Example 2 — visual**

Input: “is de CSS-fix visueel ok?”

Output: `scripts/visual-shot.sh panel dark …`. If root PNG appeared, fail. If exit 2, report skip.

**Example 3 — harness**

Input: new skill without `evals/evals.json`

Output: `node scripts/agent-bench.mjs` blocking; skill `chefbar-bench` playbook.

## Performance Notes

- fmt/clippy/test before visual-shot (shot is slower and warning-only).
- agent-bench is O(skills × routing cases) string scans — fine; run it whenever `.cursor/` changed.
- qa-converge: re-run only the failing slice, not the whole graph.
- Do not start GTK from QA except via `visual-shot.sh`.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| clippy unused_mut | fix the binding; do not allow dead_code |
| visual-shot git dirty PNG | shot must write under `docs/` or `/tmp`, not repo root |
| agent-bench routing < 0.75 | unique tokens in skill **descriptions** (triggers are description-only) |
| rustc not found | expected on laptop `joep`; use cloud/CI |
| visual-shot exit 2 | Xvfb/imagemagick missing — warning-only |
| bench `<>` in description | Cursor strips angle brackets; rewrite the YAML description |

## Next

Harness scoring rules → `chefbar-bench`. Dispatch → `chefbar-graph-loop`.
