# Paste into chefbar-policy-http prompts

- `ureq` `redirects(0)`. Bearer never follows a 302.
- Outbound HTTP(S) requests go through `EndpointPolicy` + `auth::get_headers`. Browser URL opens (`xdg-open`) are a separate, policy-checked path without auth headers.
- `safe_join` same-origin (scheme, host, port); reject host-swapping absolute URLs.
- CF Access headers: both or neither.
- Never log tokens. Never put tokens on `Snapshot`.
- No tokio, reqwest, webview, Electron. Profile JSON camelCase + per-field env.
