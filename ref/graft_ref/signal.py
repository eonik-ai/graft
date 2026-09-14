# SPDX-License-Identifier: Apache-2.0
"""Reference signal → dirty-set rule on rational frame ranges."""

from __future__ import annotations

from typing import Any

SPINE_ROLES = frozenset({"hook", "body", "proof", "cta"})
DEFAULT_SPILL_FRAMES = 11


def _end(value: dict[str, int]) -> int:
    return value["start"] + value["duration"]


def overlap_frames(a: dict[str, int], b: dict[str, int]) -> int:
    return max(0, min(_end(a), _end(b)) - max(a["start"], b["start"]))


def dirty_from_signal(
    time_map: list[dict[str, Any]],
    slots: list[dict[str, Any]],
    *,
    kind: str,
    signal_range: dict[str, int],
    spill_threshold_frames: int = DEFAULT_SPILL_FRAMES,
) -> dict[str, Any]:
    """Return semantic slots/kerfs for one signal on one exact build."""
    declared = next(
        (
            slot["window"]["range"]
            for slot in slots
            if slot.get("window", {}).get("kind") == kind
        ),
        None,
    )
    addressed = declared or signal_range
    by_id = {slot["id"]: slot for slot in slots}
    overlaps: dict[str, int] = {}
    dirty: set[str] = set()

    for entry in time_map:
        overlap = overlap_frames(entry["dest"], addressed)
        if overlap > 0:
            dirty.add(entry["slot"])
            overlaps[entry["slot"]] = overlap

    warnings: list[str] = []
    if kind == "hook_rate":
        dirty.update(slot["id"] for slot in slots if slot.get("role") == "hook")
        for slot_id in sorted(dirty):
            slot = by_id[slot_id]
            if slot.get("role") == "hook":
                continue
            overlap = overlaps.get(slot_id, 0)
            if overlap < spill_threshold_frames:
                dirty.discard(slot_id)
            elif overlap:
                warnings.append(
                    f"{slot_id} overlaps hook_rate window by {overlap} frames "
                    f"(>= spill_threshold_frames {spill_threshold_frames}); "
                    "pad hook to the declared window or accept a dirty body"
                )

    spine = sorted(
        (slot for slot in slots if slot.get("role") in SPINE_ROLES),
        key=lambda slot: (slot["range"]["start"], slot["id"]),
    )
    kerfs = [
        [left["id"], right["id"]]
        for left, right in zip(spine, spine[1:])
        if left["id"] in dirty or right["id"] in dirty
    ]
    clean = [slot["id"] for slot in spine if slot["id"] not in dirty]
    ordered = [slot["id"] for slot in spine if slot["id"] in dirty]
    ordered.extend(sorted(dirty - set(ordered)))
    mixes: list[str] = []
    for slot_id in ordered:
        role = by_id.get(slot_id, {}).get("role")
        if role in {"vo", "bed"} and "audio_mix" not in mixes:
            mixes.append("audio_mix")
        if role == "brand" and "overlay_mix" not in mixes:
            mixes.append("overlay_mix")
    return {
        "range": addressed,
        "slots": ordered,
        "kerfs": kerfs,
        "clean": clean,
        "mixes": mixes,
        "warnings": warnings,
    }
