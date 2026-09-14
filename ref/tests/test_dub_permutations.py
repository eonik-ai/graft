# SPDX-License-Identifier: Apache-2.0
"""Exact dirty-set permutations on the dubbed 10s clock.

Hand fixtures are the contract. The 1-frame sweep checks every dest frame
on the shipped time map — including joins at 90 and 210 and a range past
the dest. Declared windows must ignore the caller's requested range.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

REF = Path(__file__).resolve().parents[1]
REPO = REF.parent
sys.path.insert(0, str(REF))

from graft_ref.signal import dirty_from_signal, overlap_frames  # noqa: E402

DUB = REPO / "examples" / "dub-en-9x16"

# kind, requested range, fixture (addressed range lives in the fixture)
NOTE_CASES = [
    ("note", {"start": 0, "duration": 90}, "dirty-note-0-3.json"),
    ("note", {"start": 90, "duration": 60}, "dirty-note.json"),
    ("note", {"start": 150, "duration": 30}, "dirty-note-5-6.json"),
    ("note", {"start": 210, "duration": 90}, "dirty-note-7-10.json"),
    ("note", {"start": 75, "duration": 30}, "dirty-note-join-hook-body.json"),
    ("note", {"start": 207, "duration": 6}, "dirty-note-join-body-cta.json"),
    ("note", {"start": 0, "duration": 300}, "dirty-note-full.json"),
    ("note", {"start": 360, "duration": 30}, "dirty-note-past.json"),
    ("note", {"start": 89, "duration": 1}, "dirty-note-frame-89.json"),
    ("note", {"start": 90, "duration": 1}, "dirty-note-frame-90.json"),
]

DECLARED_CASES = [
    ("vo_hold", {"start": 0, "duration": 1}, "dirty.json"),
    ("vo_hold", {"start": 90, "duration": 60}, "dirty.json"),
    ("vo_hold", {"start": 210, "duration": 90}, "dirty.json"),
    ("vo_hold", {"start": 360, "duration": 30}, "dirty.json"),
    ("hold", {"start": 0, "duration": 90}, "dirty-hold.json"),
    ("hold", {"start": 90, "duration": 30}, "dirty-hold.json"),
    ("hold", {"start": 210, "duration": 90}, "dirty-hold.json"),
    ("hook_rate", {"start": 0, "duration": 1}, "dirty-hook-rate.json"),
    ("hook_rate", {"start": 90, "duration": 60}, "dirty-hook-rate.json"),
    ("hook_rate", {"start": 240, "duration": 30}, "dirty-hook-rate.json"),
]


def _load(name: str) -> dict:
    return json.loads((DUB / name).read_text(encoding="utf-8"))


def _run(kind: str, requested: dict) -> dict:
    score = _load("score.json")
    time_map = _load("time-map.json")
    return dirty_from_signal(
        time_map["entries"],
        score["slots"],
        kind=kind,
        signal_range=requested,
        spill_threshold_frames=score["spill_threshold_frames"],
    )


def _match_fixture(got: dict, expected: dict) -> None:
    assert got["range"] == expected["signal"]["range"]
    assert got["slots"] == expected["slots"]
    assert got["kerfs"] == expected["kerfs"]
    assert got["clean"] == expected["clean"]
    assert got["mixes"] == expected.get("mixes", [])
    if "warnings" in expected:
        assert got["warnings"] == expected["warnings"]


class TestDubPermutations(unittest.TestCase):
    def test_note_cases_match_fixtures(self) -> None:
        for kind, requested, name in NOTE_CASES:
            with self.subTest(name=name, requested=requested):
                expected = _load(name)
                got = _run(kind, requested)
                _match_fixture(got, expected)
                self.assertNotIn("speed", json.dumps(got))
                self.assertNotIn("bed", got["slots"])

    def test_declared_windows_ignore_requested_range(self) -> None:
        for kind, requested, name in DECLARED_CASES:
            with self.subTest(kind=kind, requested=requested, name=name):
                expected = _load(name)
                got = _run(kind, requested)
                _match_fixture(got, expected)
                self.assertEqual(got["range"], expected["signal"]["range"])
                self.assertNotIn("body", got["slots"] if kind != "hold" else [])
                if kind == "hold":
                    self.assertIn("body", got["slots"])
                    self.assertNotIn("hook", got["slots"])
                if kind in {"vo_hold", "hook_rate"}:
                    self.assertNotIn("body", got["slots"])
                    self.assertIn("body", got["clean"])

    def test_every_dest_frame_note_equals_time_map_overlap(self) -> None:
        score = _load("score.json")
        entries = _load("time-map.json")["entries"]
        spine = [
            slot["id"]
            for slot in sorted(
                (s for s in score["slots"] if s["role"] in {"hook", "body", "proof", "cta"}),
                key=lambda s: (s["range"]["start"], s["id"]),
            )
        ]
        bound = {entry["slot"] for entry in entries}
        self.assertEqual(bound, {"hook", "body", "cta", "captions", "vo"})
        self.assertNotIn("bed", bound)
        for start in range(0, 300):
            addressed = {"start": start, "duration": 1}
            overlap = {
                entry["slot"]
                for entry in entries
                if overlap_frames(entry["dest"], addressed) > 0
            }
            got = _run("note", addressed)
            with self.subTest(start=start):
                self.assertEqual(got["range"], addressed)
                self.assertEqual(set(got["slots"]), overlap)
                self.assertEqual(
                    got["slots"],
                    [sid for sid in spine if sid in overlap]
                    + sorted(overlap - set(spine)),
                )
                self.assertEqual(got["clean"], [sid for sid in spine if sid not in overlap])
                kerfs = [
                    [spine[i], spine[i + 1]]
                    for i in range(len(spine) - 1)
                    if spine[i] in overlap or spine[i + 1] in overlap
                ]
                self.assertEqual(got["kerfs"], kerfs)
                if "vo" in overlap:
                    self.assertEqual(got["mixes"], ["audio_mix"])
                else:
                    self.assertEqual(got["mixes"], [])
                self.assertNotIn("bed", got["slots"])

    def test_declared_kinds_constant_across_every_requested_frame(self) -> None:
        expected = {
            "vo_hold": _load("dirty.json"),
            "hold": _load("dirty-hold.json"),
            "hook_rate": _load("dirty-hook-rate.json"),
        }
        for kind, fixture in expected.items():
            for start in range(0, 300, 1):
                got = _run(kind, {"start": start, "duration": 1})
                with self.subTest(kind=kind, start=start):
                    _match_fixture(got, fixture)

    def test_picture_time_map_omits_vo_from_every_kind(self) -> None:
        """Unbound vo is not addressable. Silence in unnamed picture is not a slot."""
        score = _load("score.json")
        entries = [
            entry
            for entry in _load("time-map.json")["entries"]
            if entry["slot"] != "vo"
        ]
        cases = [
            ("note", {"start": 90, "duration": 60}, ["body", "captions"], []),
            ("vo_hold", {"start": 0, "duration": 1}, ["hook", "captions"], []),
            ("hold", {"start": 90, "duration": 30}, ["body", "captions"], []),
            ("hook_rate", {"start": 0, "duration": 1}, ["hook", "captions"], []),
            ("note", {"start": 0, "duration": 90}, ["hook", "captions"], []),
            ("note", {"start": 210, "duration": 90}, ["cta", "captions"], []),
        ]
        for kind, requested, slots, mixes in cases:
            got = dirty_from_signal(
                entries,
                score["slots"],
                kind=kind,
                signal_range=requested,
                spill_threshold_frames=score["spill_threshold_frames"],
            )
            with self.subTest(kind=kind, requested=requested):
                self.assertEqual(got["slots"], slots)
                self.assertEqual(got["mixes"], mixes)
                self.assertNotIn("vo", got["slots"])
                self.assertNotIn("bed", got["slots"])
                if kind in {"vo_hold", "hook_rate"}:
                    self.assertNotIn("body", got["slots"])
                    self.assertIn("body", got["clean"])
                if kind == "hold":
                    self.assertIn("body", got["slots"])
                    self.assertNotIn("hook", got["slots"])
                if kind == "note" and requested["start"] == 90:
                    self.assertIn("body", got["slots"])
                if kind == "note" and requested["start"] in {0, 210}:
                    self.assertNotIn("body", got["slots"])

    def test_picture_every_dest_frame_never_names_vo_or_mix(self) -> None:
        score = _load("score.json")
        entries = [
            entry
            for entry in _load("time-map.json")["entries"]
            if entry["slot"] != "vo"
        ]
        vo_hold = None
        hold = None
        hook_rate = None
        for start in range(0, 300):
            addressed = {"start": start, "duration": 1}
            overlap = {
                entry["slot"]
                for entry in entries
                if overlap_frames(entry["dest"], addressed) > 0
            }
            self.assertNotIn("vo", overlap)
            self.assertNotIn("bed", overlap)
            note = dirty_from_signal(
                entries,
                score["slots"],
                kind="note",
                signal_range=addressed,
                spill_threshold_frames=score["spill_threshold_frames"],
            )
            with self.subTest(start=start):
                self.assertEqual(set(note["slots"]), overlap)
                self.assertEqual(note["mixes"], [])
                self.assertNotIn("vo", note["slots"])
                self.assertNotIn("bed", note["slots"])
            vo = dirty_from_signal(
                entries,
                score["slots"],
                kind="vo_hold",
                signal_range=addressed,
                spill_threshold_frames=score["spill_threshold_frames"],
            )
            hold_got = dirty_from_signal(
                entries,
                score["slots"],
                kind="hold",
                signal_range=addressed,
                spill_threshold_frames=score["spill_threshold_frames"],
            )
            hook = dirty_from_signal(
                entries,
                score["slots"],
                kind="hook_rate",
                signal_range=addressed,
                spill_threshold_frames=score["spill_threshold_frames"],
            )
            if vo_hold is None:
                vo_hold, hold, hook_rate = vo, hold_got, hook
            with self.subTest(declared=start):
                self.assertEqual(vo["slots"], vo_hold["slots"])
                self.assertEqual(hold_got["slots"], hold["slots"])
                self.assertEqual(hook["slots"], hook_rate["slots"])
                self.assertNotIn("vo", vo["slots"])
                self.assertNotIn("vo", hold_got["slots"])
                self.assertNotIn("vo", hook["slots"])
                self.assertEqual(vo["mixes"], [])
                self.assertEqual(hold_got["mixes"], [])
                self.assertEqual(hook["mixes"], [])
                self.assertNotIn("body", vo["slots"])
                self.assertNotIn("body", hook["slots"])
                self.assertIn("body", hold_got["slots"])

    def test_join_frames_are_exclusive_at_90_and_210(self) -> None:
        at_89 = _run("note", {"start": 89, "duration": 1})
        at_90 = _run("note", {"start": 90, "duration": 1})
        at_209 = _run("note", {"start": 209, "duration": 1})
        at_210 = _run("note", {"start": 210, "duration": 1})
        self.assertIn("hook", at_89["slots"])
        self.assertNotIn("body", at_89["slots"])
        self.assertIn("body", at_90["slots"])
        self.assertNotIn("hook", at_90["slots"])
        self.assertIn("body", at_209["slots"])
        self.assertNotIn("cta", at_209["slots"])
        self.assertIn("cta", at_210["slots"])
        self.assertNotIn("body", at_210["slots"])


if __name__ == "__main__":
    unittest.main()
