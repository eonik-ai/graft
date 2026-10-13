# Reference CLI loop

Ship the next cut without recutting the tree. Drop a new hook, VO, or dest.
The clean slots stay. You play both.

This is the **reference host**. An app host should call `graft_compile::compile`
([hosts.md](hosts.md)), not scrape these verbs. The IR is [mission](mission.md)
and [ADR 0007](adr/0007-compiler-is-a-workspace-subsystem.md). Porcelain on
plumbing: [ADR 0009](adr/0009-porcelain-closes-the-founding-loop.md). Flags
live in [README.md](../README.md).

## Takes

A take directory is a recipe, not scene detection. Authorship is ranges
on a library clip (`composition.json` `slots[]`). Named `hook.mov` /
`body.mov` / `cta.mov` still bind if `slots[]` is omitted (the crutch).

```
takes/product.mp4
takes/composition.json # dest, slots[], nests, joins, markers, spans, adjustments, graph nodes
takes/vo.wav           # optional overlay; dest-time pad
takes/bed.wav
takes/sfx.wav
takes/look.cube        # optional lut3d if composition.json omits look
takes/legal.cube       # optional lut1d after look
takes/joins.json       # optional; composition.json.joins wins then merges
```

Unknown media names are listed and ignored. graft does not infer `hook`
/ `body` from pixels. A node or slot may name `file` relative to the
take directory; porcelain hashes that file into `material` and stores
the in/out on the binding.

```json
{
  "dest": "1080x1920",
  "working": {"pix_fmt": "yuv422p10le", "color": "bt709"},
  "joins": [{"left": "hook", "right": "body", "transition": "dipToBlack", "duration_frames": 6}],
  "markers": [{"id": "punch", "range": {"start": 0, "duration": 30}}],
  "spans": [{"id": "look", "enable": "between(t,0,4)"}],
  "adjustments": [
    {
      "enable": "between(t,0,1)",
      "nodes": [
        {"id": "grade", "op": "lgg", "params": {"lift": "0", "gamma": "1", "gain": "1", "sat": "1.05"}}
      ]
    }
  ],
  "nests": {
    "plate": {
      "slots": [{"id": "hook", "file": "product.mp4", "in": 0, "out": 1}],
      "nodes": [{"id": "look", "op": "lut3d", "file": "plate.cube"}]
    }
  },
  "slots": [
    {"id": "hook", "scion": "plate"},
    {"id": "body", "file": "product.mp4", "in": 1, "out": 3},
    {"id": "cta", "file": "product.mp4", "in": 3, "out": 4},
    {"id": "captions", "role": "captions", "file": "caps.vtt", "in": 0, "out": 4},
    {"id": "overlay", "role": "overlay", "file": "bug.png", "in": 0, "out": 4, "place": {"start": 0, "duration": 4}, "x": 8, "y": 16, "opacity": 0.8},
    {"id": "sfx", "role": "sfx", "file": "whoosh.wav", "in": 0, "out": 0.4, "place": 2.0}
  ],
  "nodes": [
    {"id": "look", "op": "lut3d", "file": "look.cube"},
    {"id": "box", "op": "drawbox", "params": {"x": "8", "y": "8", "w": "40", "h": "16", "color": "white"}},
    {"id": "eq", "op": "eq", "params": {"saturation": "1.05"}},
    {"id": "duck", "op": "sidechain", "inputs": ["bed", "vo"]},
    {"id": "loud", "op": "loudnorm", "params": {"I": "-14"}},
    {"id": "fill", "op": "source", "file": "fill.mp4", "in": 0, "out": 4},
    {"id": "mask", "op": "source", "file": "mask.png", "in": 0, "out": 4},
    {"id": "matte", "op": "maskedmerge", "inputs": ["#picture", "fill", "mask"]},
    {"id": "comp", "op": "acompressor"}
  ]
}
```

Missing join is a cut. `graft swap <node-id>` updates that node's
material or params (`.cube` / media / JSON). `graft swap hook file --in
--src-out` rebinds a range; `--out` is the dest path.

## Porcelain

```
takes/  →  graft ship     →  first dest from library-clip ranges + look/sfx/joins
                │
                ├──────── →  graft swap hook file --in --src-out  →  next dest, body HIT, look kept
                │
                ├──────── →  graft swap look file --scion plate   →  nest look miss; outer body HIT
                │
                ├──────── →  graft swap look|eq|…   →  slots HIT, that node misses
                │
                └──────── →  graft address --kind hook_rate --build <id>
                                   →  empty child (graph inherits) + covering nodes
                                   →  “graft swap look <file> --scion plate”
                                      when the dirty slot is a nest
                                   →  otherwise “graft swap hook <file> [--in --src-out]”
                                      (or swap <node> when no slot is dirty)
```

`swap --from` compiles every take in a folder against the same parent so
the body encode is paid once. `graft preview` is `compile`, not a second
filtergraph.

A child with `graph: null` inherits the parent graph. Fork and iterate
leave `graph` unset on purpose. `swap look` / `swap legal` write the
override on the child.

Plumbing (`scion fork`, `bind`, `compile`, `signal`, `iterate`) is still
there. Porcelain calls it.

## Ledger

A compile that encoded prints dirty slots, kerfs, graph nodes, delivery
cache, clean slots with BlobIds, encode wall-clock, dest path, and build
id. `ship` fails if it cannot encode a dest. `swap` fails if a sibling
`slot_encode` recoded when only that slot or look changed.
