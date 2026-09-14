# Time map

A platform metric is a time range on the **shipped** file. A slot lives
on the **score clock**. The time map is the join, stored under the exact
build identity.

```
build → time map → dest frames → slot_id
```

Compile emits that map with dest ranges, source ranges, `scion`,
`scion_hash`, and `build`. Entries include every **bound** slot, spine
first, so a signal can address a `vo` or `captions` window. `hook_rate`
still does not dirty `body`. `graft signal` and `graft feedback ingest`
read only that artifact. A root `time-map.json` is a fixture, not
sufficient provenance for a live workspace.

## Signal

```
signal := (kind, declared or explicit range, dest_id, build)
```

If the score declares a window for `kind`, that range is used even when
the caller passes another `--t`. Dirty slots = time-map entries whose
dest range intersects the addressed range.

### hook_rate rule

For `kind = hook_rate`, always include the slot with `role: hook`.

If the declared window bleeds into body, **drop body** from the dirty set
when the overlap is strictly less than `spill_threshold_frames` (default
**11** frames, 0.367s at 30fps). Warn when overlap ≥ threshold: pad the
hook to the declared window, or accept a dirty body.

The window is **declared** on the score (`slot.window`). It is not inferred
from the binding duration.

## Worked map (23s @ 30fps, 9x16)

| Dest frames | Slot | Typical signal |
| --- | --- | --- |
| `[0, 90)` | hook | hook_rate, thumbstop |
| `[90, 600)` | body | hold / "too slow" on 8–12s |
| `[600, 690)` | cta | click-through, end-card skip |

Current behavior: `graft signal --kind hook_rate --build <id>` → slots
`{hook}`, kerfs `{hook→body}`. `body_v1` stays clean. A "too slow"
signal still names the slot; iterate does not invent `params.speed`.

## Overlay clock (10s @ 30fps, 9x16)

[`examples/dub-en-9x16/`](../examples/dub-en-9x16/) cuts a 10s dest from
real Veo clips (`clip_01`/`02`/`03`) plus a MyWonder take as `vo`. The
score names hook `[0, 3)`, body `[3, 7)`, cta `[7, 10)`. `vo` and
`captions` cover the whole dest. graft does not infer those names from
the mp4.

| Signal | Addressed dest | Dirty | Clean spine |
| --- | --- | --- | --- |
| `vo_hold` | declared `[0, 90)` | hook, captions, vo + `audio_mix` | **body**, cta |
| `note` at 3–5s | explicit `[90, 150)` | body, captions, vo + `audio_mix` | hook, cta |
| `hold` | declared body `[90, 210)` | body, captions, vo + `audio_mix` | hook, cta |

A full-span `vo` overlapping the hook window is dirty for `hook_rate` on
this clock. The load-bearing hook-only fixture remains
[`examples/hook-v3-body-v1-9x16/`](../examples/hook-v3-body-v1-9x16/).

Reference implementation: [`ref/graft_ref/signal.py`](../ref/graft_ref/signal.py).
The test in `ref/tests/test_signal.py` is part of the spec.
