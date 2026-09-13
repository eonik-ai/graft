<div align="center">
  <a href="https://github.com/eonik-ai/graft"><img src="docs/brand/mark.png" alt="Deux clips. Une jointure." width="120" /></a>
  <h1>graft</h1>
  <p><strong>Changez le hook. Gardez le body.</strong></p>
  <p>
    <a href="README.md">English</a> ·
    <a href="README.es.md">Español</a> ·
    <a href="README.pt-BR.md">Português</a> ·
    <strong>Français</strong> ·
    <a href="README.zh-CN.md">简体中文</a> ·
    <a href="README.ja.md">日本語</a> ·
    <a href="README.ko.md">한국어</a> ·
    <a href="README.de.md">Deutsch</a> ·
    <a href="README.ru.md">Русский</a>
  </p>
</div>

---

Le score est la source. L’essence est immuable. Le mp4 est une compilation.
Vous greffez un nouveau hook. Le body reste.

git versionne le **score** (JSON). CAS versionne l’essence. L’action cache
versionne les encodes, donc changer un hook copie le body en bitstream.
graft n’est pas un NLE. Le runtime, c’est **ffmpeg** et **ffprobe** sur `PATH`.

![graft CLI compile, relie un autre hook et garde body et cta propres](docs/assets/landing.gif)

_Une vraie session locale, enregistrée avec [asciinema](https://github.com/asciinema/asciinema).
La source de lecture est [`landing.cast`](docs/assets/landing.cast)._

## Bien démarrer

### Prérequis

graft appelle le ffmpeg du système ; il ne lie pas x264 GPL. Rust 1.85+ (`rustup`).
`$FFMPEG` / `$FFPROBE` remplacent les binaires de `PATH`.

### Installation

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --help
```

Binaires GitHub Release (quand une étiquette `v0.2.*` sera coupée) : macOS arm64
et Linux x64. crates.io n’est pas encore publié (`publish = false`).

Depuis un clone :

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
cargo run -- -C examples/hook-v3-body-v1-9x16 signal --kind hook_rate --t 0-3
```

L’exemple travaillé n’est que du JSON (hachages factices, pas de média dans git).
`graft compile` y imprime un **plan**. Vos clips, eux, compilent en mp4.

### Lancer la première compilation

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

Ouvrez `ad.mp4`. Rebind le hook et compilez à nouveau : `graft dirty` affiche
`body` **hit**. L’encode du body est copié en bitstream. Dest est un produit
du linker, jamais de l’essence.

```sh
graft signal --kind hook_rate --t 0-3
```

affiche `{hook}` plus le kerf hook→body, pas `body`.

Non pris en charge aujourd’hui : speed/retime, superposition de calques, audio,
export NLE. `params.speed` ne change que l’action key.

## État du projet

| Pièce | État |
| --- | --- |
| Mission, principles, ADR | rédigés |
| Score / scion / time-map schema | format id `0.1.0` |
| Signal → dirty-set (`hook_rate` ne salit pas body) | `ref/` + Rust |
| Action graph + action cache | `graft-compile` / `graft-cas` |
| Grain image (`graft-intra`) | dans l’arbre ; dest est `GFI1`, pas un fichier lecteur |
| Long-GOP x264 mp4 | ffmpeg système ; fichiers de slot closed-GOP ; concat `-c copy` |
| Adaptateurs NLE / preview / S3 | pas dans cette version |

Le format id `0.1.0` du schema n’est pas une version de crate. La première
étiquette GitHub du compilateur est `v0.2.0` (ne réutilisez pas l’étiquette
spec `v0.1.0`). Les ruptures de schema passent par RFC.

## Ce que graft n’est pas

- Pas git sur des pixels. N’appliquez pas xdelta à un mp4 de livraison.
- Pas un aller-retour lossless vers chaque NLE. Les adaptateurs sont des invités ; la perte est documentée.
- Pas un serveur de verrou, un DAM ni un outil de revue.

Les cousins (git, OTIO, IMF, ffmpeg concat) sont dans [docs/comparison.md](docs/comparison.md).

## Surface de commandes

```text
graft init
graft slot hook --window 0-3
graft bind hook ./hooks/v3.mov
graft scion 9x16 --dest 1080x1920 --encoder x264
graft compile --out ad.mp4
graft dirty
graft signal --kind hook_rate --t 0-3
```

`export` n’est pas implémenté. `--encoder graft-intra` est le backend à grain
image (tests / image-seq), pas un dest QuickTime.

## Étapes suivantes

La documentation pour les implémenteurs est en anglais. [Traductions de ce README](docs/TRANSLATING.md).

| Doc | Ce qu’elle tranche |
| --- | --- |
| [docs/mission.md](docs/mission.md) | Pourquoi graft existe |
| [docs/principles.md](docs/principles.md) | Non négociables et non-objectifs |
| [docs/glossary.md](docs/glossary.md) | score, slot, scion, kerf, dest |
| [docs/architecture.md](docs/architecture.md) | Couches, graphe des crates, pipeline de compile |
| [docs/schema.md](docs/schema.md) | Commentaire sur le JSON Schema normatif |
| [docs/compile.md](docs/compile.md) | Clés de cache, grain, smart concat |
| [docs/time-map.md](docs/time-map.md) | Comment une métrique adresse un slot |
| [docs/adapters.md](docs/adapters.md) | Matrice de perte |
| [docs/comparison.md](docs/comparison.md) | git, OTIO, IMF, Vit, Aspect |
| [docs/roadmap.md](docs/roadmap.md) | Travail encore devant |
| [docs/brand/](docs/brand/) | Marque : deux clips, une jointure |
| [docs/adr/](docs/adr/) | Décisions déjà prises |

Contrat machine normatif : [`schema/`](schema/).

## Contribuer

Lisez [CONTRIBUTING.md](CONTRIBUTING.md). Les changements de schema exigent un RFC.
Chaque commit requiert un Developer Certificate of Origin (`Signed-off-by`).
Soyez correct : [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Licence

Copyright 2026 [eonik](https://www.eonik.ai/) ([github.com/eonik-ai](https://github.com/eonik-ai)).

Sous [Apache License, Version 2.0](LICENSE).
La concession de brevets est la raison d’Apache-2.0, pas MIT.
Les contributeurs sont de première classe : pas de CLA, pas de cession de copyright.
Vous gardez le copyright de vos correctifs ; DCO et Apache §5 les licencient en entrée.
Voir [NOTICE](NOTICE) et [CONTRIBUTING.md](CONTRIBUTING.md).
