[English](README.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Español](README.es.md) · Português (Brasil) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Русский](README.ru.md)

<img src="docs/brand/mark.png" width="96" align="right" alt="Dois clips. Uma emenda.">

# graft

[![CI](https://github.com/eonik-ai/graft/actions/workflows/ci.yml/badge.svg)](https://github.com/eonik-ai/graft/actions/workflows/ci.yml)
[![Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

O score é a fonte. A essence é imutável. O mp4 é uma compilação.
Você enxerta um hook novo. O body permanece.

git versiona o **score** (JSON). CAS versiona a essence. O action cache
versiona os encodes, então mudar um hook copia o body por bitstream.
graft não é um NLE. O runtime é **ffmpeg** e **ffprobe** no `PATH`.

[Apache-2.0](LICENSE) · [mission](docs/mission.md) · [principles](docs/principles.md) · [schema](schema/) · [ferramentas próximas](docs/comparison.md)

![graft CLI compilando, vinculando outro hook e mantendo body e cta limpos](docs/assets/landing.gif)

_Uma sessão local real, gravada com [asciinema](https://github.com/asciinema/asciinema).
A fonte da reprodução é [`landing.cast`](docs/assets/landing.cast)._

## Começar

### Requisitos

graft chama o ffmpeg do sistema; não vincula x264 GPL. Rust 1.85+ (`rustup`).
`$FFMPEG` / `$FFPROBE` substituem os binários do `PATH`.

### Instalação

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --help
```

Binários de GitHub Release (quando uma tag `v0.2.*` for cortada): macOS arm64
e Linux x64. crates.io ainda não está publicado (`publish = false`).

A partir de um clone:

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
cargo run -- -C examples/hook-v3-body-v1-9x16 signal --kind hook_rate --t 0-3
```

O exemplo trabalhado é só JSON (hashes placeholder, sem mídia no git).
`graft compile` ali imprime um **plan**. Os seus clips compilam para mp4.

### Executar a primeira compilação

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

Reproduza `ad.mp4`. Faça bind de novo no hook e compile outra vez: `graft dirty`
mostra `body` **hit**. O encode do body é copiado por bitstream. Dest é produto
do linker, nunca essence.

```sh
graft signal --kind hook_rate --t 0-3
```

imprime `{hook}` mais o kerf hook→body, não `body`.

Sem suporte hoje: speed/retime, sobreposição de camadas, áudio, exportação NLE.
`params.speed` muda só a action key.

## Estado do projeto

| Peça | Estado |
| --- | --- |
| Mission, principles, ADR | escritos |
| Score / scion / time-map schema | format id `0.1.0` |
| Signal → dirty-set (`hook_rate` não suja body) | `ref/` + Rust |
| Action graph + action cache | `graft-compile` / `graft-cas` |
| Grão de quadro (`graft-intra`) | no repositório; dest é `GFI1`, não um arquivo de player |
| Long-GOP x264 mp4 | ffmpeg do sistema; arquivos de slot closed-GOP; concat `-c copy` |
| Adaptadores NLE / preview / S3 | não nesta versão |

O format id `0.1.0` do schema não é versão de crate. A primeira tag GitHub do
compilador é `v0.2.0` (não reutilize a tag de spec `v0.1.0`). Mudanças
incompatíveis de schema passam por RFC.

## O que graft não é

- Não é git sobre pixels. Não faça xdelta de um mp4 de entrega.
- Não é ida e volta lossless para todo NLE. Adaptadores são convidados; a perda está documentada.
- Não é servidor de lock, DAM nem ferramenta de review.

Os primos (git, OTIO, IMF, ffmpeg concat) estão em [docs/comparison.md](docs/comparison.md).

## Superfície de comandos

```text
graft init
graft slot hook --window 0-3
graft bind hook ./hooks/v3.mov
graft scion 9x16 --dest 1080x1920 --encoder x264
graft compile --out ad.mp4
graft dirty
graft signal --kind hook_rate --t 0-3
```

`export` não está implementado. `--encoder graft-intra` é o backend de grão de
quadro (testes / image-seq), não um dest QuickTime.

## Próximos passos

A documentação para implementadores está em inglês. [Traduções deste README](docs/TRANSLATING.md).

| Doc | O que resolve |
| --- | --- |
| [docs/mission.md](docs/mission.md) | Por que graft existe |
| [docs/principles.md](docs/principles.md) | Não negociáveis e não-objetivos |
| [docs/glossary.md](docs/glossary.md) | score, slot, scion, kerf, dest |
| [docs/architecture.md](docs/architecture.md) | Camadas, grafo de crates, pipeline de compile |
| [docs/schema.md](docs/schema.md) | Comentário sobre o JSON Schema normativo |
| [docs/compile.md](docs/compile.md) | Chaves de cache, grain, smart concat |
| [docs/time-map.md](docs/time-map.md) | Como uma métrica aponta para um slot |
| [docs/adapters.md](docs/adapters.md) | Matriz de perda |
| [docs/comparison.md](docs/comparison.md) | git, OTIO, IMF, Vit, Aspect |
| [docs/roadmap.md](docs/roadmap.md) | Trabalho ainda à frente |
| [docs/brand/](docs/brand/) | Marca: dois clips, uma emenda |
| [docs/adr/](docs/adr/) | Decisões já tomadas |

Contrato normativo de máquina: [`schema/`](schema/).

## Contribuindo

Leia [CONTRIBUTING.md](CONTRIBUTING.md). Mudanças de schema precisam de RFC.
Todo commit exige Developer Certificate of Origin (`Signed-off-by`).
Seja gentil: [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Licença

Copyright 2026 [eonik](https://www.eonik.ai/) ([github.com/eonik-ai](https://github.com/eonik-ai)).

Licenciado sob a [Apache License, Version 2.0](LICENSE).
A concessão de patentes é o motivo de Apache-2.0, não MIT.
Contribuidores são de primeira: não há CLA nem cessão de copyright.
Você conserva o copyright dos seus patches; DCO e Apache §5 os licenciam para dentro.
Veja [NOTICE](NOTICE) e [CONTRIBUTING.md](CONTRIBUTING.md).
