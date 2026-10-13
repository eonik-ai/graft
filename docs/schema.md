# Schema commentary

**Normative** documents are the JSON Schema files in [`/schema`](../schema/).
This page is commentary. If they disagree, the schema wins; then file a PR
to fix this page.

Format id: **0.2.0**. The `graft` field on every document must match.
This is not a product version. `0.1.0` documents must be migrated; they are
not silently rehashed.

## Documents

| File | Job |
| --- | --- |
| `score.json` | One concept: clock, slots, declared windows, layer order |
| `scions/<id>.json` | Named variant: optional parent, dest, ordered layer opinions |
| `.graft/builds/<id>/build.json` | Immutable compile identity |
| `.graft/builds/<id>/time-map.json` | Exact shipped dest ranges → slots |
| `feedback/<id>.json` | Raw platform signal plus resolved slots/kerfs |

Materials live in CAS, referenced as `blake3:<64 hex>`.

## Time

Clock, dest, and source ranges use rational rate plus integer frames:

```json
{ "rate": { "num": 30, "den": 1 }, "range": { "start": 0, "duration": 90 } }
```

`0.1.0` `span` / `in_s` / `out_s` / `fps` values migrate through
`graft migrate`. Canonical hashes of `0.2.0` documents are not comparable
to `0.1.0` hashes.

## Slot vs dest

```json
{
  "id": "hook",
  "role": "hook",
  "range": { "start": 0, "duration": 90 },
  "window": { "kind": "hook_rate", "range": { "start": 0, "duration": 90 } }
}
```

`range` is where the slot lives on the **score clock**. `window` is what a
metric means. `9x16` is not a slot. It is `scion.dest`.

## Layers and inheritance

`score.layers` is the strength order: `base`, `copy`, `grade`, `legal`.
Each scion layer is an opinion map of slot bindings. Later names win.
A child scion names `parent` and records only its overrides. Parentage is
variant derivation, not Git history. `scion.graph` inherits the same
way: a child with `graph` omitted keeps the parent's nodes. A child
that sets `graph` replaces the whole graph (it does not patch one node
in place).

## Binding

```json
"hook": {
  "material": "blake3:…",
  "source": {
    "rate": { "num": 30, "den": 1 },
    "range": { "start": 12, "duration": 90 }
  },
  "params": { "speed": 1.0 },
  "audio": { "material": "blake3:…", "source": { "rate": { "num": 30, "den": 1 }, "range": { "start": 12, "duration": 90 } } },
  "nested": "plate"
}
```

Optional `score.joins[]` names a transition between adjacent spine
slots (`left`, `right`, `transition`, `duration_frames`). Missing join is
`cut`. `transition` is `cut`, `fade` (ffmpeg `xfade=fade`), any `xfade`
name, or a host alias (`dipToBlack` → `fadeblack`, `wipeLeft` →
`wipeleft`, `dissolve` → `dissolve`). `whip*` / `flash` fail closed (guest).
`fade` is an **overlap**; dest length shortens by `duration_frames` per
overlap. The time map stores compiled dest ranges.

Optional `score.markers[]` is `{id, range}` on dest time. Markers do not
dirty picture. Porcelain `composition.json` `markers` writes them. The
delivery encode muxes them as chapters. OTIO import upserts sidecar
joins and markers onto the score.

`dest_default` is a dest id hint (`"9x16"`). `graft scion create` uses it
when `--dest-id` is omitted. It is not geometry; `--dest WxH` stays
required.

`encoder.level` is passed to ffmpeg as `-level:v`. `rate_control.mode =
bitrate` uses `-b:v`. The encoder fingerprint is on the **delivery**
key only. Changing level or bitrate misses the sink, not clean working
sources. Other `params` keys besides `speed` are rejected. Optional `binding.x` /
`binding.y` / `binding.opacity` are overlay mix geometry. They are not
in the working-source key. `overlay_mix` fingerprints them, including
`binding.motion` (`{x,y,opacity}` arrays of `{t,v,ease?}`; linear stays the
historical overlay expression, other eases shape that window). `graft swap <overlay> --x --y` updates that place without a
new file and clears motion on the axes it sets. Optional `binding.remap`
is stepwise speed (`v` holds until the next key; any ease other than
`hold` or `linear` is `bad_remap`). It is on the
working-source key. Encode folds source trims and concats working pads.
Dest duration stays the score slot. Intra fails closed on a curve.

Optional `binding.nested` is a scion id. Compile resolves it to the
inner working `#picture` (never the delivery mp4) and uses that blob as
the slot material. Graph op `nested` (`params.scion`) is the same pad
for overlays / mattes / stacks. Porcelain `composition.json` `nests`
writes `nests/<id>/score.json` plus `scions/<id>.json`. A parent slot
`{ "id": "hook", "scion": "plate" }` stores `nested: "plate"`. A cycle
fails closed. Intra fails closed on a nested pad. `graft address` on a
dirty nested slot names inner covering nodes as `plate.look` and next
is `graft swap look <file> --scion plate`. A covering `switch` node wins
that `next` (`graft swap switch <file> --scion angles`). Porcelain nest
`switch.cuts` authors the node: the first cut is the spine slot, later
cuts are `source` pads.

