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

Do not duplicate `chefbar-architecture` or invent a second poll-loop as a “helper daemon.”
