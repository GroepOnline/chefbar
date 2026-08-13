---
name: chefbar-bench
description: >-
  ChefBar agent-bench: node scripts/agent-bench.mjs scores skills, agents,
  graph.yaml pairing, routing.json accuracy, Cargo.toml forbidden crates, and a
  stylesheet invariant scan. Writes .cursor/evals/last-report.json. Exit 1 if
  blocking failures or routing below --min-routing (default 0.75). Use when
  adding a skill, worker, or eval case, or when CI reports bench failure.
---

# ChefBar agent-bench

Deterministic harness score. **No LLM. No network.** Node standard library only.

Scoring notes: [references/scoring.md](references/scoring.md). Runner: `scripts/agent-bench.mjs`. Corpus: `.cursor/evals/routing.json`.

## Instructions

1. From repo root: `node scripts/agent-bench.mjs` (optional `--json`, `--min-routing 0.8`).
2. **Skills** (`.cursor/skills/*/SKILL.md`):
   - YAML frontmatter `name` == directory, kebab-case
   - `description` 120–1024 chars, **no `<>`**
   - Headings matching Instructions\|Playbook\|Workflow, Example, Performance, Troubleshooting
   - `evals/evals.json`: `skill_name` match, **≥3** cases, each with `prompt`, `expected_output`, **≥2** `expectations`
   - `evals/triggers.json`: `should_trigger[].must_match_description` terms **all appear in the description**; `should_not_trigger[].must_not_all_match` must **not** all appear
3. **Agents** (`.cursor/agents/*.md`):
   - `name` == filename stem; description 120–1024
   - Headings matching Owns\|Identity, Playbook\|Workflow\|Instructions, Output, Handoff\|Anti-pattern, Done\|Definition
   - Body preferably **≥80 lines** (warning if thinner)
4. **Graph pairing** (`graph.yaml`): each expected worker has a skill directory; `writes:` lists are file-disjoint (ignore `diff-nits` / `inline-tests-*`).
5. **Routing**: token overlap of the query vs `name + description + first 4000 chars of body`. Filename-like tokens (`.rs`, `/`) get extra weight. `expect_skills` / `expect_agents` must appear in the **top-2** of that kind. `forbidden_skills`: top-1 skill must not be that name. Misses are warnings; **accuracy < --min-routing is blocking**.
6. **Invariants**: `Cargo.toml` must not pull tokio, async-std, reqwest, hyper, actix, axum. `src/css.rs` must not emit `gap:`, `inset:`, or CSS custom properties `--foo:`.
7. Report is written to `.cursor/evals/last-report.json` (gitignored). Do not commit it.

### Adding a routing case

Edit `.cursor/evals/routing.json`. Put the distinctive tokens (**filenames, constants**) in the **skill YAML description**, not only the body — trigger checks are description-only; routing uses description **and** body.

Pairing expected by the bench (agent → skill):

| Agent | Skill |
| --- | --- |
| chefbar-orchestrator | chefbar-graph-loop |
| chefbar-architect | chefbar-architecture |
| chefbar-rust-core | chefbar-rust |
| chefbar-actor | chefbar-actor |
| chefbar-gtk-panel | chefbar-gtk-panel |
| chefbar-tray-ipc | chefbar-tray-ipc |
| chefbar-policy-http | chefbar-policy-http |
| chefbar-actions-palette | chefbar-actions-palette |
| chefbar-qa | chefbar-qa |
| chefbar-kater | chefbar-kater |

Plus extra skill `chefbar-bench` (this one).

### Overall score

`0.4 * structure + 0.35 * routing% + 0.25 * quality`. Quality uses description length, `.rs` paths, forbidden-stack names, `references/`, eval count, triggers, body length.

## Examples

```bash
node scripts/agent-bench.mjs
node scripts/agent-bench.mjs --min-routing 0.8
node scripts/agent-bench.mjs --json | head
```

**Example — new skill**

Input: add `.cursor/skills/chefbar-foo/SKILL.md` without evals

Output: blocking `missing evals/evals.json`. Add ≥3 evals + triggers whose terms occur in the description. Add a routing case if the skill should win a distinctive prompt.

**Example — routing miss**

Input: `routing poll-rhythm missed`

Output: ensure `VAULT_POLL_MS` / `FETCH_BUDGET_MS` / `fetch_all` appear in `chefbar-actor` description. Do not stuff every skill with `state.rs`.

## Performance Notes

- Bench is O(skills × routing cases) — keep the corpus tight (dozens, not thousands).
- Distinctive tokens in the **description** beat repeating them in a 400-line body.
- Filename-like tokens (`.rs`, `/`) get +3 overlap.
- Stopwords include `chefbar`, `skill`, `agent` — they do not help routing.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| trigger term missing | copy the term into the YAML description |
| routing miss | unique filenames/constants in description; avoid overlapping jargon |
| `name` !== directory | rename folder or frontmatter |
| description has `<>` | Cursor strips those; rewrite |
| owns overlap | fix `writes:` in `graph.yaml` |
| CSS `gap` | gtk-panel / `src/css.rs` — child margin, not `gap` |
| overall fail with routing 70% | add tokens or relax is not allowed; min default 0.75 |

## Next

Run as part of `chefbar-qa`. Graph dispatch → `chefbar-graph-loop`. New skill shape → `/chefbar-new-skill`.
