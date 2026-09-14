<div align="center">
  <a href="https://github.com/eonik-ai/graft">
    <img src="docs/brand/mark.png" alt="Zwei Clips. Eine Fuge." width="120" />
  </a>
  <h1>graft</h1>
  <p><strong>Ändere den hook. Behalte den body.</strong></p>
  <p>Ein local-first Kompositions-Workspace. Der inkrementelle Compiler ist seine Build-Engine.</p>
  <p>
    <a href="#erste-schritte">Erste Schritte</a> ·
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
    <a href="README.fr.md">Français</a> ·
    <a href="README.zh-CN.md">简体中文</a> ·
    <a href="README.ja.md">日本語</a> ·
    <a href="README.ko.md">한국어</a> ·
    <strong>Deutsch</strong> ·
    <a href="README.ru.md">Русский</a>
  </p>
</div>

---

Der score ist Quelle. Essence ist unveränderlich. Die mp4 ist ein Compile-Ergebnis.

Die Gründungsschleife ist concept → scions → Team-Iteration → ausgelieferter
build → signal → adressierter slot → neuer scion. Git besitzt die
Rezept-Historie; graft besitzt Kompositionssemantik und Compile; CAS speichert
unveränderliches Quellmaterial; der action cache hält abgeleitete Encodes.

Ändere einen hook und compile erneut: `graft` encoded den hook, bitstream-kopiert
den body und linkt ein neues dest. Ein Plattform-signal gegen genau diesen
build benennt den hook, ohne den Baum neu zu schneiden.

![graft CLI compiliert, bindet einen anderen hook und hält body und cta sauber](docs/assets/landing.gif)

_Eine echte lokale Sitzung, aufgenommen mit [asciinema](https://github.com/asciinema/asciinema).
Die Wiedergabequelle ist [`landing.cast`](docs/assets/landing.cast)._

## Was graft heute kann

- Ein concept und viele geerbte scions als Git-verfolgte Rezepte halten
- Nach scion und layer binden; die stärksten Layer-Meinungen flachlegen
- Semantischen diff und Drei-Wege-merge ohne Medien-Merge zeigen
- Benannte `hook`-, `body`-, `proof`- und `cta`-slots zu einem dest compilieren
- `vo` / `bed` als gemischtes Audio, `captions` als Sidecar und `brand` als
  Overlay auf dem Bild-concat compilieren
- Ein `fade`-join auf dem score benennen, ohne benachbarte slot_encode neu
  zu schreiben
- Synchrones AAC getrennt cachen und mit dem Videolink muxen
- Aufgezeichnetes `params.speed` anwenden (setpts/atempo oder intra-Resample),
  ohne benachbarte saubere slots dirty zu machen
- Ein Long-GOP-kerf füllen, wenn der Schnitt nicht IDR-ausgerichtet ist;
  leeres kerf an closed-GOP-Dateischnitten
- Einen geänderten hook neu encoden und den unveränderten body bitstream-kopieren
- `hook_rate` über das deklarierte Fenster und die ausgelieferte time map auflösen
- Eine begrenzte OTIO-Teilmenge mit Verlustbericht exportieren/importieren
- Einen gewählten scion per Decode und Composite previewen, mit Sync-Audio
  und Overlay-Mix
- Blobs zu einer object-store-Wurzel mit Missing-Blob-Entdeckung
  synchronisieren. Es gibt kein Remote-compile.

## Was es nicht kann

graft repariert keine beliebigen Mid-GOP-Timelines, erfindet kein speed aus
Feedback und rundet keine NLE-Effects, Grades oder Generators. Es ist kein
Git-auf-Pixeln, kein Ersatz für Git-Historie, kein Lock-Server, kein DAM
und kein Review-Werkzeug.

Der sequenzielle Compiler, der Action-Graph, das Dateisystem-CAS, der
object-store-Transport, das Frame-Grain-Backend und der System-ffmpeg/x264-Pfad
liegen im Baum.
Die Schema-format-id `0.2.0` ist keine Crate-Version.
Das erste Compiler-/Workspace-GitHub-Tag ist `v0.2.0`; das Spec-Tag `v0.1.0`
nicht wiederverwenden.
Inkompatible Schema-Änderungen brauchen ein RFC.

## Erste Schritte

Laufzeit: **ffmpeg** und **ffprobe** auf `PATH` (oder `$FFMPEG` / `$FFPROBE`).
graft ruft das System auf; es linkt kein GPL-x264. Rust 1.85+ (`rustup`).

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --help
```

GitHub-Release-Binaries kommen, wenn ein `v0.2.*`-Tag geschnitten wird:
macOS arm64 und Linux x64. crates.io ist noch nicht veröffentlicht
(`publish = false`).

Aus einem Clone:

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
```

Dein erstes Compile:

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

Spiele `ad.mp4`. Forke einen hook-scion und binde einen anderen Take:

```sh
graft scion fork 9x16 hook-v2
graft bind hook ./hook-v2.mov --scion hook-v2
graft diff 9x16 hook-v2
graft compile --scion hook-v2 --out ad-v2.mp4
graft dirty --scion hook-v2
```

`graft dirty` nennt `hook` und dessen hook→body-kerf. `body` und `cta` sind
sauber; ihre encoded Bytes werden wiederverwendet.

Adresse eine Plattformmetrik über den ausgelieferten build:

```sh
graft signal --kind hook_rate --build <build-id>
graft iterate --from <build-id> --feedback <id> --scion hook-v3
```

Das macht `hook` plus den hook→body-kerf dirty, nicht `body`. `iterate` forkt
eine Änderungsanfrage; es erfindet den Ersatzclip nicht.

Das ausgearbeitete Beispiel ist nur JSON (Platzhalter-Hashes, keine Medien
in git), deshalb druckt dessen compile einen **plan**. Deine eigenen Clips
compilieren zu mp4.

## Sicherheit

Lokale Material-Bytes und Build-Ausgaben leben unter `.graft/`, das ignoriert
ist. Keine Essence (`.mov`, `.mp4`, `.mxf`) und keine Zugangsdaten committen.
Schwachstellen privat über [SECURITY.md](SECURITY.md) melden.

## Außerdem

| | |
|---|---|
| Warum graft existiert | [Mission](docs/mission.md) · [principles](docs/principles.md) |
| Compilervertrag | [Architecture](docs/architecture.md) · [cache and kerfs](docs/compile.md) · [time map](docs/time-map.md) |
| Maschinenvertrag | [JSON Schema](schema/) · [worked example](examples/hook-v3-body-v1-9x16/) |
| Grenzen | [git, OTIO, IMF, ffmpeg concat](docs/comparison.md) · [adapter loss matrix](docs/adapters.md) |
| Projekt | [Roadmap](docs/roadmap.md) · [contributing](CONTRIBUTING.md) · [translations](docs/TRANSLATING.md) |

## Lizenz

Copyright 2026 [eonik](https://www.eonik.ai/) ([github.com/eonik-ai](https://github.com/eonik-ai)).

Lizenziert unter der [Apache License, Version 2.0](LICENSE).
Die Patentlizenz ist der Grund für Apache-2.0 statt MIT.
Mitwirkende sind erstklassig: kein CLA, keine Copyright-Abtretung.
Du behältst das Copyright an deinen Patches; DCO + Apache §5 lizenzieren sie inbound.
Siehe [NOTICE](NOTICE) und [CONTRIBUTING.md](CONTRIBUTING.md).
