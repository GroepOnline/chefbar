# ChefApp 5.0 — plan & lane contracts

ChefApp 5.0 maakt van ChefBar één native, read-first control-plane voor de Vault-tabs, server-API’s en Factory-control-plane. De campagne start vanaf `feat/chefapp-5.0` (`c197b57`); lanes blijven file-disjoint.

## Lane G — tooling & docs (§6.4)

Lane G raakt uitsluitend:

- `scripts/**`, met `scripts/visual-shot.sh` als reproduceerbare Xvfb/ImageMagick-runner;
- `.github/workflows/ci.yml`, met harde Rust-gates en warning-only visual jobs;
- `docs/**`, `README.md`, `CONTRIBUTING.md`;
- `Cargo.toml` alleen voor dev-deps/scripts;
- `tests/**` wanneer tooling-contracttests nodig zijn.

De visual runner ondersteunt panel, overlay/palette, drawer, beide density-varianten en vijftien domeinshots via `focus-domain`: `inbox`, `fleet`, `herdr`, `vault`, `accounts`, `providers`, `crm`, `share`, `clipboard`, `desktop`, `taken`, `linear`, `containers`, `secrets`, `kater`. `--mode all` maakt de volledige matrix; `--mode all-domains` maakt alleen de 15 domeinshots.

## Gates

De broncode bouwt en test uitsluitend op `chef-runner-01-1` (geen lokale Rust-build op Joep):

```text
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
shellcheck install.sh scripts/*.sh
```

Fmt, clippy en tests zijn harde gates. Visual shots zijn warning-only in CI, met upload van screenshots en per-shot `*-stderr.log` naast de PNG (niet alleen de workflow-log).

## §7 acceptatie

De acht functionele acceptatiepunten staan in [`chefapp-qa.md`](chefapp-qa.md): launch/stacked close, 15-domein-search, freshness/offline, tray, inbox/quiet, read-first domeinen, secrets-audit en doctor/single-instance. Geen plaintext secrets, tweede poll-loop of tweede socket is toegestaan.

## Buiten scope van lane G

Geen `src/**`-logica, geen panel/harness-rewrite, geen force-push naar `main`, en geen lokale build. Lane G levert één PR tegen `feat/chefapp-5.0`; de dirigent merge’t volgens de 5.0 mergetrain.
