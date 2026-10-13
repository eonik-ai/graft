# Compile

The compiler turns a scion into a dest file. It is incremental.

The compile IR is an **action graph**. Dirty is “does this action key
have an artifact?”, not “diff two scion JSON files.” See ADR 0006 and
RFC 0003.

The default ffmpeg path is a **working-depth graph** plus one delivery
encode (ADR 0010, [composition.md](composition.md)). Sources normalize
to `dest.working` (default `yuv422p10le` + dest color, range `tv`,
ProRes HQ) and PCM 48 kHz stereo. Joins are decoded `concat` or overlap `xfade` /
`acrossfade`. `binding.nested` / graph `nested` compile an inner scion
to its working `#picture` (action-cached; never the delivery concat)
and bind that pad on the parent. `scion.graph` nodes append after `#picture` / `#audio`,
or replace the synthesized spine. A `group` is expanded first: children
inherit `enable=` and each child is its own action. Filter-shaped ops
this ffmpeg has lower generically; a missing filter is `toolchain`
(RFC 0010). The node key includes `keys` when set. Delivery (`dest.pix_fmt` + encoder)
runs once at the sink.

`graft-intra` stays a frame splice. `concat -c copy` of H.264 cells is
a special case only when no downstream filter exists — not this path.

## Two hashes

Hash function: **BLAKE3**. Canonical encoding: UTF-8 JSON, sorted keys,
no insignificant whitespace, numbers as decimal seconds with a fixed
scale (milliseconds — 3 decimal places — unless an RFC says otherwise).

| Type | What is hashed | Display |
| --- | --- | --- |
| `ActionKey` | canonical JSON below | 64 hex |
| `BlobId` | **bytes** of a material or working/delivery blob | `blake3:<hex>` |

Do not use one string for both. Mixing them is how “git for video” fails.

Delivery output is kind `concat`. **Never** hash the shipped file as
essence.

## Action keys

Working keys include dest size, rate, and working depth. They do
**not** include the delivery encoder.

### working_source

```
working_source := H(
  slot.id | slot.role | material.hash | source | audio | params | working
)
```

`working` = `{width, height, rate, working_pix_fmt, working_color, working_range}`.
The source node trims `[in, out)`, scales, `fps`, `format`, and
`setparams` so mixed geometry and untagged color can join.

### audio_encode

Working PCM (`pcm_s24le`, 48 kHz, stereo). Same working fingerprint.
`vo` / `bed` / `sfx` are overlay pads placed on the **compiled** dest
clock.

### kerf / join

```
kerf := H(
  left.working_source | right.working_source | transition | duration_frames | working
)
```

`transition` is `cut`, `fade` (xfade `fade`), or any ffmpeg `xfade`
name. `fade` is an **overlap**. `duration_frames` is the overlap.
Dest length shortens by each overlap; the time map records compiled
dest ranges. Pair keys stay independent so a hook swap still hits
body→cta in the plan. The folded working spine is a separate node.

### working_spine

```
working_spine := H( ordered working_source[] | joins[] | working )
```

Cut folds with the `concat` filter. Overlap folds with `xfade` and
`acrossfade`. Changing the delivery encoder does not miss this key.

### working_node

```
working_node := H( op | id | input keys | extra | working )
```

`extra` is material, source, place, enable, role, params. Used for
`scion.graph` (`lut3d`, `overlay`, `gap`, …).

### deliver

```
deliver := H( picture_key | audio_key | dest | encoder )
```

`encoder` includes impl, version, profile, level, rate_control, preset,
keyint, `sc_threshold`, and toolchain. A dest encoder change misses
**delivery** only. Clean working sources and the working spine stay
hits.

`sc_threshold` is **0** and `keyint` is **fixed** so GOP layout of the
sink is deterministic across machines.

### captions / overlay_mix / audio_mix

