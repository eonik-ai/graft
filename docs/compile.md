# Compile

The compiler turns a scion into a dest file. It is incremental.

## Cache keys

Hash function: **BLAKE3**. Canonical encoding: UTF-8 JSON, sorted keys,
no insignificant whitespace, numbers as decimal seconds with a fixed
scale (milliseconds — 3 decimal places — unless an RFC says otherwise).

### slot_encode

```
slot_encode := H(
  slot.role | material.hash | in_s | out_s | params | dest | encoder
)
```

`dest` = `{width, height, fps, pix_fmt, color}`.

`encoder` = `{impl, version, profile, level, rate_control, preset, keyint, sc_threshold}`.

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
Intra / matching `stsd`: kerf is a no-op splice.

### scion hash

```
scion_hash := H( ordered slot_encode[] | kerf[] | dest )
```

The shipped mp4 is **not** hashed as essence.

## Dirty set

Given previous scion hash P and new scion N:

1. Recompute each `slot_encode`. Miss → slot is dirty.
2. For each adjacent pair, recompute `kerf`. Miss if either side's
   `slot_encode` changed, or transition changed.
3. Concat: dirty slot encodes, dirty kerfs, copy hits.

Retiming a slot (speed ≠ 1, or span length change) invalidates that
slot's GOP-copy; the slot re-encodes in full.

## Grain

| Working format | Independent unit | Replace hook | Retime ("too slow") |
| --- | --- | --- | --- |
| Image seq / DPX / EXR / JPEG2000 | frame | those frames | new essence for the slot |
| ProRes / DNxHR / All-I | frame | sample-accurate splice if `stsd` matches | re-encode that slot |
| H.264/HEVC/AV1 Long-GOP | GOP / IDR (~0.5–2s) | hook GOPs + kerf | full slot re-encode |
| HLS / CMAF | segment aligned to GOP | replace hook segments | new segments + playlist |

## Worked compile

Previous ship: `hook_v2 + body_v1 + cta_v1 @ 9x16`.
Now: `hook_v3` on the same body and cta.

| Piece | Cache |
| --- | --- |
| hook encode | MISS (new material hash) |
| body encode | HIT |
| cta encode | HIT |
| kerf hook→body | MISS (left changed) |
| kerf body→cta | HIT |

Concat: `hook_encode + kerf_hook_body + cached_body + cached_kerf_body_cta + cached_cta`.
Body's 17s is bitstream-copied.

## What not to build

- xdelta / rsync / FastCDC on the delivery mp4
- MP4 `elst` as the composition model (poor support; hidden keyframe lead-in)
- A second, cloud-only IR