Optional `binding.audio.place` is a dest-clock range. When it is set,
or when the audio source duration is not the picture dest duration,
compile dest-places that pad (J/L) instead of concat-locking it to the
picture cut. Porcelain `composition.json` `slots[].audio` (`in` / `out`
/ `place`) writes this. `graft swap` of a picture range keeps the
parent J/L offset.

Optional `dest.working` is `{pix_fmt, color, primaries?, trc?}`.
Default `yuv422p10le` + dest color, range `tv` (ProRes HQ).
`yuv444p10le` is ProRes 4444. `primaries` / `trc` enter the working
fingerprint only when set.

Optional `scion.graph` is `{mode, nodes[]}`. `mode` is `append`
(default) or `replace`. A node is `{id, op, inputs?, material?,
source?, place?, enable?, role?, params?, keys?, nodes?}`. `op` is a
structural name or a filter-shaped name (`^[a-z][a-z0-9_]{0,63}$`).
Compile checks this ffmpeg: a missing filter is `toolchain`, an unknown
param is `unknown_param`. `keys` is `{param: [{t, v, ease?}]}` with
`ease` of `hold`, `linear`, `ease_in`, `ease_out`, `ease_in_out`.
`nodes` is an in-graph `group` (an adjustment). Compile expands the
children and inherits the group's `enable`. Dedicated structural ops
stay: `chromakey`,
`colorkey`, `lumakey`, `chromahold`, `alphamerge`, `drawbox`,
`drawgrid`, `colorchannelmixer`, `setsar`, `setdar`, `perspective`,
`shear`, `scroll`, `premultiply`, `unpremultiply`, `streamselect`,
`maskedmerge`, `hstack`, `vstack`, `xstack`, `mix`, `lenscorrection`,
`yadif`, `bwdif`, `tile`, `haldclut`, `colorcorrect`, `cdl`, `lgg`,
`geq`, `selectivecolor`, `exposure`, `colorhold`, `colorize`, `deband`,
`switch` (`params.cuts` of `{t, map}`),
`nested` (`params.scion`), `group`, plus mixboard audio (`acompressor`, `alimiter`, `equalizer`,
`highpass`, `lowpass`, `bass`, `treble`, `adelay`, `aecho`, `areverse`,
`pan`). Any other filter this binary has lowers the same way. `colorspace` takes `all` / `primaries` / `trc`. Blend
`params.mode` accepts host aliases (`hardLight`). Overlay
`params.opacity` is alpha on a graph overlay node; role overlay uses
`binding.x` / `y` / `opacity`. Role `overlay` is a dest-placed picture pad
(with `brand`). Role `sfx` is an overlay audio pad on dest time. The reference host
authors this from `takes/composition.json` (`file` is porcelain; the
stored node has a `blake3:` material). Porcelain `adjustments[]`
stores one `group` per layer (`enable` on the group, children nested).
`keys` and `ease` on a node are stored, not flattened. Porcelain `slots[]` bind
`file` + `in` / `out` on a library clip; that is not a schema object —
the stored form is a score slot plus a scion binding with `source`.
A child that omits `graph` inherits the parent nodes.

Source duration at `params.speed` must match the slot's dest duration
(one-frame tolerance). Speed ≠ 1 invalidates that slot's action key and
is applied at encode (`graft-intra` frame resample; ffmpeg `setpts` /
`atempo`). Adjacent clean slots stay cache hits. Iterate names the slot;
it does not invent a speed.

`encoder.profile` is optional (x264 fingerprints still send `"high"`;
`graft-intra` uses `"intra"`). Toolchain identity is part of the encoder
fingerprint.

## Build and feedback

A build record names `scion`, `scion_hash`, `dest_id`, and the time-map
path used to resolve later feedback. Feedback stores the raw metric and,
after ingest, the resolved slots and kerfs. `graft iterate` forks a new
scion with a change request; it does not invent the replacement clip.

## Worked scion

See [`examples/hook-v3-body-v1-9x16/`](../examples/hook-v3-body-v1-9x16/).
That fixture is part of the contract: `make test` requires that
`hook_rate` on the declared `[0, 90)` window dirties hook + hook→body
kerf only.

See [`examples/dub-en-9x16/`](../examples/dub-en-9x16/) for the same dest
axis with overlapping `vo`/`captions`, inherited picture scions, and the
`vo_hold` / 3–5s `note` / `hold` dirty sets. Material ids there are
BLAKE3 of local Veo and MyWonder files; essence is not in git.
