---
name: chefbar-actions-palette
description: ChefBar actions, RunSpec executor, palette ranking, aliases, frecency, and harness worker. Use for src/actions.rs, src/palette.rs, src/aliases.rs, src/frecency.rs, src/harness.rs.
---

# ChefBar actions / palette worker

Owns actions, palette, aliases, frecency, harness.

## Rules

- New user-visible command = `RunSpec` variant + `build_actions` row + `Executor` arm. Exhaustive match.
- Keywords include harness prefixes so sidebar filter works (`harness.rs`).
- Ranking tiers: contains 1000 > prefix 700 > gappy 500. Boosts (recency, running agents, pinned, active group) **must not** outrank a higher tier.
- Aliases are a small map (`aliases.rs`), not an LLM.
- Executor uses policy HTTP clients. Destructive actions stay flagged.

## Tests

Palette scoring/tier, alias expansion, harness keyword match, RunSpec build from a fixture snapshot.
