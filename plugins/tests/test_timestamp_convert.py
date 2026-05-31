from __future__ import annotations

import sys
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path


PLUGIN_DIR = Path(__file__).resolve().parents[1]
if str(PLUGIN_DIR) not in sys.path:
    sys.path.insert(0, str(PLUGIN_DIR))

from timestamp_convert import timestamp_convert  # noqa: E402


class TimestampConvertTests(unittest.TestCase):
    def test_rejects_empty_value(self) -> None:
        result = timestamp_convert({}, {})
        self.assertIn("failed", result["message"])
        self.assertEqual(result["added_nodes"], [])
        self.assertEqual(result["added_edges"], [])
        self.assertTrue(result["tokens"])

    def test_interprets_unix_seconds(self) -> None:
        result = timestamp_convert({"value": "1711584000"}, {})
        self.assertIn("timestamp_convert completed", result["message"])
        tokens = "\n".join(result["tokens"])
        self.assertIn("unix_seconds", tokens)
        self.assertIn("2024-03-28T00:00:00+00:00", tokens)
        self.assertEqual(len(result["added_nodes"]), 1)

    def test_interprets_unix_milliseconds(self) -> None:
        result = timestamp_convert({"value": "1711584000123"}, {})
        tokens = "\n".join(result["tokens"])
        self.assertIn("unix_milliseconds", tokens)
        self.assertIn("2024-03-28T00:00:00.123000+00:00", tokens)

    def test_interprets_apple_cocoa_seconds(self) -> None:
        # 0 in Cocoa epoch means 2001-01-01 00:00:00 UTC
        result = timestamp_convert({"value": 0}, {})
        tokens = "\n".join(result["tokens"])
        self.assertIn("apple_cocoa_seconds", tokens)
        self.assertIn("2001-01-01T00:00:00+00:00", tokens)

    def test_lists_all_interpretations_when_possible(self) -> None:
        result = timestamp_convert({"value": "1"}, {})
        tokens = "\n".join(result["tokens"])
        self.assertIn("unix_seconds", tokens)
        self.assertIn("unix_milliseconds", tokens)
        self.assertIn("unix_microseconds", tokens)
        self.assertIn("unix_nanoseconds", tokens)
        self.assertIn("apple_cocoa_seconds", tokens)

    def test_supports_negative_timestamp(self) -> None:
        result = timestamp_convert({"value": "-1"}, {})
        tokens = "\n".join(result["tokens"])
        self.assertIn("unix_seconds", tokens)
        expected = (datetime(1970, 1, 1, tzinfo=timezone.utc) - timedelta(seconds=1)).isoformat()
        self.assertIn(expected, tokens)


if __name__ == "__main__":
    unittest.main()
