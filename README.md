<div align="center">
  <a href="https://github.com/eonik-ai/graft">
    <img src="docs/brand/mark.png" alt="Two clips. One join." width="120" />
  </a>
  <h1>graft</h1>
  <p><strong>Change the hook. Keep the body.</strong></p>
  <p>A local-first composition workspace. The incremental compiler is its build engine.</p>
  <p>
    <a href="#get-started">Get started</a> ·
    <a href="docs/mission.md">Mission</a> ·
    <a href="schema/">Schema</a> ·
    <a href="docs/roadmap.md">Roadmap</a>
  </p>
  <p>
    <a href="https://github.com/eonik-ai/graft/actions/workflows/ci.yml"><img src="https://github.com/eonik-ai/graft/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-18181B?style=flat-square" alt="Apache-2.0 license" /></a>
    <img src="https://img.shields.io/badge/Rust-1.85%2B-B7410E?style=flat-square" alt="Rust 1.85+" />
    <img src="https://img.shields.io/badge/runtime-ffmpeg-007808?style=flat-square" alt="ffmpeg runtime" />
  </p>
  <p>
    <strong>English</strong> ·
    <a href="README.es.md">Español</a> ·
    <a href="README.pt-BR.md">Português</a> ·
    <a href="README.fr.md">Français</a> ·
    <a href="README.zh-CN.md">简体中文</a> ·
    <a href="README.ja.md">日本語</a> ·
    <a href="README.ko.md">한국어</a> ·
    <a href="README.de.md">Deutsch</a> ·
    <a href="README.ru.md">Русский</a>
  </p>
</div>

---

The score is source. Essence is immutable. The mp4 is a compile.

Ship the next cut without recutting the tree. The founding loop is
concept → scions → team iteration → shipped build → signal → addressed
slot → new scion. Git owns recipe history; graft owns composition
semantics and compilation; CAS stores immutable source material; the
action cache keeps derived encodes.

Drop a new hook and compile again: `graft swap` encodes the hook,
bitstream-copies the body, and links a new dest. A platform signal
against that exact build names the hook without recutting the tree.

![graft CLI shipping a cut, swapping a hook, and keeping body and cta clean](docs/assets/landing.gif)

