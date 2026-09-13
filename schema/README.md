# schema

Normative machine contract for graft **0.1.0**.

JSON Schema draft 2020-12. Document `$id` values are URNs, not hosted
URLs, until this project has a stable HTTP home.

| File | Document |
| --- | --- |
| [score.schema.json](score.schema.json) | recipe: clock, slots, windows, layers |
| [scion.schema.json](scion.schema.json) | bindings + dest + encoder |
| [time-map.schema.json](time-map.schema.json) | compile output: dest time → slot |

Examples live in [`../examples`](../examples/). `make test` validates
them against these schemas (structural checks in stdlib; optional
`jsonschema` if installed).

Change process: RFC, then bump `0.1.0` in every `graft` field and `$id`.
