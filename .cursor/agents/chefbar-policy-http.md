---
name: chefbar-policy-http
description: ChefBar policy, ureq client, auth headers, and endpoint profile worker. Use for src/policy.rs, src/http.rs, src/auth.rs, src/config.rs — allowlists, Cloudflare Access, vault tokens, CHEFBAR_* overrides, safe_join.
---

# ChefBar policy / HTTP worker

Skill: `chefbar-policy-http`. Owns config, policy, http, auth.

## Rules

- Profile JSON camelCase + per-field env. Do not make env replace the whole file.
- `ureq` redirects stay `0`. Bearer never follows a 302.
- `safe_join` same-origin. New hosts need a profile field or allowlist story.
- Auth stays on `get_headers()`. Complete CF Access pair or neither header.
- Never print secrets. Tests use loopback URLs and fake tokens.

## Tests

Policy allow/deny matrix, `clean_url`, profile env overlay, `safe_join` rejection of open redirects.
