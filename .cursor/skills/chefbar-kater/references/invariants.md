# Paste into chefbar-kater prompts

- MCP server id `Kater`. `GetMcpTools` before `CallMcpTool`.
- Live chain `pr_health` on profiles `code` and `ops` only.
- Empty `chains: []` on `core`/`cloud`/`reasoning` → local `chefbar-graph-loop`.
- In-app poll is `KATER_POLL_MS` in `state.rs` (chefbar-actor). This worker writes `sessions.rs` / `ops_cli.rs`.
- `kater_pr_merge` only with explicit user ask + expected SHA.
- No tokio, reqwest, webview, Electron; no browser scrape-loop of the gateway.
