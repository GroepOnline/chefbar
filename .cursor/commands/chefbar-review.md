---
name: chefbar-review
description: Fan-in ChefBar review — rust-core, architect, policy, gtk — without landing drive-by refactors.
---

# /chefbar-review

Run chain `review` from skill `chefbar-graph-loop`.

Spawn readonly workers in parallel (Task): `chefbar-rust-core`, `chefbar-architect`, plus `chefbar-policy-http` and/or `chefbar-gtk-panel` if those files are in the diff.

Synthesize one review: blockers first (second loop, GTK HTTP, secrets, tokio, CSS illegal properties), then nits. Do not apply patches unless the user asked.
