# SPDX-License-Identifier: Apache-2.0
"""Signal → dirty set. This module is part of the spec, not a sketch."""

from __future__ import annotations

from typing import Any

SPINE_ROLES = frozenset({"hook", "body", "proof", "cta"})
DEFAULT_SPILL_S = 0.35


def overlap_s(a0: float, a1: float, b0: float, b1: float) -> float:
    lo = max(a0, b0)
    hi = min(a1, b1)
    return max(0.0, hi - lo)


def dirty_from_signal(
    time_map: list[dict[str, Any]],
    slots: list[dict[str, Any]],
    *,
    kind: str,
    t0_s: float,
    t1_s: float,
    spill_threshold_s: float = DEFAULT_SPILL_S,
) -> dict[str, Any]:
    """Return {slots, kerfs, clean, warnings} for one signal on one build."""
    by_id = {slot["id"]: slot for slot in slots}
    overlaps: dict[str, float] = {}
    dirty: set[str] = set()

    for entry in time_map:
        start, end = entry["span"]
        ov = overlap_s(start, end, t0_s, t1_s)
        if ov > 0:
            dirty.add(entry["slot"])
            overlaps[entry["slot"]] = ov

    warnings: list[str] = []
    if kind == "hook_rate":
        for slot in slots:
            if slot.get("role") == "hook":
                dirty.add(slot["id"])
        for slot_id in list(dirty):
            slot = by_id[slot_id]
            if slot.get("role") == "hook":
                continue
            ov = overlaps.get(slot_id, 0.0)
            if ov < spill_threshold_s:
                dirty.discard(slot_id)
            elif ov > 0:
                warnings.append(
                    f"{slot_id} overlaps hook_rate window by {ov:.3f}s "
                    f"(>= spill_threshold_s {spill_threshold_s}); "
                    "pad hook to the declared window or accept a dirty body"
                )

    spine = [
        slot
        for slot in sorted(slots, key=lambda s: (s["span"][0], s["id"]))
        if slot.get("role") in SPINE_ROLES
    ]
    kerfs: list[list[str]] = []
    for left, right in zip(spine, spine[1:]):
        if left["id"] in dirty or right["id"] in dirty:
            kerfs.append([left["id"], right["id"]])

    clean = [slot["id"] for slot in spine if slot["id"] not in dirty]
    ordered_dirty = [
        slot["id"] for slot in spine if slot["id"] in dirty
    ]
    # include non-spine dirty slots (vo, captions) after the spine
    for slot_id in sorted(dirty):
        if slot_id not in ordered_dirty:
            ordered_dirty.append(slot_id)

    return {
        "slots": ordered_dirty,
        "kerfs": kerfs,
        "clean": clean,
        "warnings": warnings,
    }
