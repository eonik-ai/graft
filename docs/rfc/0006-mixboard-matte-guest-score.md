# RFC 0006 — mixboard, matte, guest score restore

- Status: accepted, implemented
- Date: 2026-09-19
- Implements: ADR 0010, [composition.md](../composition.md) §5 / §10 / C12
- Format id: `0.2.0` (additive)

## Problem

A host authors track mattes, stacks, blend-mode names, a mixboard, and
markers that should survive a guest. OTIO already carries `joins` and
`markers` in the sidecar; import only restored `scion.graph`. The dest
did not mux chapters. This ffmpeg already has the filters.

## Decision

1. OTIO import upserts sidecar `joins` and `markers` onto the score.
2. Graph ops this ffmpeg has: `maskedmerge`, `hstack`, `vstack`,
   `xstack`, `mix`, `unpremultiply`, `lenscorrection`, `yadif`.
3. Host blend aliases (`hardLight` → `hardlight`, `colorDodge` →
   `dodge`, …). HSL channel blends stay guest.
4. Audio mixboard: `acompressor`, `alimiter`, `equalizer`, `highpass`,
   `lowpass`, `bass`, `treble`, `adelay`, `aecho`, `areverse`, `pan`.
5. `score.markers` mux as dest chapters. Delivery key includes them
   only when non-empty.
6. Porcelain graph nodes may set `in` / `out` so `source` pads exist.

## Not this increment

- Nested scion-as-pad.
- `drawtext` / OCIO / `subtitles` (C11).
- Whip / flash; HSL blends (`hue` / `saturation` / `color` /
  `luminosity`).
