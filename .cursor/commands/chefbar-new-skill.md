---
name: chefbar-new-skill
description: Create a new ChefBar-specific Cursor skill under .cursor/skills using skill-creator conventions.
---

# /chefbar-new-skill

Read `.agents/skills/skill-creator/SKILL.md`, then create a ChefBar skill.

## Placement

- ChefBar-specific: `.cursor/skills/<kebab-name>/SKILL.md`
- Subagent that executes it: `.cursor/agents/<kebab-name>.md`
- Ecosysteem-install: `/find-skills` + `npx skills add` → `.agents/skills/` + `skills-lock.json`

## Required frontmatter

```yaml
---
name: kebab-name
description: What it does AND when to trigger (pushy). Include ChefBar filenames and synonyms.
---
```

`name` matches the directory. Keep `SKILL.md` well under 500 lines; put tables in `references/`. Point at invariants in `.cursor/rules/chefbar-invariants.mdc`. Add a row to `AGENTS.md` and `docs/agent-harness.md`.

Required next to `SKILL.md`:

- `evals/evals.json` — `skill_name`, ≥3 cases (`prompt`, `expected_output`, ≥2 `expectations`)
- `evals/triggers.json` — `should_trigger[].must_match_description` terms must appear in the YAML **description**

If the skill has a worker, add `.cursor/agents/<name>.md` (owns, playbook, output, handoff, anti-patterns, definition of done) and a `skill:` + `writes:` row in `.cursor/skills/chefbar-graph-loop/references/graph.yaml`. Distinctive filenames in the **description** so `node scripts/agent-bench.mjs` routing stays ≥ 0.75.

Do not duplicate `chefbar-architecture` or invent a second poll-loop as a “helper daemon.”
