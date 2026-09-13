# Comparison

graft exists because nearby tools each miss a piece. None of them are
enemies; several are guests.

| System | Gives you | Does not |
| --- | --- | --- |
| **Git** | commits, branches, merge, and transport for recipe documents | scion semantics, media storage, build provenance, signal resolution |
| **OTIO** | clips, tracks, media refs, SchemaDef | named slots, variant sets, compile cache, signal map |
| **IMF ST 2067** | CPL + Track Files; Netflix supplemental IMP; Photon validator | authoring; incomplete in-production packages (UG non-goal) |
| **CapCut draft JSON** | tracks + materials + ids, source vs target timerange, canvas ratio | public API; stable schema; slot semantics |
| **GES / MLT** | a real compose+render runtime | versioning, variants, incremental encode cache |
| **ffmpeg concat, smartcut, LosslessCut** | GOP-aware splice | awareness of which recipe node dirtied |
| **OpenUSD** | layers, variants, strength (algebra to copy) | video essence |
| **Dits / Xet / FastCDC** | keyframe-aware CAS of *files* | slots; a new hook encode is still a new file |
| **Vit** (Resolve JSON + Git) | project-file versioning in Git | incremental compile, slots, dest as a first-class axis |
| **Aspect / Shade** | mount + review + file versioning | merge of edits; composition IR |
| **Postlab / Avid bins / Resolve Cloud** | project lock | variants as data |
| **Frame.io / review players** | comments on a time range | a compiler that dirties a slot |
| **Marpipe / Bannerflow** | combinatorial ad variants | a general IR or open compiler |

Framekit's 2026 survey of 13 tools: **none** do storage + lock + catalogue
+ review. graft does not grow into those four jobs. Its job is the local
composition-and-iteration loop; incremental compile is the build subsystem
that makes surgical variants economical. This boundary supersedes the earlier
compiler-only description (ADR 0007).

## What graft adds

Git remains the history layer. graft adds the video-aware workspace semantics
that a general text VCS should not own:

- named concepts and multiple derived scions;
- ordered layer opinions and deterministic flattening;
- semantic diff and structured composition conflicts;
- exact scion/build/time-map provenance;
- platform-signal resolution to slots and kerfs; and
- an action graph that reuses clean derived resources.

The current implementation provides those workspace semantics plus the
incremental compiler. NLE guests remain lossy; only the scoped OTIO subset
is implemented.

## Closest cousins

**IMF** is the existence proof for layer C on finished masters. graft
applies the same split (recipe / essence / supplemental rebuild) to
authoring, with *named slots* and a *time map*.

**OTIO** is the existence proof that interchange should be a graph, not
an NLE project. `graft-otio` exports and imports a supported subset with
an explicit loss report. It does not replace OTIO.

**USD** is the existence proof that variants and layers are a composition
algebra. Copy the ideas. Do not copy `.usda`.
