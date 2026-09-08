"""Static contract for the bounded CodeQL advanced-setup workflow."""

from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github" / "workflows" / "codeql.yml"
CODEQL_PIN = "cdf488f595d80d6e07e03d4674febd5ab45fa938"
CHECKOUT_PIN = "11d5960a326750d5838078e36cf38b85af677262"


class CodeqlWorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.text = WORKFLOW.read_text(encoding="utf-8")

    def test_preserves_languages_and_default_query_suite(self) -> None:
        self.assertIn(
            "language: [actions, javascript-typescript, python]", self.text
        )
        self.assertNotRegex(self.text, r"(?m)^\s+queries:")
        self.assertNotIn("skip-queries", self.text)

    def test_resources_fit_the_runner_cgroup(self) -> None:
        self.assertEqual(len(re.findall(r"(?m)^\s+ram: 2048$", self.text)), 2)
        self.assertEqual(len(re.findall(r"(?m)^\s+threads: 1$", self.text)), 2)
        self.assertIn("max-parallel: 1", self.text)
        self.assertIn("timeout-minutes: 45", self.text)

    def test_triggers_runner_and_permissions_remain_explicit(self) -> None:
        for trigger in ("push:", "pull_request:", "schedule:"):
            self.assertIn(trigger, self.text)
        self.assertIn(
            "runs-on: [self-hosted, Linux, X64, pr-isolated]", self.text
        )
        for permission in (
            "actions: read",
            "contents: read",
            "packages: read",
            "security-events: write",
        ):
            self.assertIn(permission, self.text)

    def test_actions_are_immutable_and_checkout_drops_credentials(self) -> None:
        self.assertIn(f"actions/checkout@{CHECKOUT_PIN}", self.text)
        self.assertEqual(self.text.count(f"github/codeql-action/init@{CODEQL_PIN}"), 1)
        self.assertEqual(
            self.text.count(f"github/codeql-action/analyze@{CODEQL_PIN}"), 1
        )
        self.assertIn("persist-credentials: false", self.text)


if __name__ == "__main__":
    unittest.main()
