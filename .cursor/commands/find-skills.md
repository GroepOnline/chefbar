---
name: find-skills
description: Discover and install agent skills from the open ecosystem (skills.sh). Use when the user asks how to do X, wants a skill for X, or wants to extend ChefBar agents with ecosystem packages.
---

# /find-skills

Read and follow `.agents/skills/find-skills/SKILL.md`.

ChefBar extras:

1. Search with `npx skills find <query>` (non-interactive). Leaderboard: https://skills.sh/
2. Prefer ≥1k installs and known publishers (`vercel-labs`, `anthropics`, `apollographql`).
3. Install **project-local** for Cursor, copied (not global, not symlink):

```bash
npx skills add <owner/repo> --skill <name> -a cursor --copy -y
```

4. Commit `skills-lock.json` and `.agents/skills/<name>/`.
5. Reject or wrap skills that push **tokio/async**, Electron, or a fictional `LSP()` tool — this crate is sync GTK3 + Grep/Read.
6. After install, add a row to `AGENTS.md` (ecosystem table) and note ChefBar overrides if any.
7. Custom ChefBar skills belong in `.cursor/skills/` (use `/chefbar-new-skill` / `skill-creator`), not as a random GitHub copy.

Do not install the whole of `affaan-m/ecc` (285 skills). Pick named `--skill`s.
