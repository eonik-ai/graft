# dub-en @ 9x16

Dirty-set **contract**, not a dubbed film. `vo` mixes over spine AAC
([RFC 0002](../../docs/rfc/0002-overlay-joins-and-mix.md)); it does not
replace picture audio. Bindings hashed from real local essence. Mp4/mov
files are not in git.

Clock is **10s @ 30fps** (300 frames), cut from three 8s Veo clips at 24fps
720×1280 plus a 10.67s MyWonder take as vo. Dest geometry matches the
clips: 720×1280. graft does not infer this breakdown from pixels.

| Slot | Dest | Essence (BLAKE3 of local file) |
| --- | --- | --- |
| hook | `[0, 3)` | `eonik_ads_script/.../7cc2ecdd…/veo/clip_01.mp4` in `[0, 3)` @ 24fps |
| body | `[3, 7)` | `…/clip_02.mp4` in `[0, 4)` @ 24fps |
| cta | `[7, 10)` | `…/clip_03.mp4` in `[0, 3)` @ 24fps |
| vo (en) | `[0, 10)` | `~/eonik/mywonder_clip1.mov` in `[0, 10)` @ 30fps |
| vo (hi) | `[0, 10)` | `~/eonik/video2.mov` in `[0, 10)` @ 30fps |
| captions | `[0, 10)` | WebVTT cued from those dest spans (`clip_01` / `clip_02` / `clip_03`) |

`scion.json` is the flattened `dub-en` used by the time map.
`scions/picture.json` binds picture + captions only.
`scions/dub-en.json` / `dub-hi.json` parent `picture` and rebind `vo` on `copy`.
Hook/body/cta BlobIds stay the same across those children.

`bed` is on the score and unbound. Silence in unnamed picture is not a slot.

## Dirty sets on this clock

| Signal | Addressed dest | Dirty slots | Mixes | Clean spine |
| --- | --- | --- | --- | --- |
| `hook_rate` | declared `[0, 90)` | hook + overlapping overlays | `audio_mix` | body, cta |
| `vo_hold` (`dirty.json`) | declared `[0, 90)` | hook, captions, vo | `audio_mix` | **body**, cta |
| `note` at 3–5s | explicit `[90, 150)` | body, captions, vo | `audio_mix` | hook, cta |
| `hold` / too-slow | declared body `[90, 210)` | body, captions, vo | `audio_mix` | hook, cta |

`vo_hold` does not dirty `body`. An approver range with no matching window
uses dest frames on this build. `hold` names body; iterate does not invent
`params.speed`. Relating `dub-en` to `picture` is the same concept, parent
scion, and unchanged hook/body/cta hashes.

The load-bearing `hook_rate` → body-stays-clean fixture remains
[`../hook-v3-body-v1-9x16/`](../hook-v3-body-v1-9x16/).

graft does not detect silence in the mp4 or guess a replacement clip.
