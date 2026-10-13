# RFC 0004 — J/L audio, keys, shapes, caption mux

- Status: accepted
- Date: 2026-09-19
- Implements: ADR 0010, [composition.md](../composition.md) §1 / §5 / §7
- Format id: `0.2.0` (additive)

## Problem

A host authors picture and sound on different dest ranges (J/L cuts),
chroma keys, boxes, and closed captions. Porcelain that only binds
picture `in`/`out` and leaves captions next to the dest is not a closed
loop. ffmpeg on this machine can lower those pads. `drawtext` / OCIO
still cannot (C11).

## Decision

Format id stays `0.2.0`. Additive fields only.

1. Optional `binding.audio.place` is a dest-clock `FrameRange`. When it
   is set, or when the audio source duration is not the picture dest
   duration, spine audio is dest-placed (`adelay` + `amix`) and is not
   concat-locked to the picture cut. Missing `place` with matching
   durations keeps today’s concat / `acrossfade`.
2. Graph ops that this ffmpeg has: `chromakey`, `colorkey`, `lumakey`,
   `chromahold`, `alphamerge`, `drawbox`, `drawgrid`, `colorchannelmixer`.
   Overlay `params.opacity` lowers to `colorchannelmixer=aa=`. Unknown
   `op` still fails closed.
3. Captions stay a sidecar blob and are also muxed into the delivery
   mp4 (`mov_text`). Burned-in text stays **absent** (no `drawtext` /
   `subtitles` on this binary).
4. Porcelain `composition.json` `slots[]` may set `audio` (`in` / `out`
   / `place`) and a `captions` role file. `graft swap` of a picture
   range keeps the parent J/L audio offset.

## Compatibility

- Intra stays a splice. These ops are ffmpeg-only.
- `hook_rate` still does not dirty `body`.
- Examples without `audio.place` are unchanged.

## Not this increment

- OCIO / libplacebo / `drawtext` / `subtitles` (toolchain).
- Nested graphs, multi-cam, whip/flash (no lowerer).
- Three-pad `maskedmerge` (use `alphamerge` + a mask pad).
