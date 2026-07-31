from __future__ import annotations

import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from chefbar.auth import auth_status, get_headers
from chefbar.endpoints import load_profile
from chefbar.security import EndpointPolicy, safe_urlopen
from chefbar.sessions import Session, load_ranked_sessions


class EndpointProfileTests(unittest.TestCase):
    def test_profile_loads_remote_surfaces(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "endpoints.json"
            path.write_text(
                json.dumps(
                    {
                        "name": "online",
                        "vaultApi": "https://vault-api.chefgroep.online/api",
                        "opsApi": "https://ops.chefgroep.online",
                        "katerWorkspace": "https://kater.chefgroep.online/agents/",
                    }
                )
            )
            profile = load_profile(path)
            self.assertEqual(profile.name, "online")
            self.assertEqual(profile.label("vaultApi"), "vault-api.chefgroep.online")
            self.assertEqual(profile.label("opsApi"), "ops.chefgroep.online")

    def test_profile_hosts_injected_into_policy(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "endpoints.json"
            path.write_text(
                json.dumps({"vaultApi": "https://vault-api.example.chefgroep.online/api"})
            )
            profile = load_profile(path)
            policy = EndpointPolicy().with_profile_hosts(*profile.all_urls())
            self.assertTrue(policy.allows(profile.vault_api))


class EndpointPolicyTests(unittest.TestCase):
    def setUp(self):
        self.policy = EndpointPolicy(
            https_allowlist=frozenset({"vault.chefgroep.online"}),
            http_allowlist=frozenset(),
            allow_tsnet_https=True,
        )

    def test_loopback_and_tailnet_http_are_allowed(self):
        self.assertTrue(self.policy.allows("http://127.0.0.1:8321/api"))
        self.assertTrue(self.policy.allows("http://100.115.43.1:18321/api"))

    def test_private_online_https_without_explicit_allowlist(self):
        policy = EndpointPolicy(https_allowlist=frozenset(), http_allowlist=frozenset())
        self.assertTrue(policy.allows("https://vault-api.chefgroep.online/api"))
        self.assertTrue(policy.allows("https://kater.chefgroep.online/agents/"))

    def test_public_http_and_unknown_https_are_blocked(self):
        self.assertFalse(self.policy.allows("http://example.com/api"))
        self.assertFalse(self.policy.allows("https://example.com/api"))

    def test_tsnet_and_explicit_https_are_allowed(self):
        self.assertTrue(self.policy.allows("https://chef-control.example.ts.net/api"))
        self.assertTrue(self.policy.allows("https://vault.chefgroep.online/api"))


class AuthHeaderTests(unittest.TestCase):
    def test_bearer_and_cf_access_headers(self):
        with patch.dict(
            os.environ,
            {
                "CHEF_VAULT_API_TOKEN": "test-token",
                "CF_ACCESS_CLIENT_ID": "cid",
                "CF_ACCESS_CLIENT_SECRET": "csec",
            },
            clear=False,
        ):
            headers = get_headers(json_body=True)
            self.assertEqual(headers["Authorization"], "Bearer test-token")
            self.assertEqual(headers["CF-Access-Client-Id"], "cid")
            self.assertEqual(headers["Content-Type"], "application/json")
            status = auth_status()
            self.assertEqual(status["mode"], "bearer+cf-access")


class SessionTests(unittest.TestCase):
    def test_kater_is_preferred_attach_action(self):
        session = Session.from_dict(
            {
                "id": "ses-1",
                "source": "opencodex",
                "state": "waiting",
                "title": "Review UDO",
                "attach": {
                    "focus": "pane-1",
                    "katerSessionId": "kater-42",
                    "evidenceUrl": "https://kater.chefgroep.online/evidence/42",
                },
            }
        )
        self.assertTrue(session.needs_attention)
        self.assertEqual(session.primary_action, ("Open sessie", "kater"))

    def test_load_ranked_sessions_prefers_attention(self):
        with patch(
            "chefbar.api.fetch_sessions",
            return_value=[
                {
                    "id": "a",
                    "source": "kater",
                    "state": "working",
                    "title": "Busy",
                },
                {
                    "id": "b",
                    "source": "kater",
                    "state": "waiting",
                    "title": "Needs you",
                },
            ],
        ):
            ranked = load_ranked_sessions()
            self.assertEqual(ranked[0].id, "b")
            self.assertEqual(ranked[0].state, "waiting")


if __name__ == "__main__":
    unittest.main()
