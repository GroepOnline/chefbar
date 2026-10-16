"""Static contract for CI runners and artifact retention."""

from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"


class CiWorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.text = WORKFLOW.read_text(encoding="utf-8")

    def test_public_repo_uses_github_hosted_runners(self) -> None:
        self.assertEqual(self.text.count("runs-on: ubuntu-latest"), 2)
        self.assertNotIn("self-hosted", self.text)

    def test_artifact_retention_is_bounded(self) -> None:
        release = self.text.split("name: chefbar-release", 1)[1].split("name:", 1)[0]
        shots = self.text.split("name: chefbar-visual-shots", 1)[1]
        self.assertIn("retention-days: 7", release)
        self.assertIn("retention-days: 1", shots)

    def test_host_deps_are_installed_then_preflighted(self) -> None:
        install_at = self.text.index("Install host build dependencies")
        preflight_at = self.text.index("Host deps preflight")
        self.assertLess(install_at, preflight_at)
        install_body = self.text[install_at:preflight_at]
        for package in ("pkg-config", "libgtk-3-dev", "libglib2.0-dev", "shellcheck"):
            self.assertIn(package, install_body)
        preflight_body = self.text[preflight_at:]
        self.assertIn("pkg-config --exists glib-2.0 gtk+-3.0", preflight_body)
        self.assertIn("exit 1", preflight_body)


if __name__ == "__main__":
    unittest.main()
