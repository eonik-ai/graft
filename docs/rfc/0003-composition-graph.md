# RFC 0003 — composition graph

- Status: accepted
- Date: 2026-09-19
- Implements: ADR 0010
- Format id: `0.2.0` (additive)

## Problem

Schema `0.2.0` and `ffmpeg-libx264-closed-gop-v1` version eight named
H.264 cells and bitstream-copy them. That cannot version a LUT, an
overlap transition, a spanning SFX, or mixed color tags. Principle 16
and [composition.md](../composition.md) name the real object: a
timestamped filtergraph, working depth, delivery at the sink.

## Decision

Format id stays `0.2.0`. Additive fields. The default ffmpeg compile
path changes.

1. Optional `dest.working`: `{pix_fmt, color}`. Default
   `yuv422p10le` + dest color, range `tv` (ProRes HQ). Request
   `yuv444p10le` for keying. Source nodes normalize to this. Delivery
   `dest.pix_fmt` / encoder apply **once** at the sink.
2. Optional `scion.graph`: `{mode, nodes[]}`. `mode` is `append`
   (default) or `replace`. Append adds nodes after the synthesized
   spine (`#picture`, `#audio`). Replace is the full graph.
3. A node is `{id, op, inputs?, material?, source?, place?, enable?,
   role?, params?}`. `op` lowers to libavfilter (or sidecar). Unknown
   `op` fails closed. Materials (`.cube`, SFX) are CAS essence.
4. `score.joins.transition` is a string: `cut`, `fade` (xfade `fade`),
   or any ffmpeg `xfade` name. `fade` is an **overlap** xfade, not
   sequential fade-out/fade-in. `duration_frames` is the overlap.
   Dest length is shortened by each overlap; the time map records
   compiled dest ranges.
5. Role `sfx` is an overlay audio pad like `vo`/`bed`, placed on dest
   time (one-shot or span).
6. Action-cache units for picture/audio pads are **working** blobs
   (ProRes HQ / PCM). The key is node + params + input blob ids + working
   format. It does **not** include the delivery encoder. Delivery is
   `Kind::Concat` (sink) and includes the encoder fingerprint.
7. `concat -c copy` of H.264 slots is removed from the default ffmpeg
   path. It remains a documented special case only when no downstream
   filter exists (not this increment).

Slots and signal windows stay. Roles stay addressing labels. Layers
`grade`/`legal` stay binding opinions unless a graph node lowers them.

## Compatibility

- Intra (`graft-intra`) keeps frame splice. Graph ops that need ffmpeg
  are ffmpeg-only.
- Load-bearing `hook_rate` tests stay (signal dirty-set ≠ action cache).
- A dest encoder change misses **delivery**, not clean working sources.
- Existing examples without `graph` synthesize a graph from slots.

## Not this increment

- OCIO / libplacebo / `drawtext` when the binary lacks them (C11).
- Host raster of eonik whip/flash (guest until a lowerer exists).
- IMF emit.
