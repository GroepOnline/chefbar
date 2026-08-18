# Buildkite — ChefBar Rust CI

Fast Rust lane for `GroepOnline/chefbar` (ChefApp / ChefBar). GitHub Actions
remains the full CI surface; Buildkite is **optional** and must **not** be a
required status check.

| Item | Value |
|---|---|
| Buildkite org | `onlinechef` |
| Pipeline slug | `chefbar` |
| Status context | `buildkite/chefbar` |
| Git origin | `https://github.com/GroepOnline/chefbar.git` (own SHA only) |
| Donor (settings) | `onlinechef/herdr` — copy agent cluster/queue, **not** the herdr repo URL |
| Required on GitHub | **none** (keep chefbar merge policy unchanged) |

## Why

Self-hosted GHA Rust (`cargo check` / clippy / test / release) is slow.
Buildkite on a warm agent (or `chef-runner-01` with `cargo` on `PATH`) is the
fast path for the same hard gates. Visual shots, shellcheck, release artifacts
stay on GHA.

## “CI Origin was built on …”

That error means the pipeline’s configured git origin does not match the
triggering PR repo (e.g. herdr pipeline fired on a chefbar/ChefFactory SHA).
Fix: **own pipeline + own origin**. Never point `onlinechef/herdr` at chefbar.

## Remaining dashboard setup (API token absent)

No `BUILDKITE_API_TOKEN` on laptop `joep` (2026-08-18). Create the pipeline in
the UI once; YAML in this directory is already the steps source.

1. Open [New pipeline](https://buildkite.com/organizations/onlinechef/pipelines/new) while logged into org `onlinechef`.
2. **GitHub repository:** `GroepOnline/chefbar` only (not `herdr`, not ChefFactory).
3. **Name / slug:** `chefbar`.
4. **Steps:** “Read steps from repository” → `.buildkite/pipeline.yml` on the build commit.
5. **GitHub App:** ensure install `154480757` allowlist includes `GroepOnline/chefbar` (selected-repos, never `all`). Sibling lane owns that click.
6. **Agents:** reuse the herdr agent cluster/queue if it already has Rust+GTK3; otherwise register `buildkite-agent` on `chef-runner-01` (user `chef`, `PATH=$HOME/.cargo/bin:$PATH`) with the token from Clusters → Agents → New token. Do not install a Buildkite daemon on laptop `joep`.
7. Trigger a build on this PR branch; confirm context `buildkite/chefbar` (not `buildkite/herdr`).

## GHA stays responsible for

- Full `.github/workflows/ci.yml` (check, fmt, clippy, shellcheck, agent-bench, test, release build, visual shots)
- Release workflow
- Whatever informal/required merge evidence chefbar already used (no Buildkite in rulesets)
