---
name: chefbar-graph
description: Run the ChefBar worker graph — file-disjoint subagents, Kater chains, QA-converge loop. Use for multi-module features, CI-red, or when the user asks for chains/graph loops.
---

# /chefbar-graph

Load skill `chefbar-graph-loop` and act as `chefbar-orchestrator`.

Arguments (optional): `feature` | `bugfix` | `review` | `ci-red` | `kater-ops` | `docs-only`. Default `feature`.

Do not implement the whole change in the parent. Dispatch workers from `.cursor/agents/` with the owns-sets in `AGENTS.md`. Cap `qa-converge` at 3.
