<div align="center">
  <a href="https://github.com/eonik-ai/graft">
    <img src="docs/brand/mark.png" alt="Deux clips. Une jointure." width="120" />
  </a>
  <h1>graft</h1>
  <p><strong>Changez le hook. Gardez le body.</strong></p>
  <p>Un workspace de composition local-first. Le compilateur incrémental est son moteur de build.</p>
  <p>
    <a href="#demarrer">Démarrer</a> ·
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

La boucle fondatrice est concept → scions → itération d’équipe → build
livré → signal → slot adressé → nouveau scion. Git possède l’historique de
la recette ; graft possède la sémantique de composition et la compilation ;
le CAS stocke le matériau source immuable ; l’action cache conserve les
encodes dérivés.

Changez un hook et compilez à nouveau : `graft` encode le hook, copie le
body en bitstream et lie un nouveau dest. Un signal de plateforme contre
ce build exact nomme le hook sans recouper l’arbre.

![graft CLI compile, relie un autre hook et garde body et cta propres](docs/assets/landing.gif)

_Une vraie session locale, enregistrée avec [asciinema](https://github.com/asciinema/asciinema).
La source de lecture est [`landing.cast`](docs/assets/landing.cast)._

## Ce que graft peut faire aujourd’hui

- Garder un concept et plusieurs scions hérités comme recettes suivies par Git
- Binder par scion et layer ; aplatir les opinions de la layer la plus forte
- Afficher un diff sémantique et un merge à trois voies sans fusionner les médias
- Compiler les slots nommés `hook`, `body`, `proof` et `cta` vers un dest
- Mettre en cache l’AAC synchronisé à part et le muxer avec le lien vidéo
- Appliquer `params.speed` enregistré (setpts/atempo ou resample intra)
  sans salir les slots propres adjacents
- Remplir un kerf Long-GOP quand la jointure n’est pas alignée IDR ;
  kerf vide aux jointures closed-GOP
- Ré-encoder un hook changé tout en copiant le body inchangé en bitstream
- Résoudre `hook_rate` via la fenêtre déclarée et la time map livrée
- Exporter/importer un sous-ensemble OTIO borné avec un rapport de perte
- Prévisualiser un scion par décodage et composite, avec audio synchronisé
- Synchroniser les blobs vers une racine object-store avec découverte des blobs manquants

## Ce qu’il ne peut pas faire

graft ne répare pas des timelines mid-GOP arbitraires, n’invente pas une
speed depuis un feedback et ne fait pas d’aller-retour des effets, grades
ou generators d’un NLE. Ce n’est pas Git-sur-pixels, un remplacement de
l’historique Git, un serveur de verrou, un DAM ni un outil de revue.

Le compilateur séquentiel, le graphe d’actions, le CAS filesystem, le
transport object-store, le backend grain image et le chemin ffmpeg/x264
système sont dans l’arbre.
Le format id `0.2.0` du schema n’est pas une version de crate.
La première étiquette GitHub compilateur/workspace est `v0.2.0` ; ne
réutilisez pas l’étiquette de spec `v0.1.0`.
Les changements incompatibles de schema passent par RFC.

## Démarrer

Runtime : **ffmpeg** et **ffprobe** sur `PATH` (ou `$FFMPEG` / `$FFPROBE`).
graft invoque le système ; il ne lie pas x264 GPL. Rust 1.85+ (`rustup`).

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --help
```

Les binaires GitHub Release arrivent quand une étiquette `v0.2.*` est
coupée : macOS arm64 et Linux x64. crates.io n’est pas encore publié
(`publish = false`).

Depuis un clone :

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
```

Votre première compilation :

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

Ouvrez `ad.mp4`. Forkez un scion hook et bindez une autre prise :

```sh
graft scion fork 9x16 hook-v2
graft bind hook ./hook-v2.mov --scion hook-v2
graft diff 9x16 hook-v2
graft compile --scion hook-v2 --out ad-v2.mp4
graft dirty --scion hook-v2
```

`graft dirty` nomme `hook` et son kerf hook→body. `body` et `cta` sont
propres ; leurs octets encodés sont réutilisés.

Adressez une métrique de plateforme via le build livré :

```sh
graft signal --kind hook_rate --build <build-id>
graft iterate --from <build-id> --feedback <id> --scion hook-v3
```

Cela salit `hook` plus le kerf hook→body, pas `body`. `iterate` forke
une demande de changement ; il n’invente pas le clip de remplacement.

L’exemple travaillé n’est que du JSON (hachages factices, pas de média
dans git), donc sa compile imprime un **plan**. Vos clips compilent en mp4.

## Sécurité

Les octets de matériau local et les sorties de build vivent sous `.graft/`,
qui est ignoré. Ne commitez pas l’essence (`.mov`, `.mp4`, `.mxf`) ni des
identifiants. Signalez les vulnérabilités en privé via [SECURITY.md](SECURITY.md).

## Aussi

| | |
|---|---|
| Pourquoi graft existe | [Mission](docs/mission.md) · [principles](docs/principles.md) |
| Contrat du compilateur | [Architecture](docs/architecture.md) · [cache and kerfs](docs/compile.md) · [time map](docs/time-map.md) |
| Contrat machine | [JSON Schema](schema/) · [worked example](examples/hook-v3-body-v1-9x16/) |
| Frontières | [git, OTIO, IMF, ffmpeg concat](docs/comparison.md) · [adapter loss matrix](docs/adapters.md) |
| Projet | [Roadmap](docs/roadmap.md) · [contributing](CONTRIBUTING.md) · [translations](docs/TRANSLATING.md) |

## Licence

Copyright 2026 [eonik](https://www.eonik.ai/) ([github.com/eonik-ai](https://github.com/eonik-ai)).

Sous [Apache License, Version 2.0](LICENSE).
La concession de brevets explique Apache-2.0 plutôt que MIT.
Les contributeurs sont de première classe : pas de CLA, pas de cession.
Vous conservez le copyright de vos rustines ; DCO + Apache §5 les licencient
en entrée. Voir [NOTICE](NOTICE) et [CONTRIBUTING.md](CONTRIBUTING.md).
