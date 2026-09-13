English · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Español](README.es.md) · [Português (Brasil)](README.pt-BR.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Русский](README.ru.md)

<img src="docs/brand/mark.png" width="96" align="right" alt="Two clips. One join.">

# graft

[![CI](https://github.com/eonik-ai/graft/actions/workflows/ci.yml/badge.svg)](https://github.com/eonik-ai/graft/actions/workflows/ci.yml)
[![Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

`graft` is an incremental compiler for video composition. A JSON score names
the slots; a content-addressed store holds the essence; an action cache keeps
every clean encode.

Change a hook and compile again. `graft` encodes the hook, recuts its join,
bitstream-copies the body, and links a new mp4. The score is source. Essence
is immutable. The mp4 is a compile.

[Apache-2.0](LICENSE) · [mission](docs/mission.md) · [principles](docs/principles.md) · [schema](schema/) · [nearby tools](docs/comparison.md)

![graft CLI compiling, rebinding a hook, and keeping body and cta clean](docs/assets/landing.gif)

_A real local session, recorded with [asciinema](https://github.com/asciinema/asciinema).
The replay source is [`landing.cast`](docs/assets/landing.cast)._

## Get started

### Requirements

graft shells out to ffmpeg; it does not link GPL x264. Rust 1.85+ (`rustup`).
`$FFMPEG` / `$FFPROBE` override the binaries on `PATH`.

### Install

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --help
```

GitHub Release binaries (when a `v0.2.*` tag is cut): macOS arm64 and Linux
x64. crates.io is not published yet (`publish = false`).

To build from a clone:

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
cargo run -- -C examples/hook-v3-body-v1-9x16 signal --kind hook_rate --t 0-3
```

The worked example is JSON only (placeholder hashes, no media in git), so its
compile prints a **plan**. Your own clips compile to mp4.

### Run your first compile

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

Play `ad.mp4`. Now bind a different hook:

```sh
graft bind hook ./hook-v2.mov
graft dirty
graft compile --out ad-v2.mp4
```

`graft dirty` names `hook` and its hook→body kerf. `body` and `cta` are clean;
their encoded bytes are reused.

## How it works

- **Recipe:** git versions the JSON score and scions.
- **Essence:** CAS stores immutable source material by BLAKE3 hash.
- **Build:** the action cache stores slot encodes and kerfs; concat links the dest.

9:16 is a **dest**, not a slot. For Long-GOP x264, graft makes closed-GOP
slot files and joins them with ffmpeg `concat -c copy`. A platform signal can
address a time range:

```sh
graft signal --kind hook_rate --t 0-3
```

That dirties `hook` plus the hook→body kerf, not `body`.

## Project status

The compiler, action graph, filesystem CAS, frame-grain backend, and system
ffmpeg/x264 path are in-tree. Speed/retime, layer overlays, audio, NLE export,
preview, and S3 are not in this release.

Schema format id `0.1.0` is not a crate version. The first compiler GitHub tag
is `v0.2.0`; do not reuse the spec tag `v0.1.0`. Breaking schema changes need
an RFC.

graft is not git-on-pixels, a lossless round-trip to every NLE, a lock server,
a DAM, or a review tool. Its [comparison with git, OTIO, IMF, and ffmpeg
concat](docs/comparison.md) is explicit about those boundaries.

## Next steps

- Start with the [mission](docs/mission.md) and [non-negotiable principles](docs/principles.md).
- Read the [compiler architecture](docs/architecture.md) and [cache/kerf model](docs/compile.md).
- Browse the normative [JSON Schema](schema/) and the [worked example](examples/hook-v3-body-v1-9x16/).
- See the [roadmap](docs/roadmap.md), [adapter loss matrix](docs/adapters.md), or [README translations](docs/TRANSLATING.md).

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
