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
DUB = REPO / "examples" / "dub-en-9x16"


def _load(name: str) -> dict:
    return json.loads((EXAMPLE / name).read_text(encoding="utf-8"))


def _load_dub(name: str) -> dict:
    return json.loads((DUB / name).read_text(encoding="utf-8"))


class TestSignal(unittest.TestCase):
    def test_hook_rate_uses_declared_window_and_does_not_dirty_body(self) -> None:
        score = _load("score.json")
        time_map = _load("time-map.json")
        expected = _load("dirty.json")
        got = dirty_from_signal(
            time_map["entries"],
            score["slots"],
            kind="hook_rate",
            signal_range={"start": 0, "duration": 1},
            spill_threshold_frames=score["spill_threshold_frames"],
        )
        self.assertEqual(got["range"], {"start": 0, "duration": 90})
        self.assertEqual(got["slots"], expected["slots"])
        self.assertEqual(got["kerfs"], expected["kerfs"])
        self.assertEqual(got["clean"], expected["clean"])
        self.assertNotIn("body", got["slots"])

    def test_tiny_spill_is_clean(self) -> None:
        slots = [
            {
                "id": "hook",
                "role": "hook",
                "range": {"start": 0, "duration": 84},
                "window": {
                    "kind": "hook_rate",
                    "range": {"start": 0, "duration": 90},
                },
            },
            {
                "id": "body",
                "role": "body",
                "range": {"start": 84, "duration": 516},
            },
        ]
        time_map = [
            {"dest": slot["range"], "slot": slot["id"]} for slot in slots
        ]
        got = dirty_from_signal(
            time_map,
            slots,
            kind="hook_rate",
            signal_range={"start": 0, "duration": 1},
        )
        self.assertEqual(got["slots"], ["hook"])

    def test_large_spill_dirties_body(self) -> None:
        slots = [
            {
                "id": "hook",
                "role": "hook",
                "range": {"start": 0, "duration": 72},
                "window": {
                    "kind": "hook_rate",
                    "range": {"start": 0, "duration": 90},
                },
            },
            {
                "id": "body",
                "role": "body",
                "range": {"start": 72, "duration": 528},
            },
        ]
        time_map = [
            {"dest": slot["range"], "slot": slot["id"]} for slot in slots
        ]
        got = dirty_from_signal(
            time_map,
            slots,
            kind="hook_rate",
            signal_range={"start": 0, "duration": 1},
        )
        self.assertEqual(got["slots"], ["hook", "body"])
        self.assertEqual(len(got["warnings"]), 1)

    def test_vo_hold_dirties_vo_and_audio_mix_not_body(self) -> None:
        score = _load_dub("score.json")
        time_map = _load_dub("time-map.json")
        expected = _load_dub("dirty.json")
        got = dirty_from_signal(
            time_map["entries"],
            score["slots"],
            kind="vo_hold",
            signal_range={"start": 0, "duration": 30},
            spill_threshold_frames=score["spill_threshold_frames"],
        )
        self.assertEqual(got["range"], {"start": 0, "duration": 90})
        self.assertEqual(got["slots"], expected["slots"])
        self.assertEqual(got["kerfs"], expected["kerfs"])
        self.assertEqual(got["clean"], expected["clean"])
        self.assertEqual(got["mixes"], ["audio_mix"])
        self.assertIn("vo", got["slots"])
        self.assertNotIn("body", got["slots"])
        self.assertIn("body", got["clean"])

    def test_explicit_dest_range_3_to_5s_names_overlapping_body_and_vo(self) -> None:
        score = _load_dub("score.json")
        time_map = _load_dub("time-map.json")
        expected = _load_dub("dirty-note.json")
        got = dirty_from_signal(
            time_map["entries"],
            score["slots"],
            kind="note",
            signal_range={"start": 90, "duration": 60},
            spill_threshold_frames=score["spill_threshold_frames"],
        )
        self.assertEqual(got["range"], {"start": 90, "duration": 60})
        self.assertEqual(got["slots"], expected["slots"])
        self.assertEqual(got["kerfs"], expected["kerfs"])
        self.assertEqual(got["clean"], expected["clean"])
        self.assertEqual(got["mixes"], ["audio_mix"])
        self.assertIn("body", got["slots"])
        self.assertIn("vo", got["slots"])

    def test_hold_names_body_and_does_not_invent_speed(self) -> None:
        score = _load_dub("score.json")
        time_map = _load_dub("time-map.json")
        expected = _load_dub("dirty-hold.json")
        got = dirty_from_signal(
            time_map["entries"],
            score["slots"],
            kind="hold",
            signal_range={"start": 90, "duration": 30},
            spill_threshold_frames=score["spill_threshold_frames"],
        )
        self.assertEqual(got["range"], {"start": 90, "duration": 120})
        self.assertEqual(got["slots"], expected["slots"])
        self.assertIn("body", got["slots"])
        self.assertNotIn("speed", json.dumps(got))
        self.assertEqual(got["mixes"], ["audio_mix"])

    def test_hook_rate_on_dub_clock_still_leaves_body_clean(self) -> None:
        score = _load_dub("score.json")
        time_map = _load_dub("time-map.json")
        expected = _load_dub("dirty-hook-rate.json")
        got = dirty_from_signal(
            time_map["entries"],
            score["slots"],
            kind="hook_rate",
            signal_range={"start": 0, "duration": 1},
            spill_threshold_frames=score["spill_threshold_frames"],
        )
        self.assertEqual(got["range"], {"start": 0, "duration": 90})
        self.assertEqual(got["slots"], expected["slots"])
        self.assertEqual(got["kerfs"], expected["kerfs"])
        self.assertEqual(got["clean"], expected["clean"])
        self.assertEqual(got["warnings"], expected["warnings"])
        self.assertNotIn("body", got["slots"])


if __name__ == "__main__":
    unittest.main()
