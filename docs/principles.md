# Principles

These are non-negotiable until an ADR supersedes them. If a PR fights a
principle, the PR loses.

## 1. The score is source

The recipe lives in git. It is JSON (this repo) and may compile out to
OTIO, FCPXML, IMF CPL, MLT. The score is not a rendered file.

## 2. Essence is immutable

Materials are CAS blobs. New bytes, new hash. Relink by hash, never by
path. Do not rewrite a camera original to "update" a slot.

## 3. The mp4 is a compile

A shipped file is a linker product: slot encodes + kerfs + concat. Never
hash it as essence. Never treat xdelta/rsync on a re-export as the delta
system.

## 4. Three layers, never mixed

| Layer | Unit | Tool |
| --- | --- | --- |
| A. Recipe | score / scion JSON | git |
| B. Essence | material hash | CAS |
| C. Build | `slot_encode` + `kerf` | compile cache |

A "version" that conflates these three is a bug.

## 5. Slots are the addressable unit

Roles: `hook` `body` `proof` `cta` `vo` `captions` `bed` `brand`.
Ratio is not a slot. **9:16 is a dest.** Same bindings, different dest,
different scion.

## 6. Windows are declared

The Meta/TikTok 3s hook window is a field on the score, not inferred
from however long the hook take happened to be. Spill into body below
`spill_threshold_s` (default 0.35) does not dirty body for `hook_rate`.

## 7. Grain is physics

- Intra / image-seq / JPEG2000: grain = frame. Kerf may be empty.
- Long-GOP H.264/HEVC/AV1: grain = GOP/IDR. Re-encode the dirty slot
  plus typically one GOP each side of a join (the kerf).
- Retiming ("too slow") invalidates GOP-copy for that slot.

## 8. Encoder fingerprint is part of the key

`slot_encode` includes impl, version, profile, crf/bitrate, preset,
keyint, `sc_threshold=0`. Two machines concat only if this matches.
Deterministic GOP layout is a feature.

## 9. Adapters are guests

Universal means compile-out / lossy import, like USD vs Maya. It does
not mean lossless AAF to every NLE. CapCut has no public edit API;
draft JSON is reverse-engineered. Document loss. Do not lie.

## 10. Device-first

The laptop holds the graph and a local CAS. Preview is decode +
composite. Export asks the compiler for the dirty set. A server is the
same compiler with a bigger object store, not a different product.

## 11. Signals dirty nodes through the time map

A platform metric is `(kind, [t0, t1), dest)`. Dirty set = slots whose
time-map span intersects that range, after the hook-window spill rule.
No time map, no surgical edit.

## 12. Do not recut the tree

The metaphor is a graft. New scion wood on the same rootstock. A feature
that requires re-encoding a clean body to change a hook is incorrect.

## Non-goals

- Git LFS as the composition model
- Byte-delta of delivery mp4 / "lossless git for video"
- Lock-and-key collab (Postlab, Avid bins, CapCut cloud lock)
- Frame.io-style review
- LucidLink-style storage
- Replacing a human editor
- In-production incomplete IMF (IMF UG non-goal; we may *emit* IMF at finish)
