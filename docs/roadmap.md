# Roadmap

There is **no product version** until a compiler GitHub tag `v0.2.0`.
Do not reuse spec tag `v0.1.0`. The `graft` field on JSON is a **format
id** (`0.1.0` today), not a crate version.

## North star

- Mission, principles, ADRs
- JSON Schema for score, scion, time map
- Worked example and signal → dirty-set reference

## Score CLI

`graft init | slot | bind | scion | dirty | signal` on JSON.
`graft compile` schedules from the action cache. With essence in CAS:
`graft-intra` copies frames; `x264` shells out to ffmpeg and writes mp4.

## CAS

Namespaced blake3 blobs + action cache. Relink by hash. Refuse path-only
bindings. `ActionKey` ≠ `BlobId`.

## Intra compile

ProRes / image-seq / All-I / `graft-intra`: sample-accurate concat.
Hook swap does not rewrite body bytes (proven on frame grain).

## Long-GOP kerf

x264 via **system ffmpeg** (`keyint` fixed, `sc_threshold=0`). Closed-GOP
slot files; concat bitstream-copies hits. Hook swap keeps the body blob.
Mid-GOP splice on a shared timeline encode is the same kerf node, later.

## OTIO export

Lossy, documented. Slots as markers. Round-trip test: cuts + media refs,
not effects.

## Signal ingest

CSV/JSON of `(kind, t0, t1, dest)` → dirty set via time map.
Keep the north-star test.

## Schema freeze (first real release)

No breaking change without a major version. Encoder fingerprint and
cache keys stable. Device-first compile for intra + Long-GOP cuts
(no arbitrary effects graph).

## Explicitly later / never in core

- Premiere/Avid/Resolve/CapCut as first-class lossless
- Review UI, DAM, storage mount, project lock
- Cloud-only IR
- Auto-publish to ad platforms
