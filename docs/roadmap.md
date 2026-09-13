# Roadmap

Pre-1.0 the **schema** is the version. The CLI is unreleased until 0.2
does something a human can run besides `make test`.

## 0.1 — north star (this tag)

- Mission, principles, ADRs
- JSON Schema for score, scion, time map
- Worked example and signal → dirty-set reference

## 0.2 — score CLI, no pixels

`graft init | slot | bind | scion | dirty | signal` on JSON only.
Print the dirty set for the worked example. Still no ffmpeg.

## 0.3 — CAS

blake3 put/get. Relink by hash. Refuse path-only bindings.

## 0.4 — intra compile

ProRes / image-seq / All-I: sample-accurate concat. Prove hook swap
does not rewrite body bytes.

## 0.5 — Long-GOP kerf

x264 (or ffmpeg libx264) with fixed `keyint`, `sc_threshold=0`.
Re-encode dirty GOPs + kerf. Bitstream-copy hits.

## 0.6 — OTIO export

Lossy, documented. Slots as markers. Round-trip test: cuts + media refs,
not effects.

## 0.7 — signal ingest

CSV/JSON of `(kind, t0, t1, dest)` → dirty set via time map.
Keep the north-star test.

## 1.0 — schema freeze

No breaking change without a major version. Encoder fingerprint and
cache keys stable. Device-first compile for intra + Long-GOP cuts
(no arbitrary effects graph).

## Explicitly later / never in core

- Premiere/Avid/Resolve/CapCut as first-class lossless
- Review UI, DAM, storage mount, project lock
- Cloud-only IR
- Auto-publish to ad platforms
