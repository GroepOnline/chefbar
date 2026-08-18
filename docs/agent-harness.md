# ChefBar agent harness

Skills, Cursor-subagents, Kater-koppelingen, named chains en graph-loops voor deze repo. Kort: [`AGENTS.md`](../AGENTS.md) (catalogus, niet always-on). Invariants: `.cursor/rules/chefbar-invariants.mdc` — description-trigger of chain, niet `alwaysApply`.

## Lay-out

```text
AGENTS.md                          # thin catalog + stateless policy
.agents/skills/                    # SSOT: ChefBar + ecosysteem (Agent Skills, alle agents)
.cursor/skills/chefbar-*           # symlinks → .agents/skills (Cursor discovery)
.cursor/rules/                     # description trigger only (alwaysApply: false, no globs)
.cursor/agents/                    # file-disjoint Cursor workers (chain-loaded)
.cursor/commands/                  # slash: find-skills, chefbar-graph, …
.agents/skills/chefbar-graph-loop/references/graph.yaml
```

## Ecosysteem (find-skills)

Geïnstalleerd via [find-skills](https://www.skills.sh/vercel-labs/skills/find-skills) / `npx skills add … -a '*' -y` (project, alle agents, geen `--copy`):

| Skill | Package | Rol |
| --- | --- | --- |
| find-skills | vercel-labs/skills | zoeken + installeren |
| skill-creator | anthropics/skills | nieuwe skills schrijven |
| rust-best-practices | apollographql/skills | idiomen (geen tokio invoeren) |
| rust-patterns | affaan-m/ecc | ownership / Result |
| rust-testing | affaan-m/ecc | alleen inline tests overnemen |
| continuous-agent-loop | affaan-m/ecc | generiek; ChefBar-SSOT is `chefbar-graph-loop` |

Bewust **niet** geïnstalleerd: actionbook LSP-skills (Critical/High risk, `LSP()` bestaat hier niet), jeffallan `rust-engineer` (duwt tokio), hele `affaan-m/ecc`-bundel (285 skills).

Bijwerken: `npx skills update -p -y`. Lockfile: [`skills-lock.json`](../skills-lock.json).

## ChefBar-skills

| Skill | Pad |
| --- | --- |
| chefbar-architecture | `.agents/skills/chefbar-architecture/` |
| chefbar-actor | `.agents/skills/chefbar-actor/` |
| chefbar-rust | `.agents/skills/chefbar-rust/` |
| chefbar-gtk-panel | `.agents/skills/chefbar-gtk-panel/` |
| chefbar-tray-ipc | `.agents/skills/chefbar-tray-ipc/` |
| chefbar-policy-http | `.agents/skills/chefbar-policy-http/` |
| chefbar-actions-palette | `.agents/skills/chefbar-actions-palette/` |
| chefbar-kater | `.agents/skills/chefbar-kater/` |
| chefbar-qa | `.agents/skills/chefbar-qa/` |
| chefbar-bench | `.agents/skills/chefbar-bench/` |
| chefbar-graph-loop | `.agents/skills/chefbar-graph-loop/` |

Elke ChefBar-skill heeft `evals/evals.json` (≥3 cases) en `evals/triggers.json`. Agents in `.cursor/agents/` zijn volledige playbooks (owns, playbook, output, handoff, anti-patterns, definition of done).

## Workers

| Agent | Writes |
| --- | --- |
| chefbar-orchestrator | — |
| chefbar-architect | — (plan) |
| chefbar-rust-core | clippy/ownership-nits |
| chefbar-actor | `src/state.rs`, `src/models.rs` |
| chefbar-gtk-panel | `src/panel/**`, `src/css.rs`, `src/motion.rs`, `src/panel_state.rs` |
| chefbar-tray-ipc | `src/tray.rs`, `src/ipc.rs`, `src/notify.rs`, `src/quiet.rs`, `src/mutes.rs`, `src/doctor.rs`, `src/log.rs` |
| chefbar-policy-http | `src/policy.rs`, `src/http.rs`, `src/auth.rs`, `src/config.rs` |
| chefbar-actions-palette | `src/actions.rs`, `src/palette.rs`, `src/aliases.rs`, `src/frecency.rs`, `src/harness.rs` |
| chefbar-qa | `scripts/**`, CI, tests in geraakte modules |
| chefbar-kater | `src/sessions.rs`, `src/ops_cli.rs` + Kater MCP |

Parallel alleen bij disjuncte write-sets (zelfde idee als ChefApp 4.0 lanes).

## Graph

```text
architect → [domain workers ∥] → rust-core → qa ⟲ qa-converge (max 3)
orchestrator ─mcp─► kater pr_health (code|ops, readonly)
```

Machine-map: `.agents/skills/chefbar-graph-loop/references/graph.yaml`.

Named chains: `feature`, `bugfix`, `review`, `ci-red`, `kater-ops`, `docs-only`.

Slash: `/chefbar-graph`, `/chefbar-review`, `/find-skills`, `/chefbar-new-skill`.

## Benchmark

Deterministisch, geen LLM, geen netwerk:

```bash
node scripts/agent-bench.mjs
```

Scoort structure (frontmatter/secties/evals), graph-pairing, routing-corpus (`.cursor/evals/routing.json`, drempel 0.75), Cargo-verboden crates, GTK3-CSS-bans in `src/css.rs`, stateless rules (geen `alwaysApply: true`, geen `globs`). Rapport: `.cursor/evals/last-report.json` (gitignored). CI draait dezelfde stap.

Nieuwe skill of worker: `description` op **één regel** (Cursor parseert geen YAML `>-` / `|`), 120–1024 tekens met unieke bestandsnamen/constanten, evals ≥3, triggers-termen in de **description**, daarna de bench groen.

## Cursor-gedrag (stateless)

ChefBar-skills zijn **Agent Skills** in `.agents/skills/` (alle agents). `.cursor/skills/chefbar-*` zijn discovery-symlinks, geen tweede kopie.

Niets automatisch in context. Laden alleen via:

- **Triggers** — YAML-`description` op skills en `.cursor/rules/*.mdc` (`alwaysApply: false`, geen `globs`).
- **Chains** — `graph.yaml` / `/chefbar-graph` / Kater `pr_health`. Orchestrator injecteert invariants in worker-prompts op chain-tijd, niet always-on.

Skills: `disable-model-invocation` blijft **uit** (anders geen description-triggers). Handmatig: `/chefbar-*`.

Vendored ecosysteem-skills onder `.agents/skills/` (niet `chefbar-*`) niet herschrijven; product-skills winnen bij conflict als ze getriggerd zijn. De bench faalt als een rule `alwaysApply: true` of een `globs:`-key heeft, of als een ChefBar-skill geen symlink vanuit `.cursor/skills/` heeft.

## Kater-koppeling

ChefBar polt Kater **in** de ene actor (`KATER_POLL_MS` 30s). Agents gebruiken MCP-server `Kater`:

- Chain `pr_health` op profiel `code`/`ops`: GitHub PR → Linear issue → Sentry
- Adapters `code`: github, sqlite, filesystem, context7, deepwiki
- Adapters `ops`: github, linear, sentry, cloudflare (+ optioneel unconfigured)
- Lege `chains` op `core`/`cloud`/`reasoning` is normaal → lokale graph

`kater_pr_merge` alleen op expliciet verzoek.

## Nieuwe custom skill

`/chefbar-new-skill` of skill-creator. Directory `.agents/skills/<name>/` plus Cursor-symlink `.cursor/skills/<name>` → `../../.agents/skills/<name>`. Rij in deze file + `AGENTS.md`.
