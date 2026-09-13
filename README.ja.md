<div align="center">
  <a href="https://github.com/eonik-ai/graft">
    <img src="docs/brand/mark.png" alt="クリップが二つ。接合は一箇所。" width="120" />
  </a>
  <h1>graft</h1>
  <p><strong>hook を変える。body は残す。</strong></p>
  <p>ローカルファーストのコンポジションワークスペース。増分コンパイラがそのビルドエンジンです。</p>
  <p>
    <a href="#はじめる">はじめる</a> ·
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
    <strong>日本語</strong> ·
    <a href="README.ko.md">한국어</a> ·
    <a href="README.de.md">Deutsch</a> ·
    <a href="README.ru.md">Русский</a>
  </p>
</div>

---

score がソース。essence は不変。mp4 はコンパイル成果物。

創業ループは concept → scions → チーム反復 → 出荷済み build → signal →
アドレスされた slot → 新しい scion。Git がレシピ履歴を持ち、graft が
コンポジション意味論とコンパイルを持ち、CAS が不変のソース素材を蓄え、
action cache が派生 encode を保持します。

hook を変えて再コンパイルすると、`graft` は hook を encode し、body を
bitstream copy し、新しい dest をリンクします。その正確な build に対する
プラットフォーム signal は、木を切り直さずに hook を指名します。

![graft CLI がコンパイルし、hook を再バインドし、body と cta をクリーンに保つ様子](docs/assets/landing.gif)

_[asciinema](https://github.com/asciinema/asciinema) で記録した実際のローカルセッション。
再生ソースは [`landing.cast`](docs/assets/landing.cast)。_

## いま graft ができること

- 1 つの concept と多数の継承 scion を Git 追跡レシピとして保つ
- scion と layer で bind し、最も強い layer の意見を平坦化する
- メディアをマージせずに意味的 diff と三方向 merge を示す
- 名前付き `hook`、`body`、`proof`、`cta` slot を dest へコンパイルする
- 同期 AAC を独立キャッシュし、映像リンクと mux する
- 記録済み `params.speed` を適用する（setpts/atempo または intra 再サンプル）。隣接するクリーン slot は汚さない
- 接合が IDR 非整列のとき Long-GOP kerf を埋める。closed-GOP ファイル接合では空 kerf
- 変わった hook を再 encode し、変わらない body を bitstream copy する
- 宣言ウィンドウと出荷済み time map で `hook_rate` を解決する
- 範囲付き OTIO サブセットを損失レポート付きで export/import する
- decode と composite で選択 scion をプレビューし、同期音声を載せる
- 欠損 blob 発見つきで object-store ルートへ blob を同期する

## できないこと

graft は任意の mid-GOP タイムラインを修復せず、feedback から speed を発明せず、
NLE の effects、grades、generators を往復しません。ピクセル上の Git でも、
Git 履歴の代替でも、ロックサーバでも、DAM でも、レビューツールでもありません。

逐次コンパイラ、action graph、ファイルシステム CAS、object-store 転送、
フレーム粒度バックエンド、システムの ffmpeg/x264 経路はツリー内にあります。
schema の format id `0.2.0` は crate バージョンではありません。
最初のコンパイラ／ワークスペース GitHub タグは `v0.2.0` です。spec タグ
`v0.1.0` を再利用しないでください。
破壊的な schema 変更には RFC が必要です。

## はじめる

ランタイム：`PATH` 上の **ffmpeg** と **ffprobe**（または `$FFMPEG` / `$FFPROBE`）。
graft はシステムを呼び出します。GPL の x264 はリンクしません。Rust 1.85+（`rustup`）。

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --help
```

`v0.2.*` タグを切ると GitHub Release バイナリが届きます：macOS arm64 と
Linux x64。crates.io は未公開です（`publish = false`）。

クローンから：

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
```

最初のコンパイル：

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

`ad.mp4` を再生します。hook scion を fork し、別テイクを bind します：

```sh
graft scion fork 9x16 hook-v2
graft bind hook ./hook-v2.mov --scion hook-v2
graft diff 9x16 hook-v2
graft compile --scion hook-v2 --out ad-v2.mp4
graft dirty --scion hook-v2
```

`graft dirty` は `hook` とその hook→body kerf を指名します。`body` と
`cta` はクリーンで、encode 済みバイトが再利用されます。

出荷済み build 経由でプラットフォーム指標をアドレスします：

```sh
graft signal --kind hook_rate --build <build-id>
graft iterate --from <build-id> --feedback <id> --scion hook-v3
```

これは `hook` と hook→body kerf を汚し、`body` は汚しません。`iterate` は
変更要求を fork します。置き換えクリップは発明しません。

作業例は JSON のみ（プレースホルダハッシュ、git にメディアなし）なので、
そこでの compile は **plan** を印字します。自分のクリップは mp4 になります。

## セキュリティ

ローカル素材バイトと build 出力は無視される `.graft/` の下にあります。
essence（`.mov`、`.mp4`、`.mxf`）や資格情報をコミットしないでください。
脆弱性は [SECURITY.md](SECURITY.md) で非公開報告してください。

## 関連

| | |
|---|---|
| graft が存在する理由 | [Mission](docs/mission.md) · [principles](docs/principles.md) |
| コンパイラ契約 | [Architecture](docs/architecture.md) · [cache and kerfs](docs/compile.md) · [time map](docs/time-map.md) |
| 機械契約 | [JSON Schema](schema/) · [worked example](examples/hook-v3-body-v1-9x16/) |
| 境界 | [git, OTIO, IMF, ffmpeg concat](docs/comparison.md) · [adapter loss matrix](docs/adapters.md) |
| プロジェクト | [Roadmap](docs/roadmap.md) · [contributing](CONTRIBUTING.md) · [translations](docs/TRANSLATING.md) |

## ライセンス

Copyright 2026 [eonik](https://www.eonik.ai/) ([github.com/eonik-ai](https://github.com/eonik-ai)).

[Apache License, Version 2.0](LICENSE) のもとでライセンスされます。
特許許諾があるため MIT ではなく Apache-2.0 です。
コントリビュータは第一級です。CLA も著作権譲渡もありません。
パッチの著作権はあなたに残り、DCO と Apache §5 がインバウンドを許諾します。
[NOTICE](NOTICE) と [CONTRIBUTING.md](CONTRIBUTING.md) を見てください。
