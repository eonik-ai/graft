# RFC 0005 — host pads and guest sidecar

- Status: accepted, implemented
- Date: 2026-09-19
- Implements: ADR 0010, [composition.md](../composition.md) §4.3 / §17
- Format id: `0.2.0` (additive)

## Problem

A host already authors extra picture tracks, host-named transitions,
adjustment spans, and markers. Porcelain that only overlays `brand`
and a guest export that drops `scion.graph` is not a closed loop.
`graft-intra` silently splicing a graph is a lie.

## Decision

1. Role `overlay` is a dest-placed picture pad (PIP / B-roll). `brand`
   stays. Both lower to sequential `overlay` on `#picture`.
2. `joins.transition` accepts host aliases (`dissolve`, `dipToBlack`,
   `wipeLeft`, …) and stores the ffmpeg `xfade` name. `whip*` / `flash`
   fail closed (guest, no lowerer).
3. Optional `score.markers[]` (`id` + dest range). OTIO exports them.
   They do not dirty picture.
4. OTIO `metadata.graft` carries `graph` and `joins`. Import restores
   `scion.graph`. Overlay / VO / captions / brand bindings export as
   clips. Native OTIO effects stay lost; the graft sidecar is the recipe.
5. Address lists covering graph nodes (`enable=` overlap or dest-wide).
   `next` names a slot when one is dirty, else the first covering node.
6. `graft-intra` fails if `scion.graph` is non-empty.
7. Graph ops this ffmpeg has: `setsar`, `setdar`, `perspective`,
   `shear`, `scroll`, `premultiply`, `streamselect`.

## Not this increment

- Nested scion-as-pad (needs a workspace of working blobs).
- `drawtext` / OCIO / `subtitles` (C11).
- Whip / flash lowerers.
