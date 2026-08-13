# Module map

| File | Role | Worker |
| --- | --- | --- |
| `src/main.rs` | CLI + bootstrap | seam (architect + owner) |
| `src/lib.rs` | `pub mod` list | seam |
| `src/state.rs` | poll-actor, `Shared` | chefbar-actor |
| `src/models.rs` | Snapshot, builders, toasts | chefbar-actor |
| `src/config.rs` | EndpointProfile | chefbar-policy-http |
| `src/policy.rs` | EndpointPolicy | chefbar-policy-http |
| `src/auth.rs` | `get_headers` | chefbar-policy-http |
| `src/http.rs` | ureq Client, redirects 0 | chefbar-policy-http |
| `src/actions.rs` | RunSpec + Executor | chefbar-actions-palette |
| `src/palette.rs` | fuzzy ranking | chefbar-actions-palette |
| `src/aliases.rs` | query aliases | chefbar-actions-palette |
| `src/frecency.rs` | recency boost | chefbar-actions-palette |
| `src/harness.rs` | rooms / kinds | chefbar-actions-palette |
| `src/panel/mod.rs` | window lifecycle | chefbar-gtk-panel |
| `src/panel/header.rs` | search header | chefbar-gtk-panel |
| `src/panel/sidebar.rs` | nav | chefbar-gtk-panel |
| `src/panel/zones.rs` | cards | chefbar-gtk-panel |
| `src/panel/drawer.rs` | detail | chefbar-gtk-panel |
| `src/panel/overlay.rs` | palette overlay | chefbar-gtk-panel |
| `src/css.rs` | GTK3 stylesheet | chefbar-gtk-panel |
| `src/motion.rs` | fades | chefbar-gtk-panel |
| `src/panel_state.rs` | persist UI | chefbar-gtk-panel |
| `src/tray.rs` | ksni + UiCommand | chefbar-tray-ipc |
| `src/ipc.rs` | unix socket | chefbar-tray-ipc |
| `src/notify.rs` | toasts | chefbar-tray-ipc |
| `src/quiet.rs` | quiet hours | chefbar-tray-ipc |
| `src/mutes.rs` | per-agent mute | chefbar-tray-ipc |
| `src/doctor.rs` | probes, exit 0/1/2 | chefbar-tray-ipc |
| `src/sessions.rs` | attach / katerSessionId | chefbar-kater |
| `src/ops_cli.rs` | ops helpers | chefbar-kater |
| `src/log.rs` | logging | chefbar-tray-ipc (read with owner) |
