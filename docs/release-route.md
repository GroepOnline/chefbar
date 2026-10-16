# Release route (this archive crate)

Declared version source: `Cargo.toml` `[package].version`.

GitHub tag `vX.Y.Z` must equal that Cargo version. A matching GitHub Release
must attach the CI-built `chefbar-linux-x86_64` binary and `SHA256SUMS`.
`.github/workflows/release.yml` is the only producer of that Release.

`chefbar --version`, `chefbar --doctor`, and the panel footer tooltip show
`CARGO_PKG_VERSION` plus `CHEFBAR_BUILD_SHA`. CI sets the SHA to the tag
commit. When the env is missing or empty the build SHA is the literal
`unknown` — never a guessed or defaulted hash.

## Why 4.0.3 (not 4.0.2, not 4.0.3-dev)

GitHub Release `4.0.2` (tag `4.0.2`, no `v` prefix, no assets) was published
from this repository at `9350837dcee42a88d7f0ecc6b146c2020b0e6697`. That
commit is an ancestor of current `main`. `Cargo.toml` at that commit was
still `4.0.0-dev`; the published name never matched the crate version.

`main` since 4.0.2 includes later unreleased work (including #58 visual craft
and CI/archive docs). Setting Cargo to `4.0.2` would label post-release
commits as the already-published 4.0.2. A `*-dev` prerelease is also
forbidden (dev below or above a published release is drift).

`4.0.3` is the next real semver that equals or exceeds the published
release and describes current `main`. This document does not create the
`v4.0.3` tag or GitHub Release.

Historical tags `4.0.0` / `4.0.1` / `4.0.2` stay archive-style. New
releases use `vX.Y.Z` only.

Rust compile remains CI-only on this laptop.
