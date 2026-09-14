# 0008 — overlay actions and named fades

- Status: accepted
- Date: 2026-09-14
- Extends: ADR 0006 (does not edit its Decision)

## Context

ADR 0006 froze the compile IR as spine `SlotEncode`, a `Kerf` per
adjacent pair with `transition = cut`, and one `Concat`. That was enough
to prove hook swap without recutting the body. Compile.md already said
the kerf transition is `cut` or a named fade. Principles §6 already
named `vo`, `captions`, `bed`, and `brand` as addressable roles.

Those claims were false in code: lower hardcoded `"cut"`, and
`score.spine()` dropped every non-spine role. Filling them in the same
graph keeps device and server on one IR (ADR 0004).

## Decision

1. A `Kerf` still exists for every adjacent spine pair. Its transition
   is `cut` (default) or `fade` from optional `score.joins`. Fade
   duration is part of the kerf action key. Intra blends frames; ffmpeg
   encodes a real kerf blob. Closed-GOP `cut` at an IDR join stays an
   empty artifact.
2. Non-spine roles are sibling actions, not extra spine concat parts:
   `vo`/`bed` → `audio_encode` + `audio_mix`; `captions` → sidecar
   blob; `brand` → `overlay_mix` on the picture concat. Changing the
   hook must not rewrite those blobs.
3. The time map lists every bound slot. `graft signal` may address a
   non-spine window. `hook_rate` still does not dirty `body`.
4. Captions are not burned into picture. Layers remain binding opinions;
   layer name `grade` does not invent a pixel grade.

This does not change ADR 0006's Decision text. It extends the same
action-cache dirty oracle to mix and fade nodes.

## Consequences

- Contributors add mix/overlay backends against `Action` + CAS.
- A fade or brand change misses concat/mix, not a clean body
  `slot_encode`.
- Schema format id stays `0.2.0` with optional `joins`.
