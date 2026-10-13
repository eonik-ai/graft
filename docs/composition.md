# Composition

This is the standing inventory of what a video **is**.

Agents read this file before any schema, compile, adapter, README, or
host-embed work. Do not invent a compositor. ffmpeg is the execution
engine. Do not treat eight named H.264 files as a film. Do not stop at
"LUT, transitions, codecs, SFX" — those are four families among many.

The file is **append-only in the catalog**. Refine a row in place when
ffmpeg or a host proves the row wrong. Add a family when one is missing.
Record the addition in [Changelog](#changelog) at the bottom. Physics
changes that fight an accepted ADR get a **new ADR**, then a one-line
pointer here.

Related: [principles.md](principles.md) §16, ADR 0010,
[compile.md](compile.md), [schema.md](schema.md),
[ffmpeg-filters](https://ffmpeg.org/ffmpeg-filters.html).

Status on every row:

| Status | Meaning |
| --- | --- |
| **have** | Schema + compiler lower it to ffmpeg today |
| **partial** | Named in graft (role, layer, param, dest field) but not a real node |
| **absent** | Real composition dimension. Not in the IR. Not in the compiler |
| **guest** | A host or NLE authors it. graft must take it as pads or document loss |
| **toolchain** | Exists in some ffmpeg builds, not all. Fingerprint the binary |

This machine’s ffmpeg (2026-09-19, re-probed 2026-10-13): **481** filters. No `drawtext`
(no libfreetype). No `subtitles`/`ass` in the filter list. No
`libplacebo`, no OCIO filter. `T` on a filter means `enable=` timeline.
A filter-shaped graph op this binary has lowers generically (RFC 0010).
A missing filter fails closed as `toolchain`.

---

## Standing physics

These are not slogans. They constrain every IR change.

**C1. A video is a timestamped graph.** Nodes are filters. Edges are
labeled pads. Time is PTS on a timebase, plus `enable='between(t,t0,t1)'`,
xfade `offset=`, overlay `x`/`y`/`enable`, audio on a sample clock.
Roles (`hook`, `body`, …) are host words stamped on spans. They are not
the graph.

**C2. ffmpeg is the lowerer.** Phase 1: `filter_complex`. Phase 2: encode
the sink. The codec is the last node, not a clip. If a proposed node
cannot lower to libavfilter + an encoder, it is not a graft node.

**C3. Pads that meet must match.** Size, frame rate, pixel format,
timebase, and color tags must agree at every mix (`xfade`, `overlay`,
`blend`, `concat`, `amix`, `acrossfade`). A mismatch is a graph error,
not a demo bug. Veo `color_space=unknown` vs bt709 is this rule.

**C4. Working depth ≠ delivery depth.** Grade, LUT, key, and most blends
run at 10- or 16-bit (`yuv444p10le`, `gbrp16le`, `rgba64le`). Delivery
is usually 8-bit `yuv420p` **once**, at the sink. Baking delivery per
slot forbids a later look.

**C5. Three stores stay.** Git versions the graph. CAS versions essence
(`.mov`, `.wav`, `.cube`, font, caption). Action cache versions **node
outputs**. Never hash the delivery mp4 as essence (principle 4).

**C6. Incremental compile is dirty-subgraph.** An ActionKey is node type
+ params + input blob ids + working format. A hit means do not re-run
that subgraph. “Keep the body” means `src_body` hit. It does **not**
mean `-c copy` of an 8-bit GOP when a look, xfade, or spanning mix sits
downstream.

**C7. A global look after a join re-filters every upstream pad.**
Bitstream-copy of H.264 slots and a post-concat LUT cannot both be true.
Resolve already accepted this: Smart/User cache is working-codec frames;
delivery is a later render.

**C8. Audio is a parallel graph, same dest clock.** Picture GOP is not
the audio grain. A bed or SFX that spans a picture join is one stream
(or an `amix` of streams), not three slot AACs glued at the same cut.

**C9. Takes/ named files are not authorship.** Nobody exports
`hook.mov` / `body.mov` / `cta.mov` and glues another file as a “new
hook.” A host binds ranges on library clips. Porcelain that requires
pre-cut takes is a reference crutch, not the product.

**C10. Layers `base`/`copy`/`grade`/`legal` are binding opinions.** They
are not pixel grade, legalize, or LUT nodes until they lower to filters.
ADR 0008 already said this. Do not pretend a layer name is a look.

**C11. Toolchain is part of the key.** Filter availability, libx264
digest, threads, and pix_fmt support belong in the encoder/action
fingerprint (principle 10). A graph that needs `lut3d` fails closed if
the binary lacks it.

**C12. Guests are lossy.** Native OTIO effects, grades, and generators
stay lost. The graft sidecar carries `scion.graph`, joins, markers, and
overlay bindings. Import restores the graph **and** upserts joins /
markers onto the score. Whip / flash and HSL blends have no lowerer —
guest, fail closed.

---

## Object model

```
essence (CAS)          graph (git)              build (action cache)
─────────────          ───────────              ────────────────────
camera / Veo .mov      pad: trim + format       working node blobs
SFX .wav               pad: xfade / overlay     (intra / FFV1 / ProRes
LUT .cube              pad: lut3d / eq           / high-depth frames)
font, .srt             pad: amix / acrossfade
brand still            sink: libx264 + aac      delivery blob (once)
```

A **pad** is: source blob id (or upstream pad) + in/out on source clock
+ placement on dest clock + filter chain + working format.

A **dest** is still size, fps, color tags, and the **delivery** encoder.
It is not the working format.

A **slot / role** is an optional label on a dest-time span so signals
and hosts can address “the hook.” Multiple pads may live in one span
(picture + VO + SFX + captions).

---

## 1. Time and edit

The dest clock is rational frames (`rate.num/rate.den`). ffmpeg uses
PTS. Map explicitly.

| Dimension | What it is | ffmpeg | graft 0.2 |
| --- | --- | --- | --- |
| Source range | `[t0, t1)` on the file | `trim` / `atrim` + `setpts=PTS-STARTPTS` / `asetpts` | **have** — working source trims the binding range |
| Dest placement | Where that range sits on the film | `tpad`, `adelay`, `overlay`/`enable`, concat order | **have** — compiled dest ranges after overlap |
| Cut | A meets B with no overlap | `concat` filter (decoded) or bitstream concat if pads already match | **have** — working `concat` (not `-c copy`) |
| Overlap | A and B share dest time | required for `xfade` / `acrossfade` | **have** — `joins.transition` + `duration_frames` |
| J-cut / L-cut | Picture and sound cut at different times | separate video/audio graphs | **have** — `binding.audio.source` + optional `audio.place`; dest-placed mix when durations differ |
| Gap / slug / black | Empty dest time | `tpad`, `color`, `aevalsrc=0`, `apad` | **have** — graph `gap` (black or silence) |
| Nested sequence | A graph used as a source | cached working subgraph as a pad | **have** — `binding.nested` / graph `nested` (`params.scion`) compile the inner scion to its working `#picture` (never delivery). Cycle fail-closed. Porcelain `nests` + slot `"scion"` |
| Multi-cam | Several sources, one switched | `streamselect`, `overlay` | **have** — porcelain `switch.cuts` lowers to delayed `overlay` + `enable=gte(t,…)` (RFC 0009). Graph `streamselect` remains a static map |
| Markers / chapters | Named dest times | metadata / `-metadata` | **have** — `score.markers[]` + dest chapter mux |
| Declared signal window | Metric span ≠ take length | n/a (graft) | **have** — `slot.window` |

Do not add a “hook file” type. Add a range on a material.

---

## 2. Destination, geometry, raster

| Dimension | What it is | ffmpeg | graft 0.2 |
| --- | --- | --- | --- |
| Frame size | dest width × height | `scale`, `scale2ref`, `pad`, `crop` | **have** — `dest.width/height` + `scale` on slot encode |
| Pixel aspect (SAR) / display aspect (DAR) | anamorphic, not square pixels | `setsar`, `setdar` | **have** — graph `setsar` / `setdar` |
| Frame rate | dest Hz, VFR vs CFR | `fps`, `framerate`, `minterpolate` | **have** — `fps=` on slot encode (CFR) |
| Field / interlace | interlaced, telecine, 3:2 | `yadif`, `bwdif`, `w3fdif`, `telecine`, `detelecine`, `fieldmatch`, `decimate`, `idet`, `setfield` | **have** — graph ops (`yadif`, `bwdif`, `w3fdif`, `telecine`, `detelecine`, `fieldmatch`, `decimate`, `setfield`). `idet` stays a probe; it does not invent fields |
| Rotation / flip / transpose | phone metadata, 90°, mirror | `transpose`, `rotate`, `hflip`, `vflip`, display-matrix side data | **have** — graph `rotate` / `flip` (`params.axis`) |
| Crop / safe / letterbox / pillarbox | reframing for 9:16 vs 16:9 | `crop`, `pad`, `zoompan` | **have** — graph `crop` / `pad` / `zoompan` |
| Perspective / corner pin / lens | ads do this constantly | `perspective`, `lenscorrection`, `lensfun`, `v360` | **have** — graph `perspective` / `lenscorrection` / `v360` |
| Shear / scroll | lower-thirds, tickers | `shear`, `scroll` | **have** |
| 9:16 / 1:1 / 16:9 | dest, not a slot | dest id | **have** as dest; **wrong** if treated as a slot |

Principle 6 stands: **9:16 is a dest.** Same graph, different dest,
different delivery encode. Working nodes that are dest-size-dependent
(scale, xfade, overlay coords) dirty.

---

## 3. Color, grade, LUT, HDR

A look is a graph, not a layer name.

### 3.1 Colorimetry (tags vs convert)

| Dimension | Values / notes | ffmpeg | graft 0.2 |
| --- | --- | --- | --- |
| Color space (matrix) | `bt709`, `bt470bg`, `smpte170m`, `bt2020nc`, `gbr`, `fcc`, `smpte240m`, `ycgco` | `colorspace`, `colormatrix`, `setparams` | **have** — source nodes `setparams` to working color |
| Primaries | `bt709`, `bt2020`, `smpte431` (DCI-P3), `smpte432` (Display P3), `bt470m/bg`, `film`, `ebu3213` | `colorspace=primaries=` | **have** — graph `colorspace` `params.primaries` (+ optional `dest.working.primaries`) |
| Transfer (TRC) | `bt709`, `smpte170m`, `linear`, `log`, `iec61966-2-1` (sRGB), `bt2020-10/12`, `smpte2084` (PQ), `arib-std-b67` (HLG), `gamma22/28` | `colorspace=trc=`, `zscale` **toolchain** | **have** — graph `colorspace` `params.trc` (+ optional `dest.working.trc`). `zscale` **toolchain** |
| Range | `tv`/`mpeg` (limited 16–235) vs `pc`/`jpeg` (full) | `setrange`, `colorspace=range=`, `-color_range` | **have** — `setrange`; working source also forces `tv` |
| Chroma location | `left`, `center`, `topleft`, … | `-chroma_sample_location` | **absent** |
| Unknown tags | generated/Veo often `unknown` | must **normalize** before a mix | **have** — working source normalizes before join |

### 3.2 Working pixel formats (depth)

Run grade here. Not on delivery `yuv420p`.

| Format | Use |
| --- | --- |
| `yuv420p` | 8-bit 4:2:0 delivery. Too early for LUT |
| `yuv422p` / `yuv422p10le` | broadcast working |
| `yuv444p` / `yuv444p10le` | grade / key without chroma smear |
| `yuv420p10le` / `p010le` | 10-bit 4:2:0 (HDR delivery, not a grade space) |
| `gbrp` / `gbrp10le` / `gbrp16le` | planar RGB, honest for LUT / curves |
| `rgba` / `rgba64le` | straight or premultiplied alpha |
| `rgb48le` | 16-bit packed RGB |

ffmpeg: `format=`, `scale` (can convert), overlay `format=yuv444p10`.

graft: dest.pix_fmt is the **delivery** sink. Working default is
`yuv422p10le` ProRes HQ (`dest.working`; `yuv444p10le` is 4444). **have**.

### 3.3 Looks (the actual grade)

| Dimension | What it is | ffmpeg | graft 0.2 |
| --- | --- | --- | --- |
| 3D LUT | `.cube`, `.3dl`, `.dat`, `.m3d`, … tetrahedral default | `lut3d=file=:interp=nearest\|trilinear\|tetrahedral\|pyramid\|prism` + `enable=` | **have** — `scion.graph` `lut3d` |
| 1D LUT | per-channel shaper / legalize | `lut1d` (nearest/linear/cosine/cubic/spline) | **have** — `lut1d` node |
| Hald CLUT | identity + identity.png workflow | `haldclut`, `haldclutsrc` | **have** — graph `haldclut` (CLUT material as second input) |
| Per-channel LUT | `lut`, `lutrgb`, `lutyuv` | expressions | **absent** |
| CDL / ASC SOP+S | slope, offset, power, saturation | `colorbalance`, `eq`, `colorchannelmixer`, or bake to LUT | **have** — graph `cdl` lowers to `colorchannelmixer` + `eq` (not studio CDL math) |
| Curves | RGB / luma / per-channel | `curves` | **have** — `curves` (`params.preset`) |
| Lift / gamma / gain / offset | Resolve-style wheels | no single filter; `eq`, `colorbalance`, `colorcorrect`, expressions | **have** — graph `lgg` / `colorcorrect` (ffmpeg wheels, not Resolve math) |
| Brightness / contrast / sat / gamma | primary | `eq`, `hue`, `huesaturation`, `vibrance` | **have** — `eq` / `hue` / `vibrance` |
| Color wheels / selective | shadows/mid/high, HSL quals | `colorbalance`, `selectivecolor`, `colorize`, `colorhold`, `colortemperature` | **partial** — `colorbalance` / `colortemperature` / `colorcorrect` / `selectivecolor` / `colorize` / `colorhold` / `exposure`; HSL blend modes **absent** |
| OCIO / ACES | studio working space | **toolchain** — not in this ffmpeg | **absent** |
| libplacebo / tone + gamut | GPU path | **toolchain** | **absent** |
| HDR tone map | PQ/HLG → SDR | `tonemap` (none/linear/gamma/clip/reinhard/hable/mobius), `tonemapx`, `zscale` | **have** — `tonemap` (hable default) |
| HDR metadata | Mastering display, CLL, HDR10+ | side data, not a filter | **absent** |
| False color / scopes | authoring, not delivery | `waveform`, `vectorscope`, `histogram`, `cie` | **absent** (tools, not recipe) |

LUT files are **essence**. Hash the `.cube`. The node key includes that
BlobId + interp + `enable` range. A brand LUT after the join dirties
from that node to the sink (C7).

---

## 4. Transitions

A transition is an **overlap** of two pads, not two fades butted.

### 4.1 Video — ffmpeg `xfade` (this binary)

`duration`, `offset` (PTS on input 0), optional `expr` for `custom`.
Inputs must match size/fps/fmt/timebase.

| Name | Name | Name | Name |
| --- | --- | --- | --- |
| `fade` | `fadefast` | `fadeslow` | `fadeblack` |
| `fadewhite` | `fadegrays` | `dissolve` | `pixelize` |
| `wipeleft` `wiperight` `wipeup` `wipedown` | `wipetl` `wipetr` `wipebl` `wipebr` | `slideleft` `slideright` `slideup` `slidedown` | `smoothleft` `smoothright` `smoothup` `smoothdown` |
| `circlecrop` `rectcrop` | `circleopen` `circleclose` | `vertopen` `vertclose` `horzopen` `horzclose` | `radial` `distance` |
| `diagtl` `diagtr` `diagbl` `diagbr` | `hlslice` `hrslice` `vuslice` `vdslice` | `hlwind` `hrwind` `vuwind` `vdwind` | `hblur` |
| `squeezeh` `squeezev` | `zoomin` | `coverleft` `coverright` `coverup` `coverdown` | `revealleft` `revealright` `revealup` `revealdown` |
| `custom` | | | |

Also **not** xfade, still real:

| Kind | ffmpeg | Notes |
| --- | --- | --- |
| Sequential fade-out / fade-in | `fade=t=out` then `fade=t=in` | **have** as graph `fade` — this is **not** a crossfade |
| Dip to color | `fade=color=` or xfade `fadeblack`/`fadewhite` | **have** as xfade `fadeblack`/`fadewhite` |
| Audio-driven / whip / flash / motion blur | host CI or `minterpolate` + `xfade` + `tblend` | **guest** — eonik authors whip + flash |
| Audio crossfade | `acrossfade` | **have** — overlap joins + graph `acrossfade` |
| Transition on only one plane | split graphs | **absent** |

### 4.2 Audio fade curves (`afade` / `acrossfade`)

`nofade`, `tri`, `qsin`, `esin`, `hsin`, `log`, `ipar`, `qua`, `cub`,
`squ`, `cbr`, `par`, `exp`, `iqsin`, `ihsin`, `dese`, `desi`, `losi`,
`sinc`, `isinc`, `quat`, `quatr`, `qsin2`, `hsin2`.

graft: **have** — graph `afade` / `acrossfade` `params.curve` (default `tri`).

### 4.3 What the first host already authors (eonik)

`TransitionKind`: `cut`, `dissolve`, `dipToBlack`, `wipeLeft/Right/Up/Down`,
`slideLeft/Right/Up/Down`, `zoomIn`, `zoomOut`, `blur`,
`whipLeft/Right/Up/Down`, `flash`. Overlap is `Clip.dissolveIn`.

graft `joins.transition` stores the ffmpeg `xfade` name. Host aliases
(`dissolve`, `dipToBlack`, `wipeLeft`, …) canonicalize on write. Whip
and flash have no lowerer. Status: **guest** on the host for those two;
**have** for the xfade set.

---

## 5. Compositing, masks, keys, blend

| Dimension | What it is | ffmpeg | graft 0.2 |
| --- | --- | --- | --- |
| Overlay / PIP | A on B at (x,y), timed | `overlay=x:y:enable=:format=:alpha=` | **have** — role `brand` / `overlay` at `binding.x` / `y` / `opacity` + graph `overlay` (`enable` optional) |
| Blend modes | Photoshop-style | `blend=all_mode=` (list below) | **have** — graph `blend` (`params.mode`) |
| Opacity | 0–1, ramped | overlay alpha, `geq`, `fade=alpha=1`, `colorchannelmixer` | **have** — overlay `params.opacity`, `binding.motion.opacity` (`geq` when keyed), graph `colorchannelmixer` |
| Premultiply | straight vs premul | overlay `alpha=straight\|premultiplied`, `premultiply`, `unpremultiply` | **have** — graph `premultiply` / `unpremultiply` |
| Mask / track matte | luma, alpha, shape | `alphamerge`, `maskedmerge`, `threshold`, second pad | **have** — `alphamerge` + three-pad `maskedmerge` |
| Chroma / luma key | green screen, luma | `chromakey`, `colorkey`, `chromahold`, `lumakey` | **have** |
| Garbage / holdout | extra mattes | same + `crop`/`geq` | **have** — graph `geq`; `maskedmerge` third input is the holdout |
| Stack / mosaic | side-by-side, xstack | `hstack`, `vstack`, `xstack`, `tile` | **have** — `hstack` / `vstack` / `xstack` / `tile` |
| Mix N videos | weighted | `mix` | **have** |
| Adjustment layer | filters on a dest-time span, all under it | a subgraph with `enable=` | **have** — porcelain `adjustments[]` stores `op: group` with `enable` on the group; compile expands children and inherits `enable=` |

### 5.1 `blend` modes (this ffmpeg, 0–39)

`normal`, `addition`, `addition128`/`grainmerge`, `and`, `average`,
`burn`, `darken`, `difference`, `difference128`/`grainextract`,
`divide`, `dodge`, `exclusion`, `extremity`, `freeze`, `glow`,
`hardlight`, `hardmix`, `heat`, `lighten`, `linearlight`, `multiply`,
`multiply128`, `negation`, `or`, `overlay`, `phoenix`, `pinlight`,
`reflect`, `screen`, `softlight`, `subtract`, `vividlight`, `xor`,
`softdifference`, `geometric`, `harmonic`, `bleach`, `stain`,
`interpolate`, `hardoverlay`.

Per-component `c0_mode`… and `c0_opacity`. Timeline-capable.

### 5.2 Host blend modes (eonik `LayerBlendMode`)

`normal`, `multiply`, `screen`, `overlay`, `darken`, `lighten`,
`difference`, `exclusion`, `hardLight`, `softLight`, `colorDodge`,
`colorBurn`, `hue`, `saturation`, `color`, `luminosity`.

Host aliases (`hardLight`, `colorDodge`, …) canonicalize to ffmpeg
names. Hue/saturation/color/luminosity are **HSL channel blends**.
ffmpeg `blend` does not have those four as named modes. They fail
closed (**guest**).

---

## 6. Motion, speed, interpolation

| Dimension | What it is | ffmpeg | graft 0.2 |
| --- | --- | --- | --- |
| Constant speed | 0.5×, 2× | `setpts=PTS/speed`, `atempo` chained (0.5–2.0 per), `rubberband` **toolchain** | **have** — `params.speed` only |
| Time remap / curve | speed keyframes | `setpts` expression, `minterpolate` | **have** — `binding.remap` is stepwise speed; encode trims each span and concats working pads. Intra fails closed. `minterpolate` is a graph op, not the default remap |
| Reverse | play backward | `reverse`, `areverse` | **have** — graph `reverse` |
| Freeze / hold | hold a frame | `tpad=stop_mode=clone`, `loop` | **have** — graph `freeze` |
| Optical flow / motion blur | slow-mo quality | `minterpolate`, `tblend`, `tmix`, `framerate` | **have** — graph ops; not the default remap |
| Zoom / pan / Ken Burns | stills and crops | `zoompan`, `crop`+`scale` expressions | **have** — graph `zoompan` (constant `z`/`d`) |
| Motion paths | overlay x(t), y(t) | overlay `x`/`y` expressions, `eval=frame` | **have** — `binding.motion` eased `{t,v,ease?}`; mix key only |
| Keyframes / easing | any param vs dest t | `enable`, expressions | **have** — `keys` on a graph node. Expression options ease in the filter string (`eval=frame`). Numeric options are `enable=` segments. Remap stays hold. `sendcmd` is not the IR |
| Speed vs audio lock | picture and sound stay together | `setpts` + `atempo` / resample | **have** on the speed path only |

Retiming invalidates GOP-copy of that span. It does not invent creative
speed from a signal (existing rule).

---

## 7. Titles, captions, graphics, brand

| Dimension | What it is | ffmpeg | graft 0.2 |
| --- | --- | --- | --- |
| Burned text | titles, lower thirds | `drawtext` **toolchain** (this binary: **missing**) | **absent** |
| Shaped text / motion graphics | host canvas | rasterize in host → overlay pad | **guest** |
| Vector / shape layers | rules, keylines | host raster or `geq`/`drawbox`/`drawgrid` | **have** — graph `drawbox` / `drawgrid` (host shapes still rasterize to overlay) |
| Image overlay | logo, end card | `overlay` + `format` | **have** — brand + `overlay` |
| Stills as layers | PNG with alpha | decode + overlay | **have** — `-loop 1` on PNG/JPEG/GIF/WebP overlay materials |
| Open captions burned | burned-in | `drawtext` / `subtitles` | **absent** |
| Closed captions sidecar | not in picture | WebVTT / SRT / MOV timed text / CEA-608/708 | **have** — captions role sidecar + mux `mov_text` into the dest |
| ASS / SRT render | styled | `subtitles`, `ass` **toolchain** | **absent** |
| Brand grade + logo | look + bug | lut3d + overlay | **have** — `lut3d` + overlay nodes |
| Slate / bars / countdown | head | `smptebars`, `smptehdbars`, `color`, `testsrc`, `aevalsrc` | **have** — lavfi source graph ops (no picture input) |

Captions as sidecar do not dirty picture. Burned captions do.

---

## 8. Film look, grain, legal, platform finish

| Dimension | What it is | ffmpeg | graft 0.2 |
| --- | --- | --- | --- |
| Film grain add | texture | `noise`, `grain` (if built), libplacebo **toolchain** | **have** — `noise` |
| Grain remove / manage | denoise vs preserve | `removegrain`, `hqdn3d`, `nlmeans`, `atadenoise`, `bm3d`, `fftdnoiz` | **have** — graph ops this binary has (`hqdn3d`, `nlmeans`, `atadenoise`, `removegrain`, …) |
| Sharpen / soft / bloom | finish | `unsharp`, `smartblur`, `cas`, `sab`, `avgblur`, `gblur`, `boxblur` | **have** — `unsharp` / `gblur` |
| Vignette / glow / flare | optical | `vignette`, `geq` | **have** — `vignette` |
| Deband / deflicker | graded footage | `deband`, `gradfun`, `deflicker` | **have** — graph `deband` / `gradfun` / `deflicker` |
| Legal / broadcast | 16–235, no illegal RGB | `lut1d`, `eq`, `waveform` check, `colorspace=range=tv` | **have** — `lut1d` / `setrange` / `eq`; layer name `legal` is still an opinion |
| Platform loudness | Meta/TikTok/YouTube | see audio §10.6 | **partial** — `loudnorm` params (`I`, `LRA`, `TP`, `dual_mono`) **have**; no platform policy checker |
| Platform picture spec | 9:16, 1080, 30/60, H.264 high, yuv420p | dest + delivery encoder | **partial** — dest exists; not a spec checker |

---

## 9. Restoration and analysis (often pre-grade)

| Dimension | ffmpeg | graft 0.2 |
| --- | --- | --- |
| Stabilize | `deshake`, `vidstabdetect`/`vidstabtransform` **toolchain** | **absent** |
| Deinterlace / IVTC | §2 | **have** — graph ops in §2. `idet` does not invent fields |
| Denoise (spatial/temporal) | `hqdn3d`, `nlmeans`, `atadenoise`, `owdenoise`, `vaguedenoiser`, `dctdnoiz` | **have** — graph ops this binary has |
| Declick / declip (audio) | `adeclick`, `adeclip` | **have** — graph ops |
| Scene detect | `scdet`, `select` | **absent** (do not infer slots from pixels) |
| Black / freeze / silence detect | `blackdetect`, `freezedetect`, `silencedetect` | **absent** as recipe; tools only |
| QA metrics | `psnr`, `ssim`, `xpsnr`, `vmaf` **toolchain**, `astats`, `ebur128` | **absent** |

Do not invent a hook from `silencedetect`. Slots stay declared.

---

## 10. Audio (complete)

Picture roles are not the audio graph. eonik already has `music` /
`voice` / `sfx`. graft has `vo` / `bed` as overlay AAC after spine.

### 10.1 Sources and roles (host words)

| Role | What it is | Typical grain |
| --- | --- | --- |
| Production / sync dialogue | from the picture take | follows picture range unless J/L |
| VO / voiceover | separate take, placed on dest clock | clip + fades |
| Bed / music | underscore, often spans joins | **one stream** or looped |
| SFX one-shot | hit, whoosh, sting, flash, UI at time T | short file + dest start |
| SFX designed / riser | may overlap a transition on purpose | aligned to xfade offset |
| Foley | footsteps, cloth, props | many short clips |
| Ambience / room / wild | loop or long file | spans |
| Stinger / logo sting | end or bumper | dest-placed |
| Dub / alt language | replaces VO, picture holds | new scion, picture pads reuse |

graft: `vo` + `bed` + `sfx` **have** as overlay audio pads on compiled
dest time.

### 10.2 Edit on the sample clock

| Dimension | ffmpeg | graft 0.2 |
| --- | --- | --- |
| Trim / place | `atrim`, `asetpts`, `adelay`, `apad` | **partial** — source range on binding.audio |
| Fade in/out | `afade` + curves §4.2 | **have** — graph `afade` |
| Crossfade | `acrossfade` (`overlap`, `duration`, `curve1/2`) | **have** — overlap joins + `acrossfade` |
| Mix N | `amix=inputs=:dropout_transition=:normalize=` | **have** — spine fold + vo/bed/sfx |
| Channel map / pan | `channelmap`, `channelsplit`, `pan`, `join`, `amerge` | **partial** — graph `pan` **have**; dest still stereo AAC |
| Sample rate / fmt | `aformat`, `aresample` | **have** — working PCM 48 kHz stereo; AAC at deliver |
| Reverse / loop | `areverse`, `aloop` | **have** — graph `areverse` / `aloop` |
| Pitch without duration / duration without pitch | `asetrate`+`aresample`, `atempo`, `rubberband` | **have** atempo on speed only |

### 10.3 Dynamics, tone, space

| Dimension | ffmpeg | graft 0.2 |
| --- | --- | --- |
| Volume / gain keyframes | `volume`, `compand` | **have** — `volume`, including `keys` when the option is an expression |
| EQ (peaking, shelf, HP/LP, graphic) | `equalizer`, `bass`, `treble`, `highpass`, `lowpass`, `highshelf`, `lowshelf`, `anequalizer`, `superequalizer`, `firequalizer`, `biquad`, `bandpass`, `bandreject` | **have** — graph ops this binary has |
| Dynamic EQ / DRC | `adynamicequalizer`, `adrc` | **have** — graph ops this binary has |
| Compress / limit / gate / clip | `acompressor`, `alimiter`, `agate`, `compand`, `mcompand`, `acontrast`, `asoftclip`, `apsyclip` | **have** — graph ops this binary has |
| Sidechain duck (bed under VO) | `sidechaincompress`, `sidechaingate` | **have** — `sidechain` on slot pads, premix |
| De-esser | `deesser` | **have** |
| Dialogue enhance / speech norm | `dialoguenhance`, `speechnorm` | **have** |
| Reverb / delay / echo | `aecho`, `chorus`, `flanger`, `aphaser`, `apulsator` | **have** — graph ops this binary has |
| Stereo / width / Haas / surround | `extrastereo`, `stereotools`, `stereowiden`, `haas`, `earwax`, `crossfeed`, `surround`, `headphone` | **have** — graph ops this binary has |
| Exciter / sub / tilt | `aexciter`, `asubboost`, `asubcut`, `asupercut`, `atilt`, `virtualbass` | **have** — graph ops this binary has |
| Denoise (audio) | `afftdn`, `afwtdn`, `anlmdn`, `arnndn` | **have** — graph ops this binary has |

### 10.4 Loudness and legal (audio)

| Dimension | Spec | ffmpeg | graft 0.2 |
| --- | --- | --- | --- |
| EBU R128 | integrated LUFS, LRA, true peak | `loudnorm` (`I`, `LRA`, `TP`), `ebur128` | **have** — `loudnorm` |
| ReplayGain | file tags | `replaygain` | **absent** |
| Platform targets | ads often ≈ −14 to −16 LUFS, TP −1 to −2 | same nodes, dest policy | **absent** |
| Dual mono | VO treated as dual-mono | `loudnorm=dual_mono=1` | **have** — `loudnorm` param `dual_mono` |

A loudnorm **after** the mix is a sink-adjacent node. Changing one SFX
hit dirties mix + loudnorm, not a clean picture subgraph.

### 10.5 Codecs (audio delivery vs working)

Working: `pcm_s24le` / `pcm_f32le` / `flac` in the action cache.

Delivery (common): `aac` (LC), `libopus`, `libmp3lame`, `ac3`/`eac3`,
`alac`. graft **have**: AAC 48 kHz stereo per spine slot, then mux.

Do not AAC-encode each slot if a later `amix` or `loudnorm` must run.
Same C4 rule as picture.

---

## 11. Sync, timecode, clocks

| Dimension | What it is | ffmpeg / other | graft 0.2 |
| --- | --- | --- | --- |
| A/V offset | VO late by 40 ms | `adelay`, `asetpts`, binding offset | **absent** as a first-class field |
| Source timebase vs dest | 24/1.001, 30/1.001 | `settb`, `asettb`, rational rate | **partial** — dest.rate; concat probes timebase |
| Embedded timecode | LTC/VITC, tmcd | probe side data | **absent** |
| Wall / cue | wait for a cue | `acue` | **absent** (not ads) |
| Clap / waveform align | host problem | host | **guest** |

---

## 12. Codecs, containers, working vs delivery

The codec is **after** the graph (C2). Two different jobs:

### 12.1 Working / cache (node outputs)

| Family | Grain | Use |
| --- | --- | --- |
| Image seq (EXR, DPX, PNG, TIFF) | frame | grade, key, highest depth |
| Intra / All-I (ProRes, DNxHR, `graft-intra`, MJPEG) | frame | preview + cache |
| FFV1 / `ffv1` | frame, lossless | archival working |
| PCM / FLAC | sample | audio cache |
| Uncompressed (`rawvideo`, `pcm_s24le`) | frame/sample | honest, huge |

graft intra path exists for picture and **fails closed** when
`scion.graph` is non-empty or extra overlay pads are bound. The default
ffmpeg path writes **ProRes HQ working nodes** and encodes delivery
x264 **once** at the sink.

### 12.2 Delivery (sink)

| Family | Notes | this ffmpeg |
| --- | --- | --- |
| H.264 / AVC | `libx264`, `h264_videotoolbox`; yuv420 8-bit typical; High profile; closed GOP if you need concat-copy | **have** as **delivery** sink (working is FFV1) |
| HEVC | `libx265`, `hevc_videotoolbox`; 8/10-bit; HDR | **absent** |
| AV1 | `libsvtav1` | **absent** as dest impl |
| VP8 / VP9 | `libvpx` / `libvpx-vp9` | **absent** |
| ProRes / DNx | mezzanine delivery | encoders present; not dest impl |
| GIF / APNG / WebP | previews, not ads | **absent** as dest |

### 12.3 Containers and mux

`mp4`/`mov` (faststart), `mkv`, `webm`, `mxf`, HLS/CMAF (`hls`, segment
+ playlist). Audio may be a sidecar. Captions may be `mov_text` or
`.vtt`.

graft: mp4 mux at the delivery sink (working picture + mixed PCM → AAC). **have**.

### 12.4 Bitstream-copy conditions (accept them)

`concat -c copy` / `-c copy` splice requires **identical**: codec,
profile, size, fps, pix_fmt, color tags, timebase, and (for H.264)
closed GOP starting on IDR.

This is a **narrow special case** of C6: the node *is* already the
delivery bitstream and **no downstream filter exists**. The moment you
add lut3d, xfade, overlay-with-enable, or loudnorm after that point,
copy is illegal. Do not build the product on the special case.

---

## 13. Metadata, platforms, legal (non-pixel)

| Dimension | Notes | graft 0.2 |
| --- | --- | --- |
| Faststart / moov | `-movflags +faststart` | check encoder args |
| Rotation matrix | iOS | **absent** |
| Language / alternate audio | dubs | **partial** — new scion pattern in examples |
| Content ID / ISRC / brand ids | tags | **absent** |
| Platform duration caps | 6s, 15s, 30s | dest clock + host |
| End-card / safe zones | Meta/TikTok | **absent** as nodes |
| Loudness / picture legal | §8, §10.6 | **absent** |
| IMF CPL / OPL | finish, not authoring | non-goal to *be* IMF; may emit later |

---

## 14. Generators

Used as pads (black, bars, tone, countdown), not as “the ad.”

Video sources: `color`, `smptebars`, `smptehdbars`, `pal75bars`,
`pal100bars`, `testsrc`, `testsrc2`, `rgbtestsrc`, `yuvtestsrc`,
`haldclutsrc`, `gradients`, `colorchart`, `zoneplate`, `perlin`,
`cellauto`, `life`, `mandelbrot`, `nullsrc`, `movie`, `buffer`.

Audio sources: `aevalsrc`, `sine`, `anoisesrc`, `anullsrc`, `amovie`.

graft walkthrough used lavfi plates. That is a generator. Status: **have**
as lavfi graph ops (`color`, `smptebars`, `testsrc`, `sine`, …) with no
picture input. A name this binary lacks is `toolchain`.

---

## 15. Signals and addressing (already graft)

Keep. Do not let this file erase them.

- Declared window on a slot (hook_rate `[0, 3)`).
- Time map on an exact shipped **delivery** build.
- Dirty-from-signal = pads whose dest span intersects the metric.
- Spill threshold. `hook_rate` must not dirty a body **subgraph** that
  the metric does not touch.
- A look that covers the whole dest **does** sit in that intersection.

---

## 16. ffmpeg filter index (this binary, grouped)

481 filters. Hardware twins (`*_cuda`, `*_vaapi`, `*_vulkan`, `*_vt`)
are the same family. Do not add an IR node per GPU twin; pick a backend
in the toolchain fingerprint.

**Audio processors:** `aap`, `acompressor`, `acontrast`, `acrusher`,
`adeclick`, `adeclip`, `adecorrelate`, `adelay`, `adenorm`,
`aderivative`, `adrc`, `adynamicequalizer`, `adynamicsmooth`, `aecho`,
`aemphasis`, `aeval`, `aexciter`, `afade`, `afftdn`, `afftfilt`, `afir`,
`aformat`, `afreqshift`, `afwtdn`, `agate`, `aiir`, `aintegral`,
`alimiter`, `allpass`, `aloop`, `amerge`, `amix`, `amultiply`,
`anequalizer`, `anlmdn`, `anlmf`, `anlms`, `apad`, `aphaser`,
`aphaseshift`, `apsyclip`, `apulsator`, `aresample`, `areverse`, `arls`,
`arnndn`, `asetrate`, `asetpts`, `asoftclip`, `asubboost`, `asubcut`,
`asupercut`, `asuperpass`, `asuperstop`, `atempo`, `atilt`, `atrim`,
`axcorrelate`, `bandpass`, `bandreject`, `bass`, `biquad`, `channelmap`,
`channelsplit`, `chorus`, `compand`, `compensationdelay`, `crossfeed`,
`crystalizer`, `dcshift`, `deesser`, `dialoguenhance`, `dynaudnorm`,
`earwax`, `equalizer`, `extrastereo`, `firequalizer`, `flanger`, `haas`,
`hdcd`, `headphone`, `highpass`, `highshelf`, `join`, `loudnorm`,
`lowpass`, `lowshelf`, `mcompand`, `pan`, `sidechaincompress`,
`sidechaingate`, `silenceremove`, `speechnorm`, `stereotools`,
`stereowiden`, `superequalizer`, `surround`, `treble`, `tremolo`,
`vibrato`, `virtualbass`, `volume`, plus `acrossfade`, `acrossover`,
`aevalsrc`, `afirsrc`, `anoisesrc`, `anullsrc`, `sine`, `sinc`.

**Color / look:** `lut3d`, `lut1d`, `lut`, `lutrgb`, `lutyuv`, `lut2`,
`tlut2`, `haldclut`, `colormap`, `colormatrix`, `colorspace`,
`colortemperature`, `colorbalance`, `colorchannelmixer`, `colorcorrect`,
`colorhold`, `colorize`, `colorlevels`, `colortemperature`, `curves`,
`eq`, `hue`, `huesaturation`, `vibrance`, `selectivecolor`, `exposure`,
`gamma`, `normalize`, `histeq`, `tonemap`, `pseudocolor`, `swapuv`,
`shuffleplanes`, `setrange`, `setparams`.

**Geometry / time:** `trim`, `setpts`, `settb`, `fps`, `framerate`,
`minterpolate`, `scale`, `scale2ref`, `crop`, `pad`, `zoompan`,
`rotate`, `transpose`, `hflip`, `vflip`, `shear`, `scroll`, `perspective`,
`tpad`, `loop`, `reverse`, `thumbnail`, `fps`, `setdar`, `setsar`.

**Composite / transition:** `overlay`, `blend`, `xfade`, `mix`,
`hstack`, `vstack`, `xstack`, `tile`, `maskedmerge`, `alphamerge`,
`premultiply`, `unpremultiply`, `threshold`, `chromakey`, `colorkey`,
`lumakey`, `chromahold`, `fade`, `concat`, `tpad`.

**Finish / restore:** `unsharp`, `cas`, `smartblur`, `sab`, `avgblur`,
`gblur`, `boxblur`, `vignette`, `noise`, `deband`, `gradfun`, `deflicker`,
`deshake`, `nlmeans`, `hqdn3d`, `atadenoise`, `bm3d`, `fftdnoiz`,
`removegrain`, `yadif`, `bwdif`, `w3fdif`, `telecine`, `detelecine`.

**Draw / generate:** `geq`, `drawbox`, `drawgrid`, `drawgraph` (no
`drawtext` here), `color`, `smptebars`, `testsrc*`, `gradients`,
`movie`, `amovie`.

**Analysis (not recipe):** `psnr`, `ssim`, `xpsnr`, `signalstats`,
`astats`, `ebur128`, `blackdetect`, `silencedetect`, `scdet`, `siti`,
`showinfo`, `waveform`, `vectorscope`.

If a needed filter is missing from the binary, that is C11, not a
reason to invent a second compositor.

---

## 17. What a host already authors (eonik)

The first host is not “three takes.” It already has:

- Shots with in/out on library clips
- `TransitionKind` list in §4.3 (including whip and flash)
- `LayerBlendMode` list in §5.2
- `AudioRole`: music, voice, sfx
- Timeline: video tracks, audio tracks, `textElements`, `overlays`,
  `ShapeElement`, `AdjustmentSpan` (`filter` / `effect` / `adjustment`)
- Export via AVFoundation + `CIVideoCompositor` (overlap holds A and B)
- Brand stills, product pixels, VO takes

Usable graft means: **emit pads for those objects** and lower them.
It does not mean: ask the host to export `takes/hook.mov`.

---

## 18. graft 0.2 coverage (honest)

| Family | Status |
| --- | --- |
| Declared slots + dest clock + signal windows | **have** |
| Binding in/out on a material | **have** — porcelain `composition.json` `slots[]` (`file` + `in`/`out` on a library clip). Named `takes/<role>.*` is a crutch |
| Speed constant | **have** |
| Working ProRes HQ + PCM nodes | **have** |
| Color normalize before join | **have** |
| Overlap xfade + acrossfade | **have** |
| Delivery encode once | **have** |
| `scion.graph` (lut3d, overlay, gap, …) | **have** |
| Brand overlay | **have** |
| Extra picture overlays (`role overlay`) | **have** — sequential `overlay` at `binding.x` / `y` / `opacity` |
| VO/bed/sfx mix | **have** |
| Captions sidecar | **have** — sidecar blob + muxed `mov_text` |
| Layer names grade/legal | **partial** as layers; porcelain `look` / `legal` / `eq` are graph nodes |
| CDL / curves / HDR / OCIO | **partial** — `cdl` / `lgg` / `curves` / `eq` / `tonemap` **have**; OCIO **absent** |
| SFX ducking (`sidechain`) | **have** — premix when inputs are `bed`+`vo` |
| Masks, keys, adjustment spans | **have** — `chromakey` / `maskedmerge` (third pad is the holdout) / porcelain `adjustments[]` stores a `group` and lowers with inherited `enable=` |
| Mixboard (EQ / compress / delay) | **have** — `equalizer` / `acompressor` / `alimiter` / `adelay` / `aecho` / `pan`, plus any audio filter this binary has |
| Stacks / mix N | **have** — `hstack` / `vstack` / `xstack` / `mix` / `tile` |
| Text / shapes / burned captions | **partial** — `drawbox` / `drawgrid` **have**; `drawtext` / `subtitles` **toolchain** |
| J/L cuts | **have** — `audio.source` + optional `audio.place` |
| Nested graphs | **have** — nest scion as working pad (RFC 0007). An in-graph `group` is stored and lowered with `enable=` (ffmpeg has no subgraph object) |
| Multi-cam switch | **have** — nest `switch.cuts` (RFC 0009). `streamselect` stays a static map |
| OTIO native effects/grades | **guest** loss; import restores `graph` / overlays **and** score joins / markers |
| Intra splice of a graph, extra overlay pads, or a nested scion | fail-closed — needs the ffmpeg working path |
| Spine concat of closed-GOP H.264 | special case only (not default) |

The ffmpeg path is pads in a working depth. Intra stays a splice and
refuses a graph.

---

## How to add

1. Find the family. If none fits, add a `##` section and a Changelog
   line. Do not start a second file for “more effects.”
2. One row per dimension: what it is, ffmpeg (or **toolchain** / host),
   graft status.
3. If the row forces a dirty-set or schema change, RFC + schema +
   `docs/schema.md` + example in the same PR. This file only records
   the dimension; it does not claim the compiler grew.
4. Do not add marketing names (`cinematic`, `AI grade`) without a
   lowerer.
5. Do not delete a row because it is unimplemented. **absent** is the
   point.

---

## Changelog

- **2026-10-13** — Filter lowerer, adjustment groups, easing (RFC 0010):
  a filter-shaped op this binary has lowers as `key=value` + `enable=`.
  A missing filter is `toolchain`. Unknown params fail closed.
  `adjustments[]` stores `op: group`. Compile expands children and
  inherits `enable=`. `keys` ease expression options (`eval=frame`) and
  segment numeric options. `loudnorm` `dual_mono` passes through.
  Still **absent** / **toolchain**: drawtext, subtitles/ass, OCIO,
  libplacebo. Still **guest**: whip/flash, HSL blends. Scene and
  silence detect do not invent slots.
- **2026-09-19** — Motion, remap, multi-cam switch (RFC 0009):
  `binding.motion` is piecewise-linear overlay x/y/opacity (mix key only;
  multi-key opacity is `geq`); `binding.remap` is stepwise speed on the
  working-source key (intra fails closed); nest `switch.cuts` authors a
  spine angle plus `source` pads and lowers with delayed `overlay`.
  `graft address` prefers `angles.switch`. `graft swap overlay --x --y`
  updates place without a file. Ops: `geq`, `selectivecolor`, `exposure`,
  `colorhold`, `colorize`, `deband`. Still **absent**: drawtext/OCIO,
  whip/flash, HSL blends, in-graph subgraph objects, easing beyond
  linear/stepwise.
- **2026-09-19** — Adjustment groups, overlay place, nest address, color
  convert (RFC 0008): porcelain `adjustments[]` stamps `enable=` on a
  node group; `binding.x` / `y` / `opacity` are mix geometry (not
  `slot_encode`); `graft address` names `plate.look` and next is
  `graft swap look --scion plate`; `colorspace` primaries/trc; `cdl` /
  `lgg` / `colorcorrect` / `bwdif` / `tile` / `haldclut`. Still
  **absent**: drawtext/OCIO, whip/flash, HSL blends, in-graph subgraph
  objects, multi-cam switch.
- **2026-09-19** — Nested scion-as-pad (RFC 0007): `binding.nested` and
  graph `nested` compile an inner score/scion to its working `#picture`
  (and working audio). Delivery concat is not nested essence. Porcelain
  `nests` + slot `"scion"`. OTIO sidecar carries `nested`. Intra fails
  closed on a nested pad. Still **absent**: drawtext/OCIO, whip/flash,
  HSL blends, in-graph subgraph objects, multi-cam switch.
- **2026-09-19** — Mixboard + matte + guest score (RFC 0006):
  `maskedmerge`, stacks/`mix`, host blend aliases, audio mixboard,
  chapter mux from `score.markers`. OTIO import upserts sidecar joins
  and markers onto the score. Porcelain `source` nodes take `in`/`out`.
  Still **absent**: nested graphs, drawtext/OCIO, whip/flash, HSL blends.
- **2026-09-19** — Host pads + guest sidecar (RFC 0005): role `overlay`,
  host join aliases, `score.markers`, OTIO sidecar restores
  `scion.graph` / overlays, address names covering nodes, intra
  fail-closed on a graph, geometry ops (`setsar` / `setdar` /
  `perspective` / `shear` / `scroll` / `premultiply` / `streamselect`),
  stills `-loop 1`. Porcelain `dest` / `spans` / `markers`. Still
  **absent**: nested graphs, drawtext/OCIO, whip/flash, three-pad mattes.
- **2026-09-19** — J/L audio (`binding.audio.place`), keys (`chromakey`
  / `colorkey` / `lumakey` / `chromahold` / `alphamerge`), shapes
  (`drawbox` / `drawgrid`), overlay opacity, captions muxed into the
  dest. Porcelain `slots[].audio` + captions/brand. RFC 0004. Scorecard
  rows updated. Still **absent**: drawtext/OCIO, nested graphs, multi-cam.
- **2026-09-19** — Porcelain authors C9 ranges: `composition.json`
  `slots[]` bind `file` + `in`/`out` on a library clip. `graft ship`
  no longer requires `hook.mov`. `graft swap hook file --in --src-out`
  rebinds a range (`--out` stays the dest path). Named takes remain a
  crutch.
- **2026-09-19** — Porcelain `takes/composition.json` authors the full
  have-set (joins, working, nodes). `swap` any node. Premix sidechain.
  Finish/grade ops added. Scorecard rows updated.
- **2026-09-19** — Porcelain closed loop: ship authors look/sfx/joins;
  resolve inherits `scion.graph`; `swap look` / `swap legal`; preview
  is `compile`. Scorecard: takes/ stories and grade/legal rows.
- **2026-09-19** — RFC 0003 implemented on the ffmpeg path: working
  graph, overlap joins, `scion.graph`, compiled dest ranges, delivery
  once. Scorecard updated. Intra unchanged.
- **2026-09-19** — First cut. Physics C1–C12. Families 1–15. ffmpeg 481
  filter grouping from the machine binary. `xfade` 58, `blend` 40,
  `afade` curves, color tags, working formats, codec split, eonik host
  surface, graft 0.2 scorecard. ADR 0010.
