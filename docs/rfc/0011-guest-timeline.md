# RFC 0011 — one guest timeline

- Status: accepted, implemented
- Date: 2026-10-14
- Implements: principle 11, [adapters.md](../adapters.md)
- Format id: `0.2.0` (no schema change)

## Problem

"Plugs into every creative app" was a row per app, and the one row that
existed (OTIO) pointed at `graft://blake3:…`. No editor can open that.
Premiere does not reliably import FCPXML. CapCut has no public edit API.
Photoshop and After Effects are not timelines. A plugin per application
is the shortcut this project already rejected.

## Decision

1. Compile still lowers through ffmpeg. Guests are not a second compiler.
2. Export flattens the selected scion into one timeline: slot id, role,
   source in/out, dest range, material hash, and a file path when
   `.graft/media.json` has one. That index is machine-local. It is written
   when a file is ingested. It is not recipe history.
3. The same timeline lowers to four files:
   - OTIO (DaVinci Resolve)
   - FCPXML 1.9 (Final Cut Pro)
   - FCP7 XMEML (Premiere Pro). `in`/`out` and `start`/`end` are exclusive.
   - MLT XML (Shotcut and Kdenlive)
4. `graft export guest --out <dir>` writes all four plus `loss.json`.
   The loss report names the file each app opens. It states that After
   Effects has no public composition format, Photoshop authors stills,
   CapCut draft JSON is refused, and AAF is not a guest format. eonik is
   a host: it calls `graft compile` on the recipe.
5. Each file embeds a graft sidecar and writes `{filename}.graft.json`
   beside it. The sidecar holds the graph, joins, markers, overlay place,
   remap, and the material hash. Import of a graft export restores those
   onto a new scion. A cut edited in the guest (source in/out and dest
   placement) wins over the sidecar. Speed is refit so the binding still
   matches the slot. A directory whose files disagree is refused.
6. A foreign file with no sidecar imports cuts and hashes. FCPXML can
   supply frame size. Paths with no hash are hashed by the CLI when the
   file exists, then recorded in the media index. Missing files stay
   offline. CapCut `draft_info.json` / `draft_content.json` /
   `draft_materials` is an error, not a parser.
7. Overlapping clips (vo under bed, two overlays) are separate tracks.
   Holes in the spine are gaps. A remap curve is restored from the
   sidecar; the guest file plays one speed.
8. Working depth, native effects, grades, and plugins stay lost. No
   drawtext, OCIO, libplacebo, whip/flash, or HSL blend is marked as
   something these files preserve.

## Not this increment

- A C ABI, a Swift package, an HTTP compile server, or an eonik call into
  `graft_compile`. The recipe and the CLI are the host plug.
- An After Effects or Photoshop plugin. Stills bind as overlay or brand.
- AAF, GES, IMF, or a CapCut draft writer.
- Lossless round-trip of a grade made inside the guest app.
