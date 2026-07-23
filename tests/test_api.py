from __future__ import annotations

import json
import unittest
from unittest.mock import patch

from chefbar import api


class FakeResponse:
    def __init__(self, body: dict) -> None:
        self.body = json.dumps(body).encode()

    def __enter__(self) -> "FakeResponse":
        return self

    def __exit__(self, *_args) -> None:
        return None

    def read(self) -> bytes:
        return self.body


class ApiTests(unittest.TestCase):
    @patch.object(api, "read_api_token", return_value=None)
    @patch.object(api.urllib.request, "urlopen")
    def test_get_works_when_optional_token_is_unset(self, urlopen, _token) -> None:
        urlopen.return_value = FakeResponse({"ok": True})
        self.assertEqual(api.api_request("/status"), {"ok": True})
        request = urlopen.call_args.args[0]
        self.assertIsNone(request.get_header("Authorization"))

    @patch.object(api, "read_api_token", return_value="secret")
    @patch.object(api.urllib.request, "urlopen")
    @patch.object(api.uuid, "uuid4", return_value="fixed-id")
    def test_switch_uses_canonical_endpoint_and_revision(
        self, _uuid, urlopen, _token
    ) -> None:
        urlopen.return_value = FakeResponse({"ok": True, "revision": 8})
        result = api.switch_account("work", "vault", 7)
        self.assertEqual(result, {"ok": True, "revision": 8})
        request = urlopen.call_args.args[0]
        self.assertTrue(request.full_url.endswith("/coding/accounts/switch"))
        self.assertEqual(request.get_header("Idempotency-key"), "chefbar-fixed-id")
        self.assertEqual(
            json.loads(request.data),
            {"source": "vault", "accountId": "work", "expectedRevision": 7},
        )


if __name__ == "__main__":
    unittest.main()
