# Glossary

Always lowercase **graft** for the project and the command, like **git**.

| Term | Meaning |
| --- | --- |
| **concept** | One intent. UUID. Owns score history. Not a file. |
| **score** | The recipe: ordered slots, layers, dest defaults. Text. Git-able. Analog of a USD stage or IMF CPL. |
| **slot** | Typed span on the score clock: `id`, `role`, `[t0, t1)`, constraints. Addressable by name. |
| **role** | Public addressing scheme: `hook` `body` `proof` `cta` `vo` `captions` `bed` `brand`. |
| **window** | Declared metric span on a slot (e.g. hook_rate `[0, 3)`). May differ from the take's duration. |
| **material** | Immutable essence. CAS hash of original bytes. |
| **binding** | `slot_id → { material, in, out, params }`. What a variant swaps. |
| **params** | Crop, speed, grade ref, volume — anything that changes the encode of that slot. |
| **layer** | USD-like opinion: `base` \| `copy` \| `grade` \| `legal`. Strength, not a pixel merge. |
| **dest** | Output spec: size, fps, color, encoder fingerprint. `9x16` is a dest. |
| **scion** | A named variant: concept + bindings + dest. Example: `hook_v3 + body_v1 + cta_v1 @ 9x16`. |
| **rootstock** | Metaphor: the concept / score the scion is grafted onto. |
| **build** | Immutable compile identity plus cached encodes, kerfs, and concatenated output. |
| **slot_encode** | Cache object: one slot rendered to a dest. Key in [compile.md](compile.md). |
| **kerf** | Re-encoded join (usually one GOP each side). Named for the millimetre of wood lost at a graft union. |
| **time map** | For one shipped build: `dest_t → slot_id`. Regenerated on retime. |
| **dirty set** | Slots and kerfs the compiler must rebuild. |
| **spill threshold** | Default `11` frames. Hook-rate bleed into body below this does not dirty body. |
| **feedback** | Raw platform metric plus resolved slots/kerfs against one build. |
| **CAS** | Content-addressed store of materials (and, on a server, of slot_encode / kerf blobs). |
| **adapter** | Guest import/export. Lossy by default. See [adapters.md](adapters.md). |
| **grain** | Smallest replaceable unit: frame (intra) or GOP (Long-GOP). |
