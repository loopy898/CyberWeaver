from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from unittest.mock import patch


PLUGIN_DIR = Path(__file__).resolve().parents[1]
if str(PLUGIN_DIR) not in sys.path:
    sys.path.insert(0, str(PLUGIN_DIR))

from reverse_geocode import reverse_geocode  # noqa: E402


class _FakeResponse:
    def __init__(self, payload: dict[str, object]) -> None:
        self._payload = payload

    def read(self) -> bytes:
        return json.dumps(self._payload).encode("utf-8")

    def __enter__(self) -> "_FakeResponse":
        return self

    def __exit__(self, exc_type, exc, tb) -> bool:
        return False


class ReverseGeocodeTests(unittest.TestCase):
    def test_rejects_missing_coordinates(self) -> None:
        result = reverse_geocode({}, {})
        self.assertIn("failed", result["message"])
        self.assertEqual(result["added_nodes"], [])
        self.assertEqual(result["added_edges"], [])

    def test_rejects_out_of_range_coordinates(self) -> None:
        result = reverse_geocode({"latitude": 91, "longitude": 0}, {})
        self.assertIn("latitude", result["message"])

    @patch("reverse_geocode.urlopen")
    def test_returns_location_from_provider(self, mock_urlopen) -> None:
        mock_urlopen.return_value = _FakeResponse(
            {
                "display_name": "Chaoyang, Beijing, China",
                "address": {
                    "country": "China",
                    "state": "Beijing",
                    "city": "Beijing",
                    "suburb": "Chaoyang",
                },
            }
        )
        result = reverse_geocode({"latitude": 39.9042, "longitude": 116.4074}, {})
        self.assertIn("reverse_geocode completed", result["message"])
        self.assertIn("Chaoyang", "\n".join(result["tokens"]))
        self.assertEqual(len(result["added_nodes"]), 1)
        self.assertEqual(result["added_edges"], [])
        self.assertTrue(mock_urlopen.called)

    @patch("reverse_geocode.urlopen", side_effect=TimeoutError("timeout"))
    def test_handles_provider_timeout(self, _mock_urlopen) -> None:
        result = reverse_geocode({"latitude": 39.9, "longitude": 116.4}, {})
        self.assertIn("failed", result["message"])
        self.assertIn("timeout", result["message"])


if __name__ == "__main__":
    unittest.main()
