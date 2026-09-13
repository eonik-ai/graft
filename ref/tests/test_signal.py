# SPDX-License-Identifier: Apache-2.0
from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

REF = Path(__file__).resolve().parents[1]
REPO = REF.parent
sys.path.insert(0, str(REF))

from graft_ref.signal import dirty_from_signal  # noqa: E402

EXAMPLE = REPO / "examples" / "hook-v3-body-v1-9x16"


def _load(name: str) -> dict:
    return json.loads((EXAMPLE / name).read_text(encoding="utf-8"))


class TestNorthStarExample(unittest.TestCase):
    def test_hook_rate_does_not_dirty_body(self) -> None:
        score = _load("score.json")
        time_map = _load("time-map.json")
        expected = _load("dirty.json")
        got = dirty_from_signal(
            time_map["entries"],
            score["slots"],
            kind="hook_rate",
            t0_s=0.0,
            t1_s=3.0,
            spill_threshold_s=score["spill_threshold_s"],
        )
        self.assertEqual(got["slots"], ["hook"])
        self.assertEqual(got["kerfs"], [["hook", "body"]])
        self.assertEqual(got["clean"], ["body", "cta"])
        self.assertEqual(got["warnings"], [])
        self.assertEqual(got["slots"], expected["slots"])
        self.assertEqual(got["kerfs"], expected["kerfs"])
        self.assertEqual(got["clean"], expected["clean"])
        self.assertNotIn("body", got["slots"])


class TestSpill(unittest.TestCase):
    def test_tiny_bleed_does_not_dirty_body(self) -> None:
        slots = [
            {"id": "hook", "role": "hook", "span": [0.0, 2.8]},
            {"id": "body", "role": "body", "span": [2.8, 20.0]},
        ]
        time_map = [
            {"span": [0.0, 2.8], "slot": "hook"},
            {"span": [2.8, 20.0], "slot": "body"},
        ]
        got = dirty_from_signal(
            time_map, slots, kind="hook_rate", t0_s=0.0, t1_s=3.0
        )
        self.assertEqual(got["slots"], ["hook"])
        self.assertEqual(got["kerfs"], [["hook", "body"]])
        self.assertEqual(got["warnings"], [])

    def test_large_bleed_dirties_body_and_warns(self) -> None:
        slots = [
            {"id": "hook", "role": "hook", "span": [0.0, 2.4]},
            {"id": "body", "role": "body", "span": [2.4, 20.0]},
        ]
        time_map = [
            {"span": [0.0, 2.4], "slot": "hook"},
            {"span": [2.4, 20.0], "slot": "body"},
        ]
        got = dirty_from_signal(
            time_map, slots, kind="hook_rate", t0_s=0.0, t1_s=3.0
        )
        self.assertEqual(got["slots"], ["hook", "body"])
        self.assertEqual(got["kerfs"], [["hook", "body"]])
        self.assertEqual(len(got["warnings"]), 1)

    def test_too_slow_on_body_leaves_hook_clean(self) -> None:
        slots = [
            {"id": "hook", "role": "hook", "span": [0.0, 3.0]},
            {"id": "body", "role": "body", "span": [3.0, 20.0]},
            {"id": "cta", "role": "cta", "span": [20.0, 23.0]},
        ]
        time_map = [
            {"span": [0.0, 3.0], "slot": "hook"},
            {"span": [3.0, 20.0], "slot": "body"},
            {"span": [20.0, 23.0], "slot": "cta"},
        ]
        got = dirty_from_signal(
            time_map, slots, kind="hold", t0_s=8.0, t1_s=12.0
        )
        self.assertEqual(got["slots"], ["body"])
        self.assertEqual(got["kerfs"], [["hook", "body"], ["body", "cta"]])
        self.assertEqual(got["clean"], ["hook", "cta"])


if __name__ == "__main__":
    unittest.main()