Sidecar captions. Brand is an `overlay` on the working picture at
`binding.x` / `y` / `opacity` / `binding.motion` (mix key, not source key).
`binding.remap` is on the working-source key.
Overlay audio mixes after the spine fold.

The time map is a **build artifact** keyed by the deliver action (dest
clock, compiled ranges). It is not an identity derived only for the CLI.

## Dirty set

The dirty oracle is the **action cache**:
`ActionKey → ActionResult { kind, blob, size, metadata, logs, provenance }`.
A miss is an absent key, or a key whose blob is missing from the
namespaced CAS.

`graft dirty` / `graft compile` read that cache. They do not compare a
previous scion document. `--prev` may print a debug overlay.

`graft signal` is a different dirty set (time map ∩ metric). Do not
collapse it into the action cache.

Schedule:

1. Recompute each working-source key. Miss → normalize that range.
2. Pair joins: miss if either side’s source key or the transition
   changed. Body source stays a hit when only the hook changes.
3. Working spine: miss if any source or join on the fold changed.
4. Overlay: `vo`/`bed`/`sfx`, `captions`, brand. A caption or SFX miss
   does not recut a clean body source.
5. `scion.graph` nodes: miss if inputs, params, or `keys` changed.
   A `group` expands before the key, so a child misses alone.
   A child that omits `graph` inherits the parent nodes (same keys).
6. Deliver: miss if the picture/audio key or the encoder changed.

`graft ship` writes look/legal nodes and `score.joins` from the take
directory. `graft preview` is this compile. The plan JSON lists `nodes[]`
and `concat_cache` (delivery).

Retiming a slot (speed ≠ 1, or span length change) invalidates that
source key; the slot re-encodes in full. `graft-intra` resamples
the sliced frame pack to the dest frame count (`source_duration / speed`).
`FfmpegX264` applies `setpts` and chained `atempo` so picture and PCM
stay locked. Feedback still does not invent a speed.

## Grain

| Working format | Independent unit | Replace hook | Retime ("too slow") |
| --- | --- | --- | --- |
| Image seq / DPX / EXR / JPEG2000 / `graft-intra` | frame | those frames | new essence for the slot |
| Working ProRes HQ / PCM (ffmpeg path) | node output | that source node | that source node |
| ProRes / DNxHR / All-I | frame | grain-table intent (`stsd` splice is not implemented) | re-encode that slot |
| H.264/HEVC/AV1 Long-GOP | GOP / IDR (~0.5–2s) | hook subgraph + downstream nodes | full slot re-encode |
| HLS / CMAF | segment aligned to GOP | replace hook segments | new segments + playlist |

Intra is proven: `SlotEncode` copies `[in, out)` frames into CAS;
kerf is empty; concat splices.

Long-GOP (`impl: x264`): graft shells out to **system** ffmpeg/`libx264`
with `keyint` = `min-keyint`, `scenecut=0`, `threads=1`. Working nodes
are ProRes HQ (picture) and PCM (sound). The sink is one `libx264` + AAC
encode. A post-join LUT re-filters the joined working picture; it does
not recode a clean body source.

Do not statically link x264 into the Apache-2.0 binary.

## Worked compile

Previous ship: `hook_v2 + body_v1 + cta_v1 @ 9x16`.
Now: `hook_v3` on the same body and cta.

| Piece | Cache |
| --- | --- |
| hook source | MISS (new material hash → new ActionKey) |
| body source | HIT (same ActionKey; body BlobId unchanged) |
| cta source | HIT |
| join hook→body | MISS (left changed) |
| join body→cta | HIT (pair keys are the two sources) |
| working spine | MISS (hook source changed) |
| deliver | MISS |

The body working blob is reused. Downstream nodes that take the new
spine (LUT, mix, delivery) miss. That is “keep the body subgraph.”

## What not to build

- xdelta / rsync / FastCDC on the delivery mp4
- MP4 `elst` as the composition model (poor support; hidden keyframe lead-in)
- A second, cloud-only IR
- Diffing two `scion.json` files as the cache
