# Cloud Agent, Daytona nood-runner, browser kits

ChefBar's Cursor Cloud environment is **repo-managed** via `.cursor/environment.json`.
New agents after merge run:

| Phase | When | What |
| --- | --- | --- |
| `install` | environment build / first bootstrap | `.cursor/install.sh` — GTK3 headers, Playwright/Chromium OS libs, rustup stable, bun, Chrome wrapper, `cargo build --locked` |
| `start` | every VM boot from a snapshot | `.cursor/start.sh` — `apt-get update`, install/upgrade the pinned toolchain packages, `rustup update stable`, `bun upgrade`, re-apply the Chrome wrapper |

You should not need to run `sudo apt update/upgrade`, `bun upgrade`, or `npm update` by hand on a Cloud Agent. ChefBar has no root `package.json`; npm is left alone so NVM/Node from the Cursor image stay intact.

A full `apt-get upgrade` of the Ubuntu image is **not** done on every boot: it is slow and can break the snapshot. Start refreshes indexes and upgrades only the packages in `.cursor/lib.sh`.

## Chrome / computer-use noise

These log lines are Chromium internals on a Cloud VM, not ChefBar crashes:

* `google_apis/gcm ... DEPRECATED_ENDPOINT` — Google's GCM registration API is gone.
* `crashpad ... /sys/devices/system/cpu/cpu0/cpufreq/scaling_*_freq` — the VM has no cpufreq sysfs.
* `services/on_device_model ... service_cli` — on-device ML is not in this image.

`.cursor/chrome-wrapper.sh` keeps Cursor's computer-use flags (`--remote-debugging-port=9222`, `--no-sandbox`, SwiftShader) and disables GCM, crash reporter, and on-device model. `install`/`start` install it as `/usr/local/bin/google-chrome` and `/usr/local/bin/chrome` (with a `.cursor-orig` backup).

## Cloudflare computer-use / browser kits

The Cloud Agent image already ships Chrome. Install/start add the OS libraries that `@cloudflare/playwright`, Playwright MCP, and Browser Rendering kits expect (`libnss3`, `libgbm1`, `libatk-bridge`, fonts, …) plus **bun**. Point local Playwright at the wrapped Chrome:

```bash
export PLAYWRIGHT_CHROME_PATH="${HOME}/.local/bin/chefbar-chrome"
```

Cloudflare Computer / Browser Rendering still runs browsers **on Cloudflare**, not inside this VM. This repo does not vendor those public kits; it only keeps the Linux side compatible so agents can drive Chrome locally and talk to CF APIs.

chefgroep-os / cheffactory Docker schemas are not in this tree. ChefBar's copy lives at `docker/cursor-cloud.Dockerfile` (sidecar / Daytona, not the Cursor Cloud base).

## Daytona emergency sandbox

Separate from GitHub Actions (`pr-isolated` / `heavy`) and from Cursor Cloud VMs. Named sandbox: `chefbar-cursor-emergency`.

Every Cloud Agent **start** (when `DAYTONA_API_KEY` is set) runs `scripts/daytona-emergency.py --ensure --refresh`:

1. Create the sandbox if it does not exist.
2. Start it if it is stopped, paused, or archived.
3. Arm idle **auto-stop after 15 minutes** and **auto-archive after 24 hours stopped**. The named box is never auto-deleted.
4. Inside the sandbox: `apt-get update`, upgrade `ca-certificates`/`curl`, `pip install -U pip`, `bun upgrade`. The snapshot is 1 GB, so this is not a full dist-upgrade.

Idle auto-stop is the shutdown path. Cursor Cloud has no stop-hook; when the agent is gone and nothing talks to Daytona, the sandbox stops itself. Start it again with:

```bash
python3 scripts/daytona-emergency.py --ensure --refresh
python3 scripts/daytona-emergency.py --smoke
python3 scripts/daytona-emergency.py --stop   # optional explicit stop
```

Requirements:

* `DAYTONA_API_KEY` must be the full `dtn_` + 64 hex characters (68 total). A truncated 67-character value returns HTTP 401.
* Do not put the key in git, Dockerfiles, or chat.
* This is **not** a GitHub Actions runner. Registering `chef-runner-01-*` labels here would steal jobs from the company hosts.

A Daytona outage is logged as a warning and does not fail Cloud Agent start.
