# Named chains

How the orchestrator sequences workers. File ownership: `AGENTS.md`. Graph YAML: `graph.yaml`.

## `feature`

1. `chefbar-architect` — plan, file map, invariant check. No product diff.
2. Parallel domain workers from the map (skip unused nodes).
3. `chefbar-rust-core` — clippy/ownership pass on the combined diff.
4. `chefbar-qa` — `cargo fmt --check`, `clippy -D warnings`, `cargo test --all-targets`, `shellcheck` if scripts changed.
5. Loop `qa-converge` until green or cap.

Optional: `chefbar-kater` MCP `pr_health` after a PR exists (readonly).

## `bugfix`

1. `chefbar-qa` reproduces (failing test or doctor/ipc scenario) **before** a wide patch.
2. Owning worker fixes (one owns-set).
3. `chefbar-rust-core` + `chefbar-qa` + `qa-converge`.

## `review`

Fan-in, readonly unless the user said “apply nits”:

- `chefbar-rust-core` — idioms, clones, unwrap, exhaustiveness
- `chefbar-architect` — second loop / second socket / GTK HTTP
- `chefbar-policy-http` — if `policy|http|auth|config` in the diff
- `chefbar-gtk-panel` — if `panel|css|motion|panel_state` in the diff

Orchestrator writes one review, ordered: blockers, then nits.

## `ci-red`

1. Read CI logs (workflow `CI` on self-hosted `company-control`).
2. Classify: fmt, clippy, test, shellcheck, visual-shot (visual is warning-only — do not treat as merge blocker).
3. Route to owning worker; `qa-converge`.

## `kater-ops`

1. `kater_profiles` / `kater_doctor` / `kater_chains` / `kater_adapters` for `code` or `ops`.
2. If `pr_health` exists, run it (GitHub PR → Linear issue → Sentry).
3. Empty `chains: []` on other profiles is normal — fall back to the local graph, do not fabricate remote tools.

## `docs-only`

Architect + `chefbar-qa` (markdown/scripts). Do not spawn GTK/actor workers.

## MCP vs in-process

| Kind | Where | Loop? |
| --- | --- | --- |
| ChefBar poll of Kater | `state.rs` 30s | **No** — already the one actor |
| Agent Kater chain | MCP `kater_chains` | Sequential tools, readonly by default |
| QA-converge | Cursor Task workers | Yes, cap 3 |
