# Paste into chefbar-policy-http prompts

- `ureq` `redirects(0)`. Bearer never follows a 302.
- Every URL through `EndpointPolicy` + `auth::get_headers`.
- `safe_join` same-origin; reject host-swapping absolute paths.
- CF Access headers: both or neither.
- Never log tokens. Never put tokens on `Snapshot`.
- No tokio, reqwest, webview, Electron. Profile JSON camelCase + per-field env.
