# Principles

These are non-negotiable until an ADR supersedes them. If a PR fights a
principle, the PR loses.

## 1. Git owns history; the score is source

Recipe documents live in Git. Git owns commits, branches, textual merge, and
transport; graft does not implement another commit database. The score and
scions are composition source, not rendered files.

## 2. graft owns composition semantics

Scions and inheritance, ordered layer opinions, semantic diff and conflicts,
build provenance, signal resolution, and compile lowering belong to graft.
Scion parentage expresses variant derivation; Git history expresses change over
time. Do not conflate them.

## 3. Essence is immutable

Materials are CAS blobs. New bytes, new hash. Relink by hash, never by
path. Do not rewrite a camera original to "update" a slot.

## 4. The mp4 is a compile

A shipped file is a linker product: slot encodes + kerfs + concat. Never
hash it as essence. Never treat xdelta/rsync on a re-export as the delta
system.

## 5. Three layers, never mixed

| Layer | Unit | Tool |
| --- | --- | --- |
| A. Recipe | score / scion JSON | git |
| B. Essence | material hash | CAS |
| C. Build | `slot_encode` + `kerf` | compile cache |

A "version" that conflates these three is a bug.

## 6. Slots are the addressable unit

Roles: `hook` `body` `proof` `cta` `vo` `captions` `bed` `brand`.
Ratio is not a slot. **9:16 is a dest.** Same bindings, different dest,
different scion.

## 7. Windows are declared

The Meta/TikTok 3s hook window is a field on the score, not inferred from
however long the hook take happened to be. Signal resolution must use the
declared window and the exact shipped-build time map. An explicit caller
range is only a fallback when no window is declared.

## 8. Grain is physics

- Intra / image-seq / JPEG2000: grain = frame. Kerf may be empty.
- Long-GOP H.264/HEVC/AV1: grain = GOP/IDR. Re-encode the dirty slot
  plus typically one GOP each side of a join (the kerf).
- Retiming ("too slow") invalidates GOP-copy for that slot.

## 9. Build identity is provenance, not just a filename

A shipped build must identify the flattened scion, destination, compiler and
encoder actions, output, and exact time map used to resolve feedback. A root
`time-map.json` or an output path is not sufficient provenance.

## 10. Encoder fingerprint is part of the key

`slot_encode` includes impl, version, profile, crf/bitrate, preset,
keyint, `sc_threshold=0`. Two machines concat only if this matches.
Deterministic GOP layout is a feature.

## 11. Adapters are guests

Universal means compile-out / lossy import, like USD vs Maya. It does
not mean lossless AAF to every NLE. CapCut has no public edit API;
draft JSON is reverse-engineered. Document loss. Do not lie.

## 12. Device-first

The laptop holds the graph and a local CAS. Preview is decode +
composite. Export asks the compiler for the dirty set. A server is the
same compiler with a bigger object store, not a different product.

## 13. Signals dirty nodes through the time map

A platform metric references the exact shipped build and its time range.
Dirty set = slots whose compiled time-map span and declared signal window
address that range. No build provenance and time map, no surgical edit.

## 14. Do not recut the tree

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
- Hiding or replacing Git history with a graft object database
- Guessing creative replacements from platform feedback
