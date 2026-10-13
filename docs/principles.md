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

A shipped file is the **delivery sink** of the composition graph
(filters, then encode). Never hash it as essence. Never treat
xdelta/rsync on a re-export as the delta system. Today's path may still
be slot encodes + kerfs + concat; that linker shape is not the
definition of a compile. See [composition.md](composition.md) and ADR
0010.

## 5. Three layers, never mixed

| Layer | Unit | Tool |
| --- | --- | --- |
| A. Recipe | score / scion JSON | git |
| B. Essence | material hash | CAS |
| C. Build | working node output (ProRes HQ / PCM), then one delivery encode | action cache |

A "version" that conflates these three is a bug.

## 6. Slots are addressing, not the graph

Roles: `hook` `body` `proof` `cta` `vo` `captions` `bed` `brand` `sfx`.
They label dest-time spans so signals and hosts can point at “the hook.”
They are not the composition. A video is pads + filters + PTS. Ratio is
not a slot. **9:16 is a dest.** Same graph, different dest, different
delivery encode. Inventory: [composition.md](composition.md).

## 7. Windows are declared

The Meta/TikTok 3s hook window is a field on the score, not inferred from
however long the hook take happened to be. Signal resolution must use the
declared window and the exact shipped-build time map. An explicit caller
range is only a fallback when no window is declared.

## 8. Grain is physics — and working ≠ delivery

- Working / action-cache grain is the **node output** (usually frame or
  a short decoded span) in a working depth. That is what incremental
  compile reuses.
- Delivery Long-GOP H.264/HEVC/AV1 grain is GOP/IDR. Bitstream-copy is
  legal only when no filter sits downstream and pads already match.
- Intra / image-seq / JPEG2000: grain = frame.
- Retiming invalidates GOP-copy of that span. A post-join LUT invalidates
  delivery-copy of every upstream pad, even if the body subgraph hits.

## 9. Build identity is provenance, not just a filename

A shipped build must identify the flattened scion, destination, compiler and
encoder actions, output, and exact time map used to resolve feedback. A root
`time-map.json` or an output path is not sufficient provenance.

## 10. Encoder fingerprint is part of the key

Working-source and working-node keys include dest size, rate, and
working depth — not the delivery encoder. The delivery sink key includes
impl, version, profile, crf/bitrate, preset, keyint, `sc_threshold=0`.
Filter availability (`lut3d`, `drawtext`, …) is part of the toolchain
fingerprint of the node that uses it. Deterministic GOP layout is a
feature of the delivery encode, not of the working graph.

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
Dirty set = pads whose compiled time-map span and declared signal window
address that range. A whole-dest look intersects every metric. No build
provenance and time map, no surgical edit.

## 14. Do not recut the tree

The metaphor is a graft. New scion wood on the same rootstock. A feature
that re-runs a **clean subgraph** to change an unrelated pad is
incorrect.

“Keep the body” means the body’s working ActionKey hits. It does **not**
mean bitstream-copy of an 8-bit H.264 cell when a look, overlap
transition, or spanning mix sits downstream (ADR 0010).

## 15. Public pages are the product

graft is a public Apache-2.0 project. README, install, packaging, changelog,
and GitHub release notes describe the software as it is. They are not a GTM
tracker, a founder checklist, or a living submissions board. English is the
only README.

Do not name pkg.bot, Repology coverage counts, lead lists, or "waiting
on you" on those pages. Do not import eonik GTM copy. Recipes under
`dist/` are packager source. A recipe is not a listing. Do not claim
`dnf install graft` because a spec file exists.

Working notes that are not user documentation stay unlinked from README
and [docs/README.md](README.md).

## 16. The composition inventory is standing

A video is LUT, overlap transitions, codecs, SFX, titles, masks, mix,
color tags, working depth, and more — all timestamped. The exhaustive
list lives in [composition.md](composition.md) and is **append-only**.
ffmpeg is the lowerer. Do not invent a compositor. Do not treat named
takes as authorship. Agents read that file without being asked (principle
16 / `.cursor/rules/17-composition.mdc`).

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
