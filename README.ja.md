[English](README.md) · [简体中文](README.zh-CN.md) · 日本語 · [한국어](README.ko.md) · [Español](README.es.md) · [Português (Brasil)](README.pt-BR.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Русский](README.ru.md)

<img src="docs/brand/mark.png" width="96" align="right" alt="クリップが二つ。接合は一箇所。">

# graft

[![CI](https://github.com/eonik-ai/graft/actions/workflows/ci.yml/badge.svg)](https://github.com/eonik-ai/graft/actions/workflows/ci.yml)
[![Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

score がソース。essence は不変。mp4 はコンパイル成果物。
hook を差し替えても body は残る。

git が **score**（JSON）を版管理する。CAS が essence を版管理する。action cache がエンコードを版管理するので、hook を変えても body は bitstream copy になる。graft は NLE ではない。実行時は `PATH` 上の **ffmpeg** と **ffprobe** が必要。

[Apache-2.0](LICENSE) · [mission](docs/mission.md) · [principles](docs/principles.md) · [schema](schema/) · [近傍のツール](docs/comparison.md)

![graft CLI がコンパイルし、hook を再バインドし、body と cta をクリーンに保つ様子](docs/assets/landing.gif)

_[asciinema](https://github.com/asciinema/asciinema) で記録した実際のローカルセッション。
再生ソースは [`landing.cast`](docs/assets/landing.cast)。_

## はじめる

### 要件

graft はシステムの ffmpeg を呼び出す。GPL の x264 はリンクしない。Rust 1.85+（`rustup`）。
`$FFMPEG` / `$FFPROBE` で `PATH` 上のバイナリを上書きできる。

### インストール

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --help
```

GitHub Release のバイナリ（`v0.2.*` タグを切ったあと）：macOS arm64 と Linux x64。
crates.io はまだ公開していない（`publish = false`）。

クローンから：

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
cargo run -- -C examples/hook-v3-body-v1-9x16 signal --kind hook_rate --t 0-3
```

作業例は JSON のみ（プレースホルダのハッシュ。git にメディアは無い）。
そこで `graft compile` すると **plan** が出る。自分のクリップは mp4 にコンパイルされる。

### 最初のコンパイルを実行する

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

`ad.mp4` を再生する。hook を bind し直して再コンパイルすると、`graft dirty` は `body` **hit** を示す。
body のエンコードは bitstream copy。dest はリンカ成果物であり、essence ではない。

```sh
graft signal --kind hook_rate --t 0-3
```

`{hook}` と hook→body の kerf を出し、`body` は出さない。

未対応：speed/retime、レイヤ重ね、音声、NLE エクスポート。
`params.speed` は action key だけを変える。

## プロジェクトの現状

| 要素 | 状態 |
| --- | --- |
| Mission、principles、ADR | 文書化済み |
| Score / scion / time-map schema | フォーマット id `0.1.0` |
| Signal → dirty-set（`hook_rate` は body を dirty にしない） | `ref/` + Rust |
| Action graph + action cache | `graft-compile` / `graft-cas` |
| フレーム粒度（`graft-intra`） | リポジトリ内。dest は `GFI1` であり、再生ファイルではない |
| Long-GOP x264 mp4 | システム ffmpeg。closed-GOP の slot ファイル。concat `-c copy` |
| NLE アダプタ / プレビュー / S3 | このリリースには無い |

schema のフォーマット id `0.1.0` は crate の版ではない。コンパイラの最初の GitHub タグは `v0.2.0`（spec タグ `v0.1.0` を再利用しない）。破壊的な schema 変更は RFC。

## graft ではないもの

- 画素に対する git ではない。納品 mp4 に xdelta をかけない。
- あらゆる NLE へのロスレス往復ではない。アダプタはゲスト。損失は文書化する。
- ロックサーバでも DAM でもレビューツールでもない。

近傍（git、OTIO、IMF、ffmpeg concat）は [docs/comparison.md](docs/comparison.md)。

## コマンド面

```text
graft init
graft slot hook --window 0-3
graft bind hook ./hooks/v3.mov
graft scion 9x16 --dest 1080x1920 --encoder x264
graft compile --out ad.mp4
graft dirty
graft signal --kind hook_rate --t 0-3
```

`export` は未実装。`--encoder graft-intra` はフレーム粒度バックエンド（テスト / image-seq）であり、QuickTime dest ではない。

## 次のステップ

実装者向けドキュメントは英語。[この README の翻訳ルール](docs/TRANSLATING.md)。

| 文書 | 決めること |
| --- | --- |
| [docs/mission.md](docs/mission.md) | なぜ graft があるか |
| [docs/principles.md](docs/principles.md) | 非交渉事項と非目標 |
| [docs/glossary.md](docs/glossary.md) | score、slot、scion、kerf、dest |
| [docs/architecture.md](docs/architecture.md) | 層、crate グラフ、コンパイルパイプライン |
| [docs/schema.md](docs/schema.md) | 規範 JSON Schema の解説 |
| [docs/compile.md](docs/compile.md) | キャッシュキー、grain、smart concat |
| [docs/time-map.md](docs/time-map.md) | メトリクスが slot を指す方法 |
| [docs/adapters.md](docs/adapters.md) | 損失マトリクス |
| [docs/comparison.md](docs/comparison.md) | git、OTIO、IMF、Vit、Aspect |
| [docs/roadmap.md](docs/roadmap.md) | これから |
| [docs/brand/](docs/brand/) | マーク：クリップが二つ、接合は一箇所 |
| [docs/adr/](docs/adr/) | 既にした決定 |

規範の機械契約：[`schema/`](schema/)。

## 貢献

[CONTRIBUTING.md](CONTRIBUTING.md) を読む。schema 変更には RFC が要る。
すべてのコミットに Developer Certificate of Origin（`Signed-off-by`）が要る。
礼儀：[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。

## ライセンス

Copyright 2026 [eonik](https://www.eonik.ai/)（[github.com/eonik-ai](https://github.com/eonik-ai)）。

[Apache License, Version 2.0](LICENSE) のもとでライセンスする。
Apache-2.0 を選ぶ理由は特許許諾であり、MIT ではない。
貢献者は一等：CLA も著作権譲渡もない。
パッチの著作権はあなたが保持する。DCO と Apache §5 がインバウンドライセンスになる。
[NOTICE](NOTICE) と [CONTRIBUTING.md](CONTRIBUTING.md) を見よ。
