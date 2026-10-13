# 0010 — composition is a timestamped filtergraph

- Status: accepted
- Date: 2026-09-19
- Extends: ADR 0002, ADR 0006, ADR 0008 (does not edit their Decision text)
- Refines: principles 4, 6, 8, 14 — see [principles.md](../principles.md) §16
- Inventory: [composition.md](../composition.md)

## Context

ADR 0006 froze compile as spine `SlotEncode` → `Kerf` → `Concat`, with
delivery H.264 as the cached unit. ADR 0008 added sequential fade kerfs
and overlay mix. Principle 14 read that as: a new hook must not
re-encode a clean body.

That is only true when the composition **is** a concat of already-looked
chunks and nothing sits after the join. A real ad is LUT, overlap
transitions, SFX, titles, mix, and a codec — timestamped, at working
depth. ffmpeg already evaluates that as `filter_complex` then an
encoder. Bitstream-copy of 8-bit slot GOPs forbids a post-join look.
The first host (eonik) already authors those objects; it does not
export `takes/hook.mov`.

## Decision

1. The composition IR is a **timestamped filtergraph**. Nodes lower to
   libavfilter (or a documented host raster that becomes a pad). The
   codec is the sink, not a clip. Roles remain addressing labels on dest
   time, not the graph.
2. Action-cache units are **working node outputs** (principle 5 still
   holds: recipe / essence / build). Delivery encode is one sink action.
   `concat -c copy` of closed-GOP H.264 is a special case allowed only
   when no downstream filter exists and pads already match.
3. “Keep the body” means the body **subgraph** hits. A look, xfade, or
   spanning mix downstream of the join may still re-filter those pixels.
   Re-running a clean subgraph to change an unrelated pad remains
   incorrect.
4. The living list of dimensions is [composition.md](../composition.md).
   Agents read it before IR work. New families are appended there. A
   node that cannot lower to ffmpeg is not added.

This does not change ADR 0006’s Decision text. It extends the same
action-cache oracle from “slot file” to “filter-subgraph node.” Schema
0.2.0 stays until an RFC grows pads.

## Consequences

- `grade` / `legal` layer names still do not invent pixel ops (ADR 0008).
  When they do, they become filters in composition.md with a schema RFC.
- Porcelain `takes/` is a reference crutch, not authorship.
- Encoder fingerprint includes filter availability (no `drawtext` /
  `lut3d` on some builds).
- OTIO guests still lose effects until those pads exist.
