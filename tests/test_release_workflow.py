"""Static contract for the tag == Cargo version release route."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github" / "workflows" / "release.yml"
CARGO_TOML = ROOT / "Cargo.toml"
CHECKOUT_PIN = "11d5960a326750d5838078e36cf38b85af677262"
STABLE_SEMVER = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")


def package_version(cargo_toml: Path = CARGO_TOML) -> str:
    text = cargo_toml.read_text(encoding="utf-8")
    try:
        body = text.split("[package]", 1)[1]
    except IndexError as exc:
        raise ValueError(f"{cargo_toml}: missing [package] table") from exc
    section = body.split("\n[", 1)[0]
    match = re.search(r'(?m)^version\s*=\s*"([^"]+)"', section)
    if not match:
        raise ValueError(f"{cargo_toml}: missing package version")
    return match.group(1)


def expected_tag(version: str) -> str:
    return f"v{version}"


class ReleaseWorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.text = WORKFLOW.read_text(encoding="utf-8")
        cls.version = package_version()

    def test_cargo_version_is_stable_semver(self) -> None:
        self.assertRegex(self.version, STABLE_SEMVER)
        self.assertNotIn("-", self.version)

    def test_workflow_is_tag_driven(self) -> None:
        self.assertIn("tags:", self.text)
        self.assertIn('- "v*"', self.text)
        self.assertIn("CHEFBAR_BUILD_SHA: ${{ github.sha }}", self.text)
        self.assertIn("sha256sum chefbar-linux-x86_64 > SHA256SUMS", self.text)
        self.assertIn("SHA256SUMS", self.text)
        self.assertIn("python3 tests/test_release_workflow.py --verify-current", self.text)
        self.assertIn(f"actions/checkout@{CHECKOUT_PIN}", self.text)
        self.assertIn("persist-credentials: false", self.text)
        self.assertIn("fetch-depth: 0", self.text)
        self.assertIn("cargo build --release --locked", self.text)

    def test_identity_source_is_option_env_not_vergen(self) -> None:
        lib = (ROOT / "src" / "lib.rs").read_text(encoding="utf-8")
        self.assertIn('option_env!("CHEFBAR_BUILD_SHA")', lib)
        self.assertIn('Some("") | None => "unknown"', lib)
        self.assertNotIn("vergen", lib.lower())


def verify_current() -> int:
    """Fail-closed live check used by release.yml on the checked-out tag."""
    version = package_version()
    if not STABLE_SEMVER.fullmatch(version):
        print(f"Cargo.toml version {version!r} is not stable X.Y.Z", file=sys.stderr)
        return 1

    tag = expected_tag(version)
    ref_name = _git_output(["rev-parse", "--abbrev-ref", "HEAD"])
    # On a tag checkout GITHUB_REF_NAME is the tag; locally HEAD may be a branch.
    github_ref = _env_or_none("GITHUB_REF_NAME")
    observed_tag = github_ref if github_ref else None
    if observed_tag and observed_tag != tag:
        print(f"tag {observed_tag} does not equal Cargo version tag {tag}", file=sys.stderr)
        return 1

    sha = _env_or_none("GITHUB_SHA") or _git_output(["rev-parse", "HEAD"])
    fetch = subprocess.run(
        ["git", "fetch", "--no-tags", "origin", "main"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if fetch.returncode != 0:
        print(fetch.stderr.strip() or "git fetch origin main failed", file=sys.stderr)
        return 1
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", sha, "origin/main"],
        cwd=ROOT,
        check=False,
    )
    if ancestor.returncode != 0:
        print(f"commit {sha} is not on origin/main", file=sys.stderr)
        return 1

    print(f"release tag contract OK ({tag} == Cargo {version}; {sha[:12]} on origin/main)")
    if ref_name:
        print(f"HEAD ref {ref_name}")
    return 0


def _env_or_none(name: str) -> str | None:
    import os

    value = os.environ.get(name, "").strip()
    return value or None


def _git_output(args: list[str]) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--verify-current", action="store_true")
    args = parser.parse_args(argv)
    if args.verify_current:
        return verify_current()
    suite = unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__])
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())
