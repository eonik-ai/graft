# RFC 0008 — adjustment groups, overlay place, nest address, color convert

- Status: accepted, implemented
- Date: 2026-09-19
- Implements: ADR 0010, [composition.md](../composition.md) C3 / C9 / §5 / §17
- Format id: `0.2.0` (additive)

## Problem

RFC 0007 made a nest a working pad. The porcelain loop still dropped
hosts there:

1. `graft address` covering nodes were the parent graph only. A dirty
   nested hook named `graft swap hook <file>` — a file rebind, not
   `graft swap look --scion plate`.
2. Host `AdjustmentSpan` is a group of filters on a dest-time span.
   Nodes could set `enable=` one by one. Porcelain had no group.
3. Role overlay / brand always mixed at `(0,0)`. Host PIP is `(x,y)`
   plus opacity. Putting those on `binding.params` would dirty
   `slot_encode` (source trim) when only the mix geometry changed.
4. `colorspace` only sent `all=`. Primaries / TRC convert was absent.
   CDL and lift/gamma/gain had no ops even though this ffmpeg has
   `colorcorrect`, `colorchannelmixer`, and `eq`.

## Decision

1. `graft address` loads the nest family. A dirty slot whose binding is
   `nested` contributes inner covering nodes as `plate.look`. The nest
   clock is the cover range (the whole inner dest is dirty when the
   parent pad is). `next` is `graft swap look <file> --scion plate`
   when such a node exists.
2. Porcelain `composition.json` `adjustments[]` is
   `{ enable, nodes[] }`. A node object is authored with that
   `enable=`. A string id stamps `enable=` on an existing node. The
   stored IR is still flat `scion.graph` nodes. Not a nested graph
   object.
3. Optional `binding.x` / `binding.y` / `binding.opacity` are mix
   geometry. They are not in the working-source key. `overlay_mix`
   fingerprints them. Execute uses them. Porcelain slot `x` / `y` /
   `opacity`. `graft swap` of the overlay file keeps the place. OTIO
   sidecar carries the three fields.
4. `colorspace` params: `all`, `primaries`, `trc` (and `iprimaries` /
   `itrc` when set). Optional `dest.working.primaries` / `trc` enter
   the working fingerprint only when present.
5. Graph ops this ffmpeg can lower: `colorcorrect`, `cdl` (slope /
   offset / power / sat → `colorchannelmixer` + `eq`), `lgg` (lift /
   gamma / gain / sat → `colorcorrect` + `eq`), `bwdif`, `tile`,
   `haldclut` (second input; PNG loops).

## Not this increment

- `drawtext` / OCIO / `subtitles` (C11).
- Whip / flash; HSL blends.
- In-graph subgraph objects (adjustment groups still flatten to
  `enable=`).
- Multi-cam switch as a host verb (`streamselect` node already exists).
- Keyframes / motion-path IR (overlay `x`/`y` may be ffmpeg expressions;
  there is no keyframe object).
