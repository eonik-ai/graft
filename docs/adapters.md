# Adapters

Adapters are guests. graft compiles **out**. Import is best-effort.
This is how OpenUSD treats Maya. It is not AAF-after-25-years.

A new adapter PR must fill a row here. A claim of "lossless" needs an RFC
and a fixture. The default is loss.

`graft-otio` depends on `graft-score` only. One flattened timeline lowers
to OTIO, FCPXML 1.9, FCP7 XMEML, and MLT. Import writes a **new** scion,
never over the source. Every export and import writes a loss report.
`graft export guest` writes all four files plus that report.
`.graft/media.json` maps a hash to the ingested path so the guest file
can open the media. The index is local. It is not recipe history.

## Loss matrix

| Guest | Export from graft | Import to graft | Loses |
| --- | --- | --- | --- |
| OTIO (DaVinci Resolve) | clips on picture / overlay / audio tracks, gaps for holes, separate tracks when clips overlap, `file://` when the media index has a path, otherwise `graft://`, dest/scion provenance, `metadata.graft` sidecar (`graph`, `joins`, `markers`, `nested`, overlay place, remap) | cuts, graft media refs, overlay/spine bindings, `scion.graph`, score joins/markers from sidecar, `binding.nested`, overlay place, timing. A guest trim updates the slot range | native OTIO effects/grades, working-depth execution, generators, non-graft metadata, inheritance/layers flatten. A remap curve plays as one speed |
| FCPXML 1.9 (Final Cut Pro) | spine, connected overlays and audio, `timeMap` when speed is not 1, embedded sidecar plus `{file}.graft.json` | same sidecar restore. Structural in/out wins | native effects, grades, plugins, working-depth execution |
| FCP7 XMEML (Premiere Pro) | sequence, video tracks, audio tracks. `in`/`out` and `start`/`end` are exclusive frame numbers. Sidecar in `<comment>` and beside the file | same sidecar restore. Structural in/out wins | native effects, grades, plugins, working-depth execution. Premiere may drop the embedded comment; the sibling `.graft.json` remains |
| MLT (Shotcut, Kdenlive) | profile, producers, one playlist per non-overlapping track, tractor | same sidecar restore | native effects, grades, plugins. A remap curve plays as one speed |
| GES | not a guest file | no | GES is a runtime, not this interchange |
| IMF CPL + MXF | finish backend later | not an authoring import | in-progress / incomplete packages (IMF UG non-goal) |
| ffmpeg graph | concat / filter_complex is compile, not interchange | no | not a project file |
| CapCut `draft_info.json` | refused | refused | no public edit API. Draft JSON is not parsed |
| After Effects | no file | no | no public composition format. Rasterize a graphic to an overlay still |
| Photoshop | no file | no | not a timeline. Overlay and brand stills are the plates it already authors |
| Avid | open `resolve.otio` where the app imports OTIO | that OTIO file | AAF is not a guest format |
| eonik | the recipe (`score.json`, scions) | not a timeline import | eonik is a host. It calls `graft compile`. It does not import these files |
| Remotion / code | not implemented | no | not an NLE |

## Rules

1. Slots and variants live in graft. On export they **may flatten to
   markers**. That is correct, not a bug.
2. Do not block a core release on a guest beyond OTIO, FCPXML, FCP7 XML,
   and MLT. Those four are views of one timeline (RFC 0011).
3. CapCut collab is lock-and-key in the product. graft will not pretend
   to be CapCut cloud. Import of draft JSON is an error.
4. GES/MLT may be an optional **runtime** so graft is not itself an NLE.
   The IR still lives here.

## Adding a row

Open an RFC with: guest name, official schema/API URL, a loss list, and
whether import is supported. Implementation follows the RFC, not the reverse.
