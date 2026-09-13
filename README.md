English · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Español](README.es.md) · [Português (Brasil)](README.pt-BR.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Русский](README.ru.md)

# graft

<img src="docs/brand/mark.png" width="120" alt="Two clips. One join.">

A compiler for video composition.

[![CI](https://github.com/eonik-ai/graft/actions/workflows/ci.yml/badge.svg)](https://github.com/eonik-ai/graft/actions/workflows/ci.yml)
[![Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

The score is source. Essence is immutable. The mp4 is a compile.
You graft a new hook. The body stays.

git versions the **score** (JSON). CAS versions essence. The action cache
versions encodes, so changing a hook bitstream-copies the body. graft is
not an NLE. Runtime is **ffmpeg** and **ffprobe** on `PATH`.

[Apache-2.0](LICENSE) · [mission](docs/mission.md) · [principles](docs/principles.md) · [schema](schema/) · [nearby tools](docs/comparison.md)

## Install

graft shells out to ffmpeg; it does not link GPL x264. Rust 1.85+ (`rustup`).
`$FFMPEG` / `$FFPROBE` override the binaries on `PATH`.

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --help
```

GitHub Release binaries (when a `v0.2.*` tag is cut): macOS arm64 and Linux
x64. crates.io is not published yet (`publish = false`).

From a clone:

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
cargo run -- -C examples/hook-v3-body-v1-9x16 signal --kind hook_rate --t 0-3
```

The worked example is JSON only (placeholder hashes, no media in git).
`graft compile` there prints a **plan**. Your clips compile to mp4.

## Compile three clips (9x16)

```sh
mkdir ad && cd ad
graft init
graft slot body --span 3-20
graft slot cta --span 20-23 --role cta
graft scion 9x16 --dest 1080x1920 --encoder x264
graft bind hook ./hook.mov
graft bind body ./body.mov
graft bind cta ./cta.mov
graft compile --out ad.mp4
```

Play `ad.mp4`. Rebind the hook and compile again: `graft dirty` shows
`body` **hit**. The body encode is bitstream-copied. Dest is a linker
product, never essence.

```sh
graft signal --kind hook_rate --t 0-3
```

prints `{hook}` plus the hook→body kerf, not `body`.

Unsupported today: speed/retime, layer overlays, audio, NLE export.
`params.speed` changes the action key only.

## Status

| Piece | State |
| --- | --- |
| Mission, principles, ADRs | encoded |
| Score / scion / time-map schema | format id `0.1.0` |
| Signal → dirty-set (`hook_rate` does not dirty body) | `ref/` + Rust |
| Action graph + action cache | `graft-compile` / `graft-cas` |
| Frame-grain (`graft-intra`) | in-tree; dest is `GFI1`, not a player file |
| Long-GOP x264 mp4 | system ffmpeg; closed-GOP slot files; concat `-c copy` |
| NLE adapters / preview / S3 | not in this release |

Schema format id `0.1.0` is not a crate version. First compiler GitHub
tag is `v0.2.0` (do not reuse spec tag `v0.1.0`). Breaking schema changes
are an RFC.

## What graft is not

- Not git-on-pixels. Do not xdelta a delivery mp4.
- Not a lossless round-trip to every NLE. Adapters are guests; loss is documented.
- Not a lock server, a DAM, or a review tool.

Cousins (git, OTIO, IMF, ffmpeg concat) are in [docs/comparison.md](docs/comparison.md).

## Command surface

```text
graft init
graft slot hook --window 0-3
graft bind hook ./hooks/v3.mov
graft scion 9x16 --dest 1080x1920 --encoder x264
graft compile --out ad.mp4
graft dirty
graft signal --kind hook_rate --t 0-3
```

`export` is not implemented. `--encoder graft-intra` is the frame-grain
backend (tests / image-seq), not a QuickTime dest.

## Documentation

Implementer docs are English. [Translations of this README](docs/TRANSLATING.md).

| Doc | What it settles |
| --- | --- |
| [docs/mission.md](docs/mission.md) | Why graft exists |
| [docs/principles.md](docs/principles.md) | Non-negotiables and non-goals |
| [docs/glossary.md](docs/glossary.md) | score, slot, scion, kerf, dest |
| [docs/architecture.md](docs/architecture.md) | Layers, crate graph, compile pipeline |
| [docs/schema.md](docs/schema.md) | Commentary on the normative JSON Schema |
| [docs/compile.md](docs/compile.md) | Cache keys, grain, smart concat |
| [docs/time-map.md](docs/time-map.md) | How a metric addresses a slot |
| [docs/adapters.md](docs/adapters.md) | Loss matrix |
| [docs/comparison.md](docs/comparison.md) | git, OTIO, IMF, Vit, Aspect |
| [docs/roadmap.md](docs/roadmap.md) | Work still ahead |
| [docs/brand/](docs/brand/) | Mark: two clips, one join |
| [docs/adr/](docs/adr/) | Decisions already made |

Normative machine contract: [`schema/`](schema/).

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md). Schema changes need an RFC.
All commits require a Developer Certificate of Origin (`Signed-off-by`).
Be kind: [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

Copyright 2026 [eonik](https://www.eonik.ai/) ([github.com/eonik-ai](https://github.com/eonik-ai)).

Licensed under the [Apache License, Version 2.0](LICENSE).
The patent grant is why this project is Apache-2.0, not MIT.
Contributors are first-class: there is no CLA and no copyright assignment.
You keep copyright on your patches; DCO + Apache §5 license them inbound.
See [NOTICE](NOTICE) and [CONTRIBUTING.md](CONTRIBUTING.md).
