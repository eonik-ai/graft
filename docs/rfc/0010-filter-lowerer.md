# RFC 0010 — the filter lowerer, adjustment groups, easing

- Status: accepted, implemented
- Date: 2026-10-13
- Implements: ADR 0010, [composition.md](../composition.md) C1 / C6 / C11
- Format id: `0.2.0` (additive)

## Problem

The inventory says ffmpeg is the lowerer. The compiler still treated a
video as a hand-maintained `NODE_OPS` allowlist plus a match arm per
filter. A real filter this binary has (`hqdn3d`, `gradfun`, `deesser`)
failed as `unknown_op`. Extra params on a dedicated arm were dropped.
Porcelain `adjustments[]` flattened the group and stamped `enable=` on
each child, so the layer was not in the IR. Keyframes were linear or
stepwise, and only on overlay motion and remap.

Adding another arm for each missing row was the shortcut.

## Decision

1. Score `validate` accepts a structural op or a filter-shaped name
   (`^[a-z][a-z0-9_]{0,63}$`). It does not run ffmpeg. `cinematic` is a
   legal name at validate. `Look!` is `unknown_op`.
2. Compile probes this binary (`ffmpeg -filters`, `ffmpeg -h filter=`).
   Structural ops keep dedicated lowerers (`source`, `overlay`, `xfade`,
   `switch`, `cdl`, `lgg`, `sidechain`, stacks, mix, …). Any other name
   this binary has lowers as `key=value` plus `enable=`. A missing
   filter (`drawtext`, `subtitles`, `ocio`, `libplacebo`) is
   `{"error":"toolchain","op":"…"}`. An unknown param is
   `{"error":"unknown_param","op":"…","param":"…"}`. Params are not
   dropped. Dedicated 1:1 arms pass through options they did not consume
   (`loudnorm` `dual_mono`).
3. A filter with two or more inputs uses those pads. A lavfi source
   (`color`, `smptebars`, `sine`, …) has no picture input. `gap`,
   `source`, `nested`, and `group` are not lavfi sources.
4. Optional `keys` on a graph node is `{param: [{t, v, ease?}]}`.
   `ease` is `hold`, `linear`, `ease_in`, `ease_out`, or `ease_in_out`,
   and it is how `v` is approached from the previous key. Expression
   options (string type and runtime `T`) lower to an ffmpeg expression
   and set `eval=frame` when that option exists. Numeric options lower
   as `enable=` segments (`hold` is one segment per key; other eases
   sample at the dest frame rate). `sendcmd` is not the IR. Linear
   overlay expressions stay the historical
   `if(lt(t,…),v0+(v1-v0)*(t-t0)/dt,…)` form. Remap stays stepwise;
   any other ease on a remap key is `bad_remap`.
5. An adjustment is a stored `group` node. Children keep their own
   nodes. `enable` lives on the group. Lowering expands the children
   and copies `enable` onto a child that has none, because ffmpeg has
   no subgraph object. The stored scion keeps the group. Address names
   the group and the children. A swap of a child forks when that scion
   binds the child anywhere in the tree. The child key includes `keys`
   and the inherited `enable`.
6. Generic audio is routed with the probed class before a node is
   treated as a picture writer. `deesser` updates `#audio`.

## Not this increment

- `drawtext`, `subtitles` / `ass`, OCIO, `libplacebo` (C11: this binary
  does not have them).
- Whip / flash. HSL blend modes (`hue`, `saturation`, `color`,
  `luminosity`).
- Scene detect or silence detect inventing slots.
- `minterpolate` as the default remap. It is a graph op.
- A platform loudness policy checker. `loudnorm` params, including
  `dual_mono`, are the node.
- HEVC / AV1 / HDR side data. Chroma location and a rotation matrix
  stay encoder and container flags.
