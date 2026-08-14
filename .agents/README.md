# Agent Skills (lockfile)

Project skills live here — **all agents**, not Cursor-native copies.

- ChefBar first-party: `.agents/skills/chefbar-*` (SSOT). Cursor discovery: symlink `.cursor/skills/chefbar-*` → this directory.
- Ecosysteem: `npx skills add <pkg> --skill <name> -a '*' -y` (symlink, geen `--copy`). Lockfile: `../skills-lock.json`.
- `-g` / user-home alleen op expliciet verzoek; cloud-agents en CI restoren uit de repo.

Canonical ChefBar behavior: this tree + `../AGENTS.md`. If an installed skill recommends tokio, mockall, a second HTTP loop, or an `LSP()` tool, ignore that chapter.

Update: `npx skills update -p -y`

Find more: `npx skills find <query>` or slash `/find-skills`.
