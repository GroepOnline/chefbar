# agent-bench scoring

Script: `scripts/agent-bench.mjs` (Node, stdlib, no network).

## Exit

- `0` if no blocking failures and routing accuracy ≥ `--min-routing` (default `0.75`)
- `1` otherwise
- Routing **misses** are warnings; only the accuracy threshold (and structure/invariants) block

## Structure (blocking)

Skills: frontmatter, kebab `name` == directory, description 120–1024, no `<>`, required headings, `evals/evals.json` ≥3 complete cases, `evals/triggers.json`.

Agents: name == stem, description length, required headings (Owns/Identity, Playbook, Output, Handoff/Anti-pattern, Done).

Commands: name + description.

## Routing

`.cursor/evals/routing.json` `cases[]`:

- `prompt` — tokenized; stopwords dropped
- `expect_skills` — at least one in top-2 skills
- `expect_agents` — at least one in top-2 agents
- `forbidden_skills` — top-1 skill must not match

Overlap: query tokens vs `name + description + body[:4000]`. Tokens containing `.rs` or `/` get extra weight.

## Invariants (blocking)

- `Cargo.toml` must not depend on tokio, async-std, reqwest, hyper, actix, axum
- `src/css.rs` must not emit `gap:`, `inset:`, or `--custom-property:`
- `graph.yaml` `writes:` paths must be file-disjoint
- `.cursor/rules/*.mdc` must be stateless: no `alwaysApply: true`, no `globs:` key, description one physical line
- ChefBar skills must not set `disable-model-invocation: true`

## Report

`.cursor/evals/last-report.json` (gitignored).
