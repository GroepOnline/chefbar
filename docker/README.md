# Docker schema — ChefBar Cloud Agent sidecar

`cursor-cloud.Dockerfile` is the Linux image recipe for a **Daytona nood-runner** or any other emergency box that should look like the ChefBar Cloud Agent toolchain (Rust stable ≥ 1.85, GTK3 headers, Chromium/Playwright OS libs, bun).

Cursor Cloud itself does **not** build this file. New Cloud Agents keep Cursor's default image and run:

1. `.cursor/install.sh` once per environment build (Rust/GTK, bun, Chrome wrapper, `cargo build --locked`)
2. `.cursor/start.sh` on every boot (apt index + toolchain package upgrades, `rustup update`, `bun upgrade`, re-apply Chrome wrapper)

## Why it lives here

chefgroep-os / cheffactory already carry other Docker schemas. ChefBar needs its own because this repo is the Cloud Agent workspace (`GroepOnline/chefbar`) and those other repos are not in this tree.

Do not `COPY` the ChefBar source into the image. The agent or Daytona sandbox checks out the git revision separately.

## Build locally

```bash
docker build -f docker/cursor-cloud.Dockerfile -t chefbar-cursor-cloud:emergency .
```

## Daytona

Create or reuse the named sandbox (no API key in git — `DAYTONA_API_KEY` from the environment):

```bash
python3 scripts/daytona-emergency.py --ensure --refresh
```

Cloud Agent `start.sh` already does that when `DAYTONA_API_KEY` is present. The sandbox auto-stops after 15 minutes idle and auto-archives after 24 hours stopped. Point a custom sandbox at this Dockerfile only when you need GTK/Rust inside Daytona; the default snapshot is enough for `process.code_run`.