_A real local session, recorded with [asciinema](https://github.com/asciinema/asciinema).
The replay source is [`landing.cast`](docs/assets/landing.cast)._

![Takes become dest v1; swap the hook and dest v2 keeps the body](docs/assets/walkthrough.gif)

_Input plates on the left dest, swapped hook on the right. Generate it with
`make walkthrough`. The mp4 is gitignored; this GIF is the still._

## What graft can do today

- Ship a first dest from a named `takes/` folder (`graft ship`)
- Swap one slot onto a new take, or a folder of takes, without recoding
  clean siblings (`graft swap`); fail if a sibling recodes
- Address a named signal to the slot that must change (`graft address`)
- Print a reuse ledger on every encode: dirty slots, clean BlobIds, time
- Keep one concept and many inherited scions as Git-tracked recipes
- Bind by scion and layer; flatten strongest-layer opinions
- Show semantic diff and three-way merge without merging media
- Compile named `hook`, `body`, `proof`, and `cta` slots to a dest
- Compile `vo` / `bed` as mixed audio, `captions` as a sidecar, and
  `brand` as an overlay on the picture concat
- Name a `fade` join on the score without rewriting adjacent slot encodes
- Cache synchronized AAC independently and mux it with the video link
- Apply recorded `params.speed` (setpts/atempo or intra resample) without
  dirtying adjacent clean slots
- Fill a Long-GOP kerf when a join is not IDR-aligned; keep an empty kerf
  at closed-GOP file joins
- Re-encode a changed hook while bitstream-copying the unchanged body
- Resolve `hook_rate` through the declared window and the shipped time map
- Export/import a scoped OTIO subset with an explicit loss report
- Preview a selected scion by decode and composite, with synced audio
  and overlay mix
- Sync blobs to a filesystem, `https://`, or `s3://` object-store root
  with missing-blob discovery. There is no remote compile.

## What it cannot do

graft does not repair arbitrary mid-GOP timelines, guess a speed from
feedback, or round-trip NLE effects, grades, or generators. It is not
Git-on-pixels, a replacement for Git history, a lock server, a DAM, or a
review tool.

The sequential compiler, action graph, filesystem CAS, object-store
transport, frame-grain backend, and system ffmpeg/x264 path are in-tree.
Schema format id `0.2.0` is not a crate version.
The first compiler GitHub tag is `v0.2.0`; do not reuse the spec tag `v0.1.0`.
Breaking schema changes need an RFC.

## Get started

Runtime: **ffmpeg** and **ffprobe** on `PATH` (or `$FFMPEG` / `$FFPROBE`).
graft shells out; it does not link GPL x264.

**macOS (Homebrew)** — this repo is the tap:

```sh
brew trust --tap eonik-ai/graft
brew tap eonik-ai/graft https://github.com/eonik-ai/graft
brew install graft
```

**Linux / macOS binary:**

```sh
curl -fsSL https://raw.githubusercontent.com/eonik-ai/graft/main/scripts/install.sh | sh
```

GitHub Releases ship macOS arm64 and Intel, Linux gnu (amd64 and arm64),
Linux musl (amd64), and `.deb` packages. Nix: `nix run github:eonik-ai/graft`.
Packager recipes for AUR, Alpine, Fedora, Debian, MacPorts, and the rest of
the [pkg.bot](https://pkg.bot/repos) families live in [`dist/`](dist/README.md)
and are **not listed until those distros accept them**.

From source (Rust 1.85+, `rustup`):

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --version
```

crates.io crate name `graft` is taken by another project. Do not
`cargo install graft` from crates.io. Details: [docs/install.md](docs/install.md).

From a clone:

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
```

Your first compile:

```
takes/
  hook.mov
  body.mov
  cta.mov
```

```sh
mkdir ad && cd ad
# put the three files in takes/
graft ship --out ad.mp4
```

`graft ship` fails if it cannot encode a dest (ffmpeg missing, or no real
takes). `graft compile` on JSON examples stays plan-only.

`.graft/` is local CAS and builds; do not commit it. Recipe history is Git.
See [docs/history.md](docs/history.md). The user loop is [docs/loop.md](docs/loop.md).

Play `ad.mp4`. Drop a different hook:

```sh
graft swap hook ./takes/hooks/v2.mov --out ad-v2.mp4
```

The reuse ledger names `hook` dirty and `body` / `cta` clean; their encoded
bytes are reused. `swap` exits non-zero if a sibling recoded.

Address a platform metric through the shipped build:

```sh
graft address --kind hook_rate --build <build-id>
graft swap hook ./takes/hooks/v3.mov --out ad-v3.mp4
```

`address` dirties `hook` plus the hook→body kerf, not `body`. It forks a
change request; it does not invent the replacement clip.

Batch a take pool against the same body:

```sh
graft swap hook --from ./takes/hooks --out-dir ./out
```

### Plumbing

The same walk as explicit verbs (porcelain calls these):

```sh
graft init
graft slot body --span 3-20
graft slot cta --span 20-23 --role cta
graft scion create 9x16 --dest 1080x1920 --encoder x264
graft bind hook ./hook.mov
graft bind body ./body.mov
graft bind cta ./cta.mov
graft compile --out ad.mp4
git add score.json scions
git commit -m "picture scion"
```

```sh
graft scion fork 9x16 hook-v2
graft bind hook ./hook-v2.mov --scion hook-v2
graft diff 9x16 hook-v2
graft compile --scion hook-v2 --out ad-v2.mp4
graft dirty --scion hook-v2
```

```sh
graft signal --kind hook_rate --build <build-id>
graft iterate --from <build-id> --feedback <id> --scion hook-v3
```

The worked example is JSON only (placeholder hashes, no media in git), so its
compile prints a **plan**. Your own clips compile to mp4. `make demo` walks
ship → swap → address → swap. `make walkthrough` writes the side-by-side GIF.

## Security

Local material bytes and build outputs live under `.graft/`, which is ignored.
Do not commit essence (`.mov`, `.mp4`, `.mxf`) or credentials. Report
vulnerabilities privately through [SECURITY.md](SECURITY.md).

## Also

| | |
|---|---|
| Why graft exists | [Mission](docs/mission.md) · [principles](docs/principles.md) · [loop](docs/loop.md) · [history](docs/history.md) |
| Install | [install](docs/install.md) · [packaging](docs/packaging.md) · [submissions](docs/SUBMISSIONS.md) · [walkthrough](docs/walkthrough.md) |
| Compiler contract | [Architecture](docs/architecture.md) · [cache and kerfs](docs/compile.md) · [time map](docs/time-map.md) |
| Machine contract | [JSON Schema](schema/) · [worked example](examples/hook-v3-body-v1-9x16/) |
| Boundaries | [git, OTIO, IMF, ffmpeg concat](docs/comparison.md) · [adapter loss matrix](docs/adapters.md) |
| Project | [Roadmap](docs/roadmap.md) · [contributing](CONTRIBUTING.md) · [translations](docs/TRANSLATING.md) |

## License

Copyright 2026 [eonik](https://www.eonik.ai/) ([github.com/eonik-ai](https://github.com/eonik-ai)).

Licensed under the [Apache License, Version 2.0](LICENSE).
The patent grant is why this project is Apache-2.0, not MIT.
Contributors are first-class: there is no CLA and no copyright assignment.
You keep copyright on your patches; DCO + Apache §5 license them inbound.
See [NOTICE](NOTICE) and [CONTRIBUTING.md](CONTRIBUTING.md).
