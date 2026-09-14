# RFC 0002 — overlay joins and mix

- Status: accepted
- Date: 2026-09-14
- Implements: ADR 0008
- Format id: `0.2.0` (additive)

## Problem

Schema `0.2.0` already names eight roles and compile.md already names
`cut` or a named fade. Lowering only walks spine slots and hardcodes
`transition = cut`. Non-spine roles never enter the action graph. A hook
swap that should leave `vo`, `bed`, `captions`, and `brand` untouched
cannot, because those actions do not exist.

## Decision

Format id stays `0.2.0`. Additive optional fields only.

1. Optional `score.joins[]`: `{left, right, transition, duration_frames}`.
   Missing join is `cut`. Allowed transitions this increment: `cut`,
   `fade`. Unknown names fail lower with a machine-readable reason.
2. Fade is a sequential fade-out / fade-in of `duration_frames` on each
   side of the join (kerf is `2 * duration_frames`). Concat trims that
   many frames from the adjacent slot encodes so dest duration still
   matches the score clock. Changing only the fade dirties that kerf and
   concat, not adjacent `slot_encode` blobs.
3. Spine stays picture `slot_encode` + `kerf` + `concat`.
4. `vo` and `bed` lower to independent `audio_encode` actions on their
   score ranges, then one `audio_mix` over spine AAC plus overlays.
5. `captions` is a sidecar (`Kind` `captions`, WebVTT/SRT next to the
   dest). Not burned in.
6. `brand` is an `overlay_mix` on the concat product. The mix miss
   re-encodes the linker product only; body `slot_encode` stays a hit.
7. Time map entries include every bound slot so a signal can address a
   vo or caption window. `hook_rate` still does not dirty `body`.

## Compatibility

- Recipe, essence, and build storage stay distinct.
- Signal dirtiness stays separate from action-cache dirtiness.
- Guest adapters remain lossy and depend on `graft-score` only.
- Closed-GOP `cut` remains the empty-kerf IDR path.
- Load-bearing hook-rate tests stay.
