# RFC 0007 — nested scion as a working pad

- Status: accepted, implemented
- Date: 2026-09-19
- Implements: ADR 0010, [composition.md](../composition.md) C4 / C9 / nested sequence
- Format id: `0.2.0` (additive)

## Problem

A host nest is a composition used as a clip, not a file. Binding only a
library range cannot hold a plate that has its own look, joins, and
clock. Baking the inner **delivery** mp4 as outer essence violates C4
(working depth ≠ delivery) and C5 (the shipped mp4 is never hashed as
essence). Intra splicing a nested pad is a lie.

## Decision

1. Optional `binding.nested` is a scion id. Compile resolves it to the
   inner **working** `#picture` (and working audio when present) and
   uses those blobs as the slot material. The stored `material` is the
   last resolved id or a placeholder; ActionKeys use the resolved bytes.
2. Graph op `nested` (`params.scion`) is a source-like pad. It does not
   steal `#picture`. Hosts wire it as an overlay / matte / stack input.
3. `compile_in` takes a nest family (`nests/<id>/score.json` +
   `scions/<id>.json`, own concept). Recurse with a visiting set; a
   cycle fails closed. Inner compile is the same compiler.
4. Working picture is cached under an action key of the inner
   `picture_key`. Concat HIT still returns that pad. Delivery concat is
   not nested essence.
5. Porcelain `composition.json` `nests` authors the inner score/scion.
   A parent slot `{ "id": "hook", "scion": "plate" }` binds
   `nested: "plate"`. `graft swap look` on a nest scion mutates that
   scion in place (nests are clips, not dest variants). Parent compile
   then misses the nested pad; sibling file-range slots stay HIT.
6. OTIO sidecar carries `nested`. Intra fails closed on a nested pad.
7. `validate_family` skips scions whose `concept` is not the score's
   (nests live next to dest scions in `scions/`).

## Not this increment

- `drawtext` / OCIO / `subtitles` (C11).
- Whip / flash; HSL blends.
- Nested filtergraph objects inside one scion (a subgraph that is not
  another scion). Adjustment spans stay `enable=` on nodes.
- Multi-cam switch as a host verb (`streamselect` node already exists).
