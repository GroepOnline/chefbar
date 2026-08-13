---
name: chefbar-graph-loop
description: ChefBar multi-agent orchestration — file-disjoint workers, Kater MCP chains, and graph loops until gates pass. Use for features spanning modules, bugfix loops, PR review fan-in, CI red loops, or when the user asks for chains, subagents, workers, graph loops, or parallel lanes. Do not implement the whole change in the parent when this skill applies.
---

# ChefBar graph loop

Parent agents **orchestrate**. Domain workers **write**. The architect **does not** land product diffs. Max three QA-converge iterations unless the user raises the cap.

Read [`references/graph.yaml`](references/graph.yaml) for the machine map and [`references/chains.md`](references/chains.md) for named chains.

## Dispatch

1. Classify the job: `feature` | `bugfix` | `review` | `ci-red` | `kater-ops` | `docs-only`.
2. Map files to workers using the ownership table in `AGENTS.md`. If two workers would edit the same file, **serialize** those two; parallelize the rest.
3. Spawn workers with the Task tool:
   - Prefer `subagent_type` equal to the agent file stem (`chefbar-actor`, …) when the harness lists it.
   - Otherwise `subagent_type: "generalPurpose"` and paste the body of `.cursor/agents/<stem>.md` at the top of the prompt.
   - Read-only exploration: `explore`.
   - Never spawn `chefbar-orchestrator` from a worker (no nested graphs).
4. One message, multiple Task calls = parallel. Sequential steps = wait, then the next message.
5. After workers return: `chefbar-rust-core` reviews the combined diff, then `chefbar-qa` runs gates.

## File-disjoint rule

A worker may **read** anything, but may **write** only its owns-set (plus tests inside those files). `Cargo.toml` / `install.sh` / CI YAML → `chefbar-qa` or architect-approved exception in the prompt.

`src/main.rs` and `src/lib.rs` are seams: the domain worker patches them only to add a module or CLI flag named in the plan.

## Graph

```text
                    orchestrator
                          |
            +-------------+-------------+
            |             |             |
       architect      kater-probe      (optional pr_health MCP)
            |
            |  plan + file map
            v
     parallel domain workers
     (actor / gtk / tray / policy / actions)
            |
            v
        rust-core review
            |
            v
            qa  ----fail----> rust-core patch --+
            |                                   |
            pass                                |
            v                                   |
          done  <-------- loop qa-converge -----+  (max 3)
```

## Loops

### `qa-converge` (default graph loop)

1. `chefbar-qa` runs fmt/clippy/test (and shellcheck if scripts changed).
2. On fail: extract the failing crate/module → the owning worker patches, or `chefbar-rust-core` if it is a clippy/ownership nit.
3. Re-run QA. Stop on pass, on the same error twice (report stuck), or at iteration 3.

### `ci-red`

Same as `qa-converge` but QA starts from CI logs. Do not rewrite unrelated modules to silence a gate.

### `kater-pr-health`

MCP chain on profile `code` or `ops`: GitHub PR checks → Linear issue → Sentry search. Read-only. Attach findings to the orchestrator summary; do not block a local green test unless the user asked for merge-readiness.

## Worker prompts (minimum)

Each Task prompt must include:

- Goal (one paragraph)
- Owns-set (files they may write)
- Files they must not write
- Invariants reminder (one actor / no tokio / GTK3 CSS / no secrets)
- Definition of done (tests to add, clippy clean)

## Fan-in review (`review` chain)

Parallel readonly: `chefbar-rust-core`, `chefbar-architect`, `chefbar-policy-http` (if network files changed), `chefbar-gtk-panel` (if UI files changed). Orchestrator synthesizes; no code from reviewers unless the user asked to apply nits.

## Stuck / abort

Abort to the user when: ownership overlaps cannot be split; a worker asks for tokio/webview; policy would allow a new origin without a profile field; QA failed three times with a new error each round (scope too big).
