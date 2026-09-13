# Compile

The compiler turns a scion into a dest file. It is incremental.

The compile IR is an **action graph**. Dirty is “does this action key
have an artifact?”, not “diff two scion JSON files.” See ADR 0006.

## Two hashes

Hash function: **BLAKE3**. Canonical encoding: UTF-8 JSON, sorted keys,
no insignificant whitespace, numbers as decimal seconds with a fixed
scale (milliseconds — 3 decimal places — unless an RFC says otherwise).

| Type | What is hashed | Display |
| --- | --- | --- |
| `ActionKey` | canonical JSON below (`slot_encode`, `kerf`, `scion_hash`) | 64 hex |
| `BlobId` | **bytes** of a material, slot_encode file, or kerf file | `blake3:<hex>` |

Do not use one string for both. Mixing them is how “git for video” fails.

Concat output is a linker product. Store it as kind `concat` if at all.
**Never** hash the shipped file as essence.

## Action keys

### slot_encode

```
slot_encode := H(
  slot.id | slot.role | material.hash | source | audio | params | dest | encoder
)
```

`dest` = `{width, height, rate, pix_fmt, color}`.

`encoder` includes impl, version, profile, level, rate_control, preset,
keyint, `sc_threshold`, and toolchain (ffmpeg/libx264 digest, threads,
closed-GOP contract). Slot id is part of the key so distinct semantic
slots cannot share an action identity.

`sc_threshold` is **0** and `keyint` is **fixed** so GOP layout is
deterministic across machines.

### kerf

```
kerf := H(
  left.slot_encode | right.slot_encode | transition | dest | encoder
)
```

`transition` is `cut` or a named fade. Kerf output is the re-encoded
GOPs that straddle the join (typically one GOP each side for Long-GOP).
Intra / matching `stsd`: kerf is a no-op splice (empty artifact, still
a node).

### scion hash

```
scion_hash := H( ordered slot_encode[] | audio_encode[] | kerf[] | dest )
```

`audio_encode` is independent: slot id, material, source, AAC 48kHz stereo,
and encoder toolchain. Video-only scions omit it.

The time map is a **build artifact** keyed by `scion_hash` (dest clock).
It is not an identity derived only for the CLI.

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

1. Recompute each `slot_encode` key. Miss → encode that slot.
2. For each adjacent pair, recompute `kerf`. Miss if the key is new
   (either side’s `slot_encode` changed, or transition changed).
3. Concat: miss if `scion_hash` is new. Copy cached slot_encode / kerf
   blobs; encode only misses.

Retiming a slot (speed ≠ 1, or span length change) invalidates that
slot’s action key; the slot re-encodes in full.

## Grain

| Working format | Independent unit | Replace hook | Retime ("too slow") |
| --- | --- | --- | --- |
| Image seq / DPX / EXR / JPEG2000 / `graft-intra` | frame | those frames | new essence for the slot |
| ProRes / DNxHR / All-I | frame | sample-accurate splice if `stsd` matches | re-encode that slot |
| H.264/HEVC/AV1 Long-GOP | GOP / IDR (~0.5–2s) | hook GOPs + kerf | full slot re-encode |
| HLS / CMAF | segment aligned to GOP | replace hook segments | new segments + playlist |

Intra is proven: `SlotEncode` copies `[in, out)` frames into CAS;
kerf is empty; concat splices.

Long-GOP (`impl: x264`): graft shells out to **system** ffmpeg/`libx264`
with `keyint` = `min-keyint`, `scenecut=0`, `threads=1`. Each slot
encode is a closed-GOP mp4 starting on IDR. Concat is `ffmpeg -c copy`
after probing parts for size, fps, codec, pix_fmt, color, and time base.
The hook→body kerf node still misses when the hook key changes; its
artifact is empty at an IDR-aligned join. Body `BlobId` is unchanged
across a hook swap. Mid-GOP splice (re-encode straddling GOPs from one
long timeline encode) is the same node, later. Synchronized AAC audio
is cached independently and muxed after the video link.

Do not statically link x264 into the Apache-2.0 binary.

## Worked compile

Previous ship: `hook_v2 + body_v1 + cta_v1 @ 9x16`.
Now: `hook_v3` on the same body and cta.

| Piece | Cache |
| --- | --- |
| hook encode | MISS (new material hash → new ActionKey) |
| body encode | HIT (same ActionKey; body BlobId unchanged) |
| cta encode | HIT |
| kerf hook→body | MISS (left changed) |
| kerf body→cta | HIT |

Concat: `hook_encode + kerf_hook_body + cached_body + cached_kerf_body_cta + cached_cta`.
Body’s frames are bitstream-copied. The concat blob is new. The body
blob is not.

## What not to build

- xdelta / rsync / FastCDC on the delivery mp4
- MP4 `elst` as the composition model (poor support; hidden keyframe lead-in)
- A second, cloud-only IR
- Diffing two `scion.json` files as the cache
