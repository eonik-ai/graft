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
variant derivation, not Git history.

## Binding

```json
"hook": {
  "material": "blake3:…",
  "source": {
    "rate": { "num": 30, "den": 1 },
    "range": { "start": 12, "duration": 90 }
  },
  "params": { "speed": 1.0 },
  "audio": { "material": "blake3:…", "source": { "rate": { "num": 30, "den": 1 }, "range": { "start": 12, "duration": 90 } } }
}
```

Source duration at `params.speed` must match the slot's dest duration
(one-frame tolerance). **Retime is not applied yet** — speed ≠ 1 invalidates
the cache key but does not resample frames.

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
