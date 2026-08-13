# Paste into chefbar-actions-palette prompts

- New command = `RunSpec` + `build_actions` row + exhaustive `Executor` arm.
- Keywords include `HarnessKind::prefixes`.
- Tiers: contains 1000 > prefix 700 > gappy 500. `boosted()` clamp 0..99.
- No HTTP in `build_actions`. `spawn_bg` is one-shot, not a poll loop.
- No tokio, reqwest, webview, Electron. Aliases are a tiny map, not an LLM.
- `CopySecretMeta { id }` copies a vault secret; `CopyText` is for non-secret clipboard. Notifications must not include the secret payload.
