# Adapters

Adapters are guests. graft compiles **out**. Import is best-effort.
This is how OpenUSD treats Maya. It is not AAF-after-25-years.

A new adapter PR must fill a row here. A claim of "lossless" needs an RFC
and a fixture. The default is loss.

## Loss matrix (0.1)

| Guest | Export from graft | Import to graft | Loses |
| --- | --- | --- | --- |
| OTIO | slots as markers + metadata / SchemaDef; clips + media refs | cuts, media refs, markers | most effects (OTIO matrix: effects largely unsupported) |
| FCPXML | timeline + markers | cuts, some effects | anything FCPXML cannot round-trip |
| MLT / GES | playlist / tractor | cuts + filters GES knows | graft slots flatten |
| IMF CPL + MXF | finish backend: one video track per composition | not an authoring import | in-progress / incomplete packages (IMF UG non-goal) |
| ffmpeg graph | concat / filter_complex | no | not a project file |
| CapCut `draft_info.json` | best-effort unofficial | best-effort unofficial | version-pinned, reverse-engineered, no public API |
| Premiere / Avid / Resolve | via OTIO or FCPXML | via OTIO or FCPXML | effects, color, plugins |
| Remotion / code | props JSON from scion | no | not an NLE |

## Rules

1. Slots and variants live in graft (or OTIO SchemaDef). On export they
   **may flatten to markers**. That is correct, not a bug.
2. Do not block a core release on an adapter.
3. CapCut collab is lock-and-key in the product. graft will not pretend
   to be CapCut cloud.
4. GES/MLT may be an optional **runtime** so graft is not itself an NLE.
   The IR still lives here.

## Adding a row

Open an RFC with: guest name, official schema/API URL, a loss list, and
whether import is supported. Implementation follows the RFC, not the reverse.
