<div align="center">
  <a href="https://github.com/eonik-ai/graft">
    <img src="docs/brand/mark.png" alt="Dois clips. Uma emenda." width="120" />
  </a>
  <h1>graft</h1>
  <p><strong>Mude o hook. Mantenha o body.</strong></p>
  <p>Um workspace de composição local-first. O compilador incremental é o motor de build.</p>
  <p>
    <a href="#comecar">Começar</a> ·
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
    <a href="README.md">English</a> ·
    <a href="README.es.md">Español</a> ·
    <strong>Português</strong> ·
    <a href="README.fr.md">Français</a> ·
    <a href="README.zh-CN.md">简体中文</a> ·
    <a href="README.ja.md">日本語</a> ·
    <a href="README.ko.md">한국어</a> ·
    <a href="README.de.md">Deutsch</a> ·
    <a href="README.ru.md">Русский</a>
  </p>
</div>

---

O score é a fonte. A essence é imutável. O mp4 é uma compilação.

O loop fundador é concept → scions → iteração da equipe → build enviado →
signal → slot endereçado → novo scion. Git possui o histórico da receita;
graft possui a semântica de composição e a compilação; o CAS guarda o
material de origem imutável; o action cache guarda os encodes derivados.

Mude um hook e compile de novo: `graft` encodeia o hook, copia o body por
bitstream e liga um dest novo. Um signal de plataforma contra esse build
exato nomeia o hook sem recortar a árvore.

![graft CLI compilando, vinculando outro hook e mantendo body e cta limpos](docs/assets/landing.gif)

_Uma sessão local real, gravada com [asciinema](https://github.com/asciinema/asciinema).
A fonte da reprodução é [`landing.cast`](docs/assets/landing.cast)._

## O que o graft faz hoje

- Guardar um concept e muitos scions herdados como receitas rastreadas pelo Git
- Fazer bind por scion e layer; achatar as opiniões da layer mais forte
- Mostrar diff semântico e merge de três vias sem misturar mídia
- Compilar slots nomeados `hook`, `body`, `proof` e `cta` para um dest
- Cachear AAC sincronizado à parte e muxá-lo com o enlace de vídeo
- Aplicar `params.speed` gravado (setpts/atempo ou resample intra) sem
  sujar slots limpos adjacentes
- Preencher um kerf Long-GOP quando a junta não está alinhada a IDR;
  kerf vazio em juntas closed-GOP
- Re-encodear um hook alterado e copiar por bitstream o body intacto
- Resolver `hook_rate` pela janela declarada e o time map enviado
- Exportar/importar um subconjunto OTIO delimitado com relatório de perda
- Pré-visualizar um scion por decode e composite, com áudio sincronizado
- Sincronizar blobs para uma raiz object-store com descoberta de blobs faltantes

## O que ele não faz

graft não repara timelines mid-GOP arbitrárias, não inventa um speed a
partir de feedback nem faz ida e volta de efeitos, grades ou generators
de um NLE. Não é Git-sobre-pixels, um substituto do histórico do Git, um
servidor de lock, um DAM nem uma ferramenta de review.

O compilador sequencial, o grafo de ações, o CAS em filesystem, o
transporte object-store, o backend de grão de quadro e o caminho
ffmpeg/x264 do sistema estão na árvore.
O format id `0.2.0` do schema não é uma versão de crate.
A primeira tag GitHub do compilador/workspace é `v0.2.0`; não reutilize
a tag de spec `v0.1.0`.
Mudanças incompatíveis de schema passam por RFC.

## Começar

Runtime: **ffmpeg** e **ffprobe** no `PATH` (ou `$FFMPEG` / `$FFPROBE`).
graft chama o sistema; não vincula x264 GPL. Rust 1.85+ (`rustup`).

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --help
```

Binários de GitHub Release chegam ao cortar uma tag `v0.2.*`: macOS arm64
e Linux x64. crates.io ainda não está publicado (`publish = false`).

A partir de um clone:

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
```

Sua primeira compilação:

```sh
mkdir ad && cd ad
graft init
graft slot body --span 3-20
graft slot cta --span 20-23 --role cta
graft scion create 9x16 --dest 1080x1920 --encoder x264
graft bind hook ./hook.mov
graft bind body ./body.mov
graft bind cta ./cta.mov
graft compile --out ad.mp4
```

Reproduza `ad.mp4`. Faça fork de um scion de hook e bind de outra take:

```sh
graft scion fork 9x16 hook-v2
graft bind hook ./hook-v2.mov --scion hook-v2
graft diff 9x16 hook-v2
graft compile --scion hook-v2 --out ad-v2.mp4
graft dirty --scion hook-v2
```

`graft dirty` nomeia `hook` e o kerf hook→body. `body` e `cta` estão
limpos; os bytes encodeados são reutilizados.

Enderece uma métrica de plataforma pelo build enviado:

```sh
graft signal --kind hook_rate --build <build-id>
graft iterate --from <build-id> --feedback <id> --scion hook-v3
```

Isso suja `hook` mais o kerf hook→body, não `body`. `iterate` faz fork
de um pedido de mudança; não inventa o clip substituto.

O exemplo trabalhado é só JSON (hashes placeholder, sem mídia no git),
então o compile imprime um **plan**. Os seus clips compilam para mp4.

## Segurança

Bytes de material local e saídas de build ficam em `.graft/`, que é
ignorado. Não faça commit de essence (`.mov`, `.mp4`, `.mxf`) nem de
credenciais. Reporte vulnerabilidades em privado por [SECURITY.md](SECURITY.md).

## Também

| | |
|---|---|
| Por que o graft existe | [Mission](docs/mission.md) · [principles](docs/principles.md) |
| Contrato do compilador | [Architecture](docs/architecture.md) · [cache and kerfs](docs/compile.md) · [time map](docs/time-map.md) |
| Contrato de máquina | [JSON Schema](schema/) · [worked example](examples/hook-v3-body-v1-9x16/) |
| Limites | [git, OTIO, IMF, ffmpeg concat](docs/comparison.md) · [adapter loss matrix](docs/adapters.md) |
| Projeto | [Roadmap](docs/roadmap.md) · [contributing](CONTRIBUTING.md) · [translations](docs/TRANSLATING.md) |

## Licença

Copyright 2026 [eonik](https://www.eonik.ai/) ([github.com/eonik-ai](https://github.com/eonik-ai)).

Licenciado sob a [Apache License, Version 2.0](LICENSE).
A concessão de patentes é o motivo de ser Apache-2.0, não MIT.
Contribuidores são de primeira: não há CLA nem cessão de copyright.
Você guarda o copyright dos seus patches; DCO + Apache §5 os licenciam de entrada.
Veja [NOTICE](NOTICE) e [CONTRIBUTING.md](CONTRIBUTING.md).
