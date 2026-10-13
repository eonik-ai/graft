# RFC 0009 — motion keyframes, time remap, multi-cam switch

- Status: accepted, implemented
- Date: 2026-09-19
- Implements: ADR 0010, [composition.md](../composition.md) C1 / C6 / §5 / §17
- Format id: `0.2.0` (additive)

## Problem

RFC 0008 placed a PIP at static `(x, y)`. Hosts author motion, speed
ramps, and angle switches. Those still fell out of the loop:

1. Overlay `x`/`y` are strings, but there is no keyframe object.
   Address / swap named a file rebind, not a place edit.
2. `params.speed` is a constant. A dest-time speed curve is a different
   working-source key. Intra cannot lower it.
3. `streamselect` is a static `map`. A host multi-cam cut list has no
   porcelain and no timed lowerer.

## Decision

1. Optional `binding.motion` is `{x, y, opacity}` arrays of
   `{t, v}` (dest seconds, linear). Compile lowers `x`/`y` to overlay
   expressions. `opacity` keys stay stepwise via the same mix path when
   a single value; multi-key opacity is a `geq` alpha expression.
   Static `binding.x` / `y` / `opacity` remain. Motion fingerprints
   `overlay_mix` only. Porcelain slot `motion`. `graft swap overlay
   --x --y` updates place without a file.
2. Optional `binding.remap` is `[{t, v}]` with `v` = speed, stepwise
   (hold until the next key). The working-source key includes it.
   Encode folds source trims at each speed and concats working pads.
   Dest duration is the score slot. Source starts at `binding.source`.
   Intra fails closed on a non-constant remap. Porcelain slot `remap`.
3. Graph op `switch` takes N picture inputs and `params.cuts`
   `[{t, map}]`. It folds later inputs over the first with
   `overlay` + `enable=gte(t,…)`. Not `sendcmd` (command syntax is
   filter-specific). Porcelain `switch.cuts` in a nest authors the
   first cut as the spine slot, the rest as `source` nodes, then
   `switch`. `graft address` prefers `angles.switch` when present.
4. Ops this ffmpeg can lower and hosts use: `geq` (garbage / holdout
   / opacity expr), `selectivecolor`, `exposure`, `colorhold`,
   `colorize`, `deband`.

## Not this increment

- `drawtext` / OCIO / `subtitles` (C11).
- Whip / flash; HSL blends.
- In-graph subgraph objects (adjustments still flatten to `enable=`).
- Optical-flow `minterpolate` as a default remap (available as a
  node later; remap stays trim+setpts).
- Easing curves beyond linear / stepwise.
