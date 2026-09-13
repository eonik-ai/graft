[English](README.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Español](README.es.md) · [Português (Brasil)](README.pt-BR.md) · [Français](README.fr.md) · Deutsch · [Русский](README.ru.md)

# graft

<img src="docs/brand/mark.png" width="120" alt="Zwei Clips. Eine Fuge.">

Ein Compiler für Videokomposition.

[![CI](https://github.com/eonik-ai/graft/actions/workflows/ci.yml/badge.svg)](https://github.com/eonik-ai/graft/actions/workflows/ci.yml)
[![Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

Der score ist Quelle. Essence ist unveränderlich. Die mp4 ist ein Compile-Ergebnis.
Du pfropfst einen neuen hook. Der body bleibt.

git versioniert den **score** (JSON). CAS versioniert essence. Der action cache
versioniert Encodes, deshalb bitstream-kopiert ein neuer hook den body.
graft ist kein NLE. Laufzeit sind **ffmpeg** und **ffprobe** auf `PATH`.

[Apache-2.0](LICENSE) · [mission](docs/mission.md) · [principles](docs/principles.md) · [schema](schema/) · [nahe Werkzeuge](docs/comparison.md)

## Installation

graft ruft das System-ffmpeg auf; es linkt kein GPL-x264. Rust 1.85+ (`rustup`).
`$FFMPEG` / `$FFPROBE` überschreiben die Binaries auf `PATH`.

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --help
```

GitHub-Release-Binaries (sobald ein Tag `v0.2.*` geschnitten ist): macOS arm64
und Linux x64. crates.io ist noch nicht veröffentlicht (`publish = false`).

Aus einem Clone:

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
cargo run -- -C examples/hook-v3-body-v1-9x16 signal --kind hook_rate --t 0-3
```

Das ausgearbeitete Beispiel ist nur JSON (Platzhalter-Hashes, keine Medien in git).
`graft compile` dort druckt einen **plan**. Deine Clips compilieren zu mp4.

## Drei Clips compilieren (9x16)

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

Spiele `ad.mp4`. Binde den hook neu und compile erneut: `graft dirty` zeigt
`body` **hit**. Der body-Encode wird bitstream-kopiert. Dest ist ein
Linker-Produkt, niemals essence.

```sh
graft signal --kind hook_rate --t 0-3
```

druckt `{hook}` plus den hook→body-kerf, nicht `body`.

Heute nicht unterstützt: speed/retime, Ebenen-Overlay, Audio, NLE-Export.
`params.speed` ändert nur den action key.

## Stand

| Teil | Stand |
| --- | --- |
| Mission, principles, ADR | geschrieben |
| Score / scion / time-map schema | format id `0.1.0` |
| Signal → dirty-set (`hook_rate` macht body nicht dirty) | `ref/` + Rust |
| Action graph + action cache | `graft-compile` / `graft-cas` |
| Frame-Korn (`graft-intra`) | im Baum; dest ist `GFI1`, keine Player-Datei |
| Long-GOP x264 mp4 | System-ffmpeg; closed-GOP-Slotdateien; concat `-c copy` |
| NLE-Adapter / Preview / S3 | nicht in dieser Version |

Die schema-format id `0.1.0` ist keine Crate-Version. Der erste Compiler-GitHub-Tag
ist `v0.2.0` (den Spec-Tag `v0.1.0` nicht wiederverwenden).
Unverträgliche Schema-Änderungen gehen über RFC.

## Was graft nicht ist

- Kein Git auf Pixeln. Kein xdelta auf einer Liefer-mp4.
- Kein verlustfreier Roundtrip in jedes NLE. Adapter sind Gäste; der Verlust ist dokumentiert.
- Kein Lock-Server, kein DAM, kein Review-Werkzeug.

Verwandte (git, OTIO, IMF, ffmpeg concat) stehen in [docs/comparison.md](docs/comparison.md).

## Befehlsfläche

```text
graft init
graft slot hook --window 0-3
graft bind hook ./hooks/v3.mov
graft scion 9x16 --dest 1080x1920 --encoder x264
graft compile --out ad.mp4
graft dirty
graft signal --kind hook_rate --t 0-3
```

`export` ist nicht implementiert. `--encoder graft-intra` ist das Frame-Korn-Backend
(Tests / image-seq), kein QuickTime-dest.

## Dokumentation

Implementierer-Dokumentation ist Englisch. [Übersetzungen dieser README](docs/TRANSLATING.md).

| Doc | Was sie klärt |
| --- | --- |
| [docs/mission.md](docs/mission.md) | Warum graft existiert |
| [docs/principles.md](docs/principles.md) | Nichtverhandelbares und Nicht-Ziele |
| [docs/glossary.md](docs/glossary.md) | score, slot, scion, kerf, dest |
| [docs/architecture.md](docs/architecture.md) | Schichten, Crate-Graph, Compile-Pipeline |
| [docs/schema.md](docs/schema.md) | Kommentar zum normativen JSON Schema |
| [docs/compile.md](docs/compile.md) | Cache-Schlüssel, grain, smart concat |
| [docs/time-map.md](docs/time-map.md) | Wie eine Metrik einen slot adressiert |
| [docs/adapters.md](docs/adapters.md) | Verlustmatrix |
| [docs/comparison.md](docs/comparison.md) | git, OTIO, IMF, Vit, Aspect |
| [docs/roadmap.md](docs/roadmap.md) | Arbeit, die noch aussteht |
| [docs/brand/](docs/brand/) | Marke: zwei Clips, eine Fuge |
| [docs/adr/](docs/adr/) | Bereits getroffene Entscheidungen |

Normativer Maschinenvertrag: [`schema/`](schema/).

## Mitwirken

Bitte [CONTRIBUTING.md](CONTRIBUTING.md) lesen. Schema-Änderungen brauchen ein RFC.
Jeder Commit braucht ein Developer Certificate of Origin (`Signed-off-by`).
Sei anständig: [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Lizenz

Copyright 2026 [eonik](https://www.eonik.ai/) ([github.com/eonik-ai](https://github.com/eonik-ai)).

Lizenziert unter der [Apache License, Version 2.0](LICENSE).
Die Patentlizenz ist der Grund für Apache-2.0, nicht MIT.
Beitragende sind erste Klasse: kein CLA, keine Copyright-Abtretung.
Du behältst das Copyright deiner Patches; DCO und Apache §5 lizenzieren sie herein.
Siehe [NOTICE](NOTICE) und [CONTRIBUTING.md](CONTRIBUTING.md).
