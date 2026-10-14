# Hosts

A **host** authors scores and scions, stores essence, and asks graft to
compile a dest. graft is the compiler. The host is the product the person
sees.

eonik is meant to be a host. It does not compile through these crates yet.
The plug is the recipe plus `graft compile` / `graft_compile::compile`.
Premiere, Final Cut, DaVinci Resolve, Shotcut, and Kdenlive are guests.
`graft export guest` writes the file each of them already imports.
They are not hosts. See [adapters.md](adapters.md) and principle 11.

## Rust

Dependency direction: host → `graft-compile` → `graft-cas` → `graft-score`.
Adapters depend on `graft-score` only.

```rust
graft_compile::compile(&score, &scion, &store, &encode, &concat)
// nested pads: compile_in(..., &family) where family is nests/<id> score+scion
```

`score` / `scion` come from `graft-score` (or JSON that matches
[`schema/`](../schema/)). `store` is `graft_cas::Store` (filesystem
`.graft/` is `graft_cas::Fs`). Long-GOP encode is `graft_compile::FfmpegX264`
plus the concat backend the CLI uses. Misses encode; hits return the
cached `BlobId`.

`CompileOutcome` carries the plan (dirty vs clean), the time map, and
the concat blob when materials were present. JSON examples in this repo
compile to a **plan** because their bindings are placeholder hashes.

Crates stay `publish = false`. Depend on the git remote:

```toml
graft-compile = { git = "https://github.com/eonik-ai/graft.git" }
```

There is no C ABI, Swift package, or HTTP compile server. A non-Rust host
either shells the `graft` CLI or adds an FFI later. Adding FFI is new
work; do not claim it exists.

## CLI as reference host

`graft ship` / `swap` / `address` write recipe JSON, put takes in CAS,
and call the same `compile`. [loop.md](loop.md) is that walk. A real app
host should call the crates, not scrape CLI output.

## What a first eonik wire looks like

1. Author `hook` / `body` / `cta` (and overlays) as score slots whose
   bindings are **ranges on library clips**, plus `score.joins` (host
   aliases canonicalize to xfade names), `score.markers`, `dest.working`,
   extra `overlay` pads, track-matte / mixboard graph nodes, and
   `scion.graph` (LUT, eq, duck, loudnorm, finish) — not a CapCut
   timeline and not pre-cut `hook.mov` files. The CLI writes those from
   `takes/composition.json` (`dest`, `slots[]` `file` + `in` / `out`,
   optional `audio` for J/L, captions / brand / overlay files, `spans`,
   `markers`, `adjustments[]`, overlay `x` / `y` / `opacity`, graph
   `source` `in`/`out`, `nests` + slot `"scion"`).
   A nest compiles to its working `#picture`, not the delivery mp4.
2. Bind each range and each LUT by putting bytes in CAS and writing the
   `blake3:` id plus source in/out (and optional `audio.place`) on the
   scion (or graph node). Captions mux into the dest.
3. Call `compile` for dest A. Preview is that compile, not a second
   concatenator.
4. Rebind the hook range on a child scion (same or another library clip).
   Leave `graph` unset so the look inherits. Call `compile` again. Assert
   the body `slot_encode` BlobId did not change and the resolved graph
   still has the look.
5. To change the look, write `child.graph` with a new `lut3d` material.
   Slots stay hits. Delivery misses.

Until that walk runs inside eonik, graft is unused by its intended host.
