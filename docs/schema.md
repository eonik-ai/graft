# Schema commentary

**Normative** documents are the JSON Schema files in [`/schema`](../schema/).
This page is commentary. If they disagree, the schema wins; then file a PR
to fix this page.

Version: **0.1.0**. The `graft` field on every document must match.

## Documents

A compile reads three JSON documents (plus CAS blobs that are not in git):

| File | Job |
| --- | --- |
| `score.json` | Slots on a clock, optional declared windows, layers, default dest name |
| `scion.json` | Bindings + dest + encoder fingerprint |
| `time-map.json` | Written **by the compiler** for a build; input to `graft signal` |

Materials live in CAS, referenced as `blake3:<64 hex>`.

## Slot vs dest

```json
{ "id": "hook", "role": "hook", "span": [0.0, 3.0], "window": { "kind": "hook_rate", "span": [0.0, 3.0] } }
```

`span` is where the slot lives on the **score clock**. `window` is what a
metric means. If a future cut shortens the hook take to 2.4s but leaves
the window at 3s, the compiler warns: pad, or accept body spill.

`9x16` is not a slot. It is `scion.dest`.

## Binding

```json
"hook": {
  "material": "blake3:…",
  "in_s": 0.4,
  "out_s": 3.4,
  "params": { "speed": 1.0 }
}
```

`in_s`/`out_s` are on the **material** timeline. Speed, crop, grade refs
are `params` and part of `slot_encode`.

## Layers

`base | copy | grade | legal`. Later names override the same slot with
higher strength. This is not a pixel blend. Copy layer changing a CTA
line does not re-encode body unless the dest text renderer is in-body
(it should not be: captions are their own slot).

## Worked scion

See [`examples/hook-v3-body-v1-9x16/`](../examples/hook-v3-body-v1-9x16/).
That fixture is part of the contract: `make test` requires that
`hook_rate` on `[0, 3)` dirties hook + hook→body kerf only.
