<div align="center">
  <a href="https://github.com/eonik-ai/graft">
    <img src="docs/brand/mark.png" alt="Dos clips. Un empalme." width="120" />
  </a>
  <h1>graft</h1>
  <p><strong>Cambia el hook. Conserva el body.</strong></p>
  <p>Un workspace de composición local-first. El compilador incremental es su motor de build.</p>
  <p>
    <a href="#empezar">Empezar</a> ·
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
    <strong>Español</strong> ·
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

El score es la fuente. La essence es inmutable. El mp4 es una compilación.

El bucle fundacional es concept → scions → iteración del equipo → build
enviado → signal → slot dirigido → nuevo scion. Git posee el historial de
la receta; graft posee la semántica de composición y la compilación; CAS
guarda el material de origen inmutable; el action cache conserva los encodes
derivados.

Cambia un hook y vuelve a compilar: `graft` encodea el hook, copia el body
por bitstream y enlaza un dest nuevo. Un signal de plataforma contra ese
build exacto nombra el hook sin recortar el árbol.

![graft CLI compilando, vinculando otro hook y manteniendo body y cta limpios](docs/assets/landing.gif)

_Una sesión local real, grabada con [asciinema](https://github.com/asciinema/asciinema).
La fuente de reproducción es [`landing.cast`](docs/assets/landing.cast)._

## Qué puede hacer graft hoy

- Guardar un concept y muchos scions heredados como recetas rastreadas por Git
- Hacer bind por scion y layer; aplanar las opiniones de la layer más fuerte
- Mostrar diff semántico y merge a tres bandas sin mezclar media
- Compilar slots con nombre `hook`, `body`, `proof` y `cta` a un dest
- Compilar `vo` / `bed` como audio mezclado, `captions` como sidecar y
  `brand` como overlay sobre el concat de imagen
- Nombrar un join `fade` en el score sin reescribir los slot_encode adyacentes
- Cachear AAC sincronizado por separado y muxearlo con el enlace de video
- Aplicar `params.speed` registrado (setpts/atempo o resample intra) sin
  ensuciar slots limpios adyacentes
- Rellenar un kerf Long-GOP cuando el empalme no está alineado a IDR;
  kerf vacío en empalmes closed-GOP
- Re-encodear un hook cambiado y copiar por bitstream el body intacto
- Resolver `hook_rate` con la ventana declarada y el time map enviado
- Exportar/importar un subconjunto OTIO acotado con un informe de pérdida
- Previsualizar un scion por decode y composite, con audio sincronizado
  y mezcla de overlay
- Sincronizar blobs a una ruta, `https://` o `s3://` con descubrimiento de
  blobs faltantes. No hay compile remoto.

## Qué no puede hacer

graft no repara timelines mid-GOP arbitrarios, no inventa un speed desde
feedback ni hace ida y vuelta de efectos, grades o generators de un NLE.
No es Git-sobre-píxeles, un reemplazo del historial de Git, un servidor
de bloqueo, un DAM ni una herramienta de revisión.

El compilador secuencial, el grafo de acciones, el CAS en filesystem, el
transporte object-store, el backend de grano de fotograma y la ruta
ffmpeg/x264 del sistema están en el árbol.
El format id `0.2.0` del schema no es una versión de crate.
La primera etiqueta GitHub del compilador/workspace es `v0.2.0`; no
reutilices la etiqueta de spec `v0.1.0`.
Los cambios incompatibles de schema van por RFC.

## Empezar

Runtime: **ffmpeg** y **ffprobe** en `PATH` (o `$FFMPEG` / `$FFPROBE`).
graft invoca el sistema; no enlaza x264 GPL. Rust 1.85+ (`rustup`).

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --help
```

Los binarios de GitHub Release llegan al cortar una etiqueta `v0.2.*`:
macOS arm64 y Linux x64. crates.io aún no está publicado (`publish = false`).

Desde un clon:

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
```

Tu primera compilación:

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

Reproduce `ad.mp4`. Bifurca un scion de hook y haz bind de otra toma:

```sh
graft scion fork 9x16 hook-v2
graft bind hook ./hook-v2.mov --scion hook-v2
graft diff 9x16 hook-v2
graft compile --scion hook-v2 --out ad-v2.mp4
graft dirty --scion hook-v2
```

`graft dirty` nombra `hook` y su kerf hook→body. `body` y `cta` están
limpios; se reutilizan sus bytes encodeados.

Dirige una métrica de plataforma a través del build enviado:

```sh
graft signal --kind hook_rate --build <build-id>
graft iterate --from <build-id> --feedback <id> --scion hook-v3
```

Eso ensucia `hook` más el kerf hook→body, no `body`. `iterate` bifurca
una petición de cambio; no inventa el clip de reemplazo.

El ejemplo trabajado es solo JSON (hashes de relleno, sin media en git),
así que su compile imprime un **plan**. Tus clips sí compilan a mp4.

## Seguridad

Los bytes de material local y las salidas de build viven bajo `.graft/`,
que está ignorado. No subas essence (`.mov`, `.mp4`, `.mxf`) ni credenciales.
Reporta vulnerabilidades en privado por [SECURITY.md](SECURITY.md).

## También

| | |
|---|---|
| Por qué existe graft | [Mission](docs/mission.md) · [principles](docs/principles.md) |
| Contrato del compilador | [Architecture](docs/architecture.md) · [cache and kerfs](docs/compile.md) · [time map](docs/time-map.md) |
| Contrato de máquina | [JSON Schema](schema/) · [worked example](examples/hook-v3-body-v1-9x16/) |
| Límites | [git, OTIO, IMF, ffmpeg concat](docs/comparison.md) · [adapter loss matrix](docs/adapters.md) |
| Proyecto | [Roadmap](docs/roadmap.md) · [contributing](CONTRIBUTING.md) · [translations](docs/TRANSLATING.md) |

## Licencia

Copyright 2026 [eonik](https://www.eonik.ai/) ([github.com/eonik-ai](https://github.com/eonik-ai)).

Licenciado bajo la [Apache License, Version 2.0](LICENSE).
La concesión de patentes es la razón de Apache-2.0 y no MIT.
Los contribuyentes son de primera: no hay CLA ni cesión de copyright.
Conservas el copyright de tus parches; DCO y Apache §5 los licencian hacia adentro.
Véase [NOTICE](NOTICE) y [CONTRIBUTING.md](CONTRIBUTING.md).
