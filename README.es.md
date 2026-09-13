[English](README.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · Español · [Português (Brasil)](README.pt-BR.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Русский](README.ru.md)

# graft

<img src="docs/brand/mark.png" width="120" alt="Dos clips. Un empalme.">

Un compilador para composición de vídeo.

[![CI](https://github.com/eonik-ai/graft/actions/workflows/ci.yml/badge.svg)](https://github.com/eonik-ai/graft/actions/workflows/ci.yml)
[![Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

El score es la fuente. La essence es inmutable. El mp4 es una compilación.
Injertas un hook nuevo. El body permanece.

git versiona el **score** (JSON). CAS versiona la essence. El action cache
versiona los encodes, así que cambiar un hook copia el body por bitstream.
graft no es un NLE. El runtime es **ffmpeg** y **ffprobe** en `PATH`.

[Apache-2.0](LICENSE) · [mission](docs/mission.md) · [principles](docs/principles.md) · [schema](schema/) · [herramientas cercanas](docs/comparison.md)

## Instalación

graft invoca el ffmpeg del sistema; no enlaza x264 GPL. Rust 1.85+ (`rustup`).
`$FFMPEG` / `$FFPROBE` sustituyen los binarios de `PATH`.

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --help
```

Binarios de GitHub Release (cuando se corte una etiqueta `v0.2.*`): macOS arm64
y Linux x64. crates.io aún no está publicado (`publish = false`).

Desde un clon:

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
cargo run -- -C examples/hook-v3-body-v1-9x16 signal --kind hook_rate --t 0-3
```

El ejemplo trabajado es solo JSON (hashes de relleno, sin media en git).
`graft compile` ahí imprime un **plan**. Tus clips sí compilan a mp4.

## Compilar tres clips (9x16)

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

Reproduce `ad.mp4`. Vuelve a hacer bind del hook y compila otra vez: `graft dirty`
muestra `body` **hit**. El encode del body se copia por bitstream. Dest es un
producto del linker, nunca essence.

```sh
graft signal --kind hook_rate --t 0-3
```

imprime `{hook}` más el kerf hook→body, no `body`.

Sin soporte hoy: speed/retime, superposición de capas, audio, exportación a NLE.
`params.speed` solo cambia la action key.

## Estado

| Pieza | Estado |
| --- | --- |
| Mission, principles, ADR | escritos |
| Score / scion / time-map schema | format id `0.1.0` |
| Signal → dirty-set (`hook_rate` no ensucia body) | `ref/` + Rust |
| Action graph + action cache | `graft-compile` / `graft-cas` |
| Grano de fotograma (`graft-intra`) | en el árbol; dest es `GFI1`, no un archivo de reproductor |
| Long-GOP x264 mp4 | ffmpeg del sistema; archivos de slot closed-GOP; concat `-c copy` |
| Adaptadores NLE / preview / S3 | no en esta versión |

El format id `0.1.0` del schema no es una versión de crate. La primera etiqueta
GitHub del compilador es `v0.2.0` (no reutilices la etiqueta de spec `v0.1.0`).
Cambios incompatibles de schema van por RFC.

## Qué no es graft

- No es git sobre píxeles. No hagas xdelta de un mp4 de entrega.
- No es un ida y vuelta lossless a cada NLE. Los adaptadores son invitados; la pérdida está documentada.
- No es un servidor de bloqueo, un DAM ni una herramienta de revisión.

Los primos (git, OTIO, IMF, ffmpeg concat) están en [docs/comparison.md](docs/comparison.md).

## Superficie de comandos

```text
graft init
graft slot hook --window 0-3
graft bind hook ./hooks/v3.mov
graft scion 9x16 --dest 1080x1920 --encoder x264
graft compile --out ad.mp4
graft dirty
graft signal --kind hook_rate --t 0-3
```

`export` no está implementado. `--encoder graft-intra` es el backend de grano de
fotograma (tests / image-seq), no un dest QuickTime.

## Documentación

La documentación para implementadores está en inglés. [Traducciones de este README](docs/TRANSLATING.md).

| Doc | Qué zanja |
| --- | --- |
| [docs/mission.md](docs/mission.md) | Por qué existe graft |
| [docs/principles.md](docs/principles.md) | No negociables y no-objetivos |
| [docs/glossary.md](docs/glossary.md) | score, slot, scion, kerf, dest |
| [docs/architecture.md](docs/architecture.md) | Capas, grafo de crates, pipeline de compile |
| [docs/schema.md](docs/schema.md) | Comentario sobre el JSON Schema normativo |
| [docs/compile.md](docs/compile.md) | Claves de caché, grain, smart concat |
| [docs/time-map.md](docs/time-map.md) | Cómo una métrica apunta a un slot |
| [docs/adapters.md](docs/adapters.md) | Matriz de pérdida |
| [docs/comparison.md](docs/comparison.md) | git, OTIO, IMF, Vit, Aspect |
| [docs/roadmap.md](docs/roadmap.md) | Trabajo que falta |
| [docs/brand/](docs/brand/) | Marca: dos clips, un empalme |
| [docs/adr/](docs/adr/) | Decisiones ya tomadas |

Contrato normativo de máquina: [`schema/`](schema/).

## Contribuir

Lee [CONTRIBUTING.md](CONTRIBUTING.md). Los cambios de schema necesitan un RFC.
Todos los commits requieren Developer Certificate of Origin (`Signed-off-by`).
Sé amable: [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Licencia

Copyright 2026 [eonik](https://www.eonik.ai/) ([github.com/eonik-ai](https://github.com/eonik-ai)).

Licenciado bajo la [Apache License, Version 2.0](LICENSE).
La concesión de patentes es la razón de Apache-2.0 y no MIT.
Los contribuyentes son de primera: no hay CLA ni cesión de copyright.
Conservas el copyright de tus parches; DCO y Apache §5 los licencian hacia adentro.
Véase [NOTICE](NOTICE) y [CONTRIBUTING.md](CONTRIBUTING.md).
