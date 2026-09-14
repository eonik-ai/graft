# Adapters

Adapters are guests. graft compiles **out**. Import is best-effort.
This is how OpenUSD treats Maya. It is not AAF-after-25-years.

A new adapter PR must fill a row here. A claim of "lossless" needs an RFC
and a fixture. The default is loss.

`graft-otio` depends on `graft-score` only. It exports a flattened scion
and imports a supported subset as a **new** scion, never over the source.
Every export and import writes a machine-readable loss report.

## Loss matrix

| Guest | Export from graft | Import to graft | Loses |
| --- | --- | --- | --- |
| OTIO (implemented subset) | clips, rational ranges, `graft://` media refs, slot markers, dest/scion provenance, spine audio metadata | cuts, graft media refs, markers, timing, bound spine audio | effects, grades, generators, transitions, non-graft metadata, inheritance/layers flatten |
| FCPXML | not implemented | not implemented | anything FCPXML cannot round-trip |
| MLT / GES | not implemented | not implemented | graft slots flatten |
| IMF CPL + MXF | finish backend later | not an authoring import | in-progress / incomplete packages (IMF UG non-goal) |
| ffmpeg graph | concat / filter_complex is compile, not interchange | no | not a project file |
| CapCut `draft_info.json` | not implemented | not implemented | version-pinned, reverse-engineered, no public API |
| Premiere / Avid / Resolve | via OTIO only today | via OTIO only today | effects, color, plugins |
| Remotion / code | not implemented | no | not an NLE |

## Rules

1. Slots and variants live in graft. On export they **may flatten to
   markers**. That is correct, not a bug.
2. Do not block a core release on an adapter beyond the scoped OTIO path.
3. CapCut collab is lock-and-key in the product. graft will not pretend
   to be CapCut cloud.
4. GES/MLT may be an optional **runtime** so graft is not itself an NLE.
   The IR still lives here.

## Adding a row

Open an RFC with: guest name, official schema/API URL, a loss list, and
whether import is supported. Implementation follows the RFC, not the reverse.
