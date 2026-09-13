[English](README.md) · 简体中文 · [日本語](README.ja.md) · [한국어](README.ko.md) · [Español](README.es.md) · [Português (Brasil)](README.pt-BR.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Русский](README.ru.md)

# graft

<img src="docs/brand/mark.png" width="120" alt="两段素材，一处接合。">

视频合成编译器。

[![CI](https://github.com/eonik-ai/graft/actions/workflows/ci.yml/badge.svg)](https://github.com/eonik-ai/graft/actions/workflows/ci.yml)
[![Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

score 是源。essence 不可变。mp4 是一次编译。
换一段 hook，body 留下。

git 版本化 **score**（JSON）。CAS 版本化 essence。action cache 版本化编码结果，因此更换 hook 时 body 走 bitstream copy。graft 不是 NLE。运行时需要 `PATH` 上的 **ffmpeg** 和 **ffprobe**。

[Apache-2.0](LICENSE) · [mission](docs/mission.md) · [principles](docs/principles.md) · [schema](schema/) · [邻近工具](docs/comparison.md)

## 安装

graft 调用系统里的 ffmpeg，不链接 GPL 的 x264。需要 Rust 1.85+（`rustup`）。
`$FFMPEG` / `$FFPROBE` 可覆盖 `PATH` 上的二进制。

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --help
```

GitHub Release 二进制（在切 `v0.2.*` 标签之后）：macOS arm64 与 Linux x64。
crates.io 尚未发布（`publish = false`）。

从克隆安装：

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
cargo run -- -C examples/hook-v3-body-v1-9x16 signal --kind hook_rate --t 0-3
```

仓库里的工作示例只有 JSON（占位哈希，git 中没有媒体）。
在那里运行 `graft compile` 只会打印一份 **plan**。你自己的素材才会编译成 mp4。

## 编译三段素材（9x16）

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

播放 `ad.mp4`。重新 bind hook 再编译一次：`graft dirty` 会显示 `body` **hit**。
body 的编码结果按 bitstream copy。dest 是链接产物，永远不是 essence。

```sh
graft signal --kind hook_rate --t 0-3
```

打印 `{hook}` 以及 hook→body 的 kerf，不会打印 `body`。

目前不支持：speed/retime、图层叠加、音频、NLE 导出。
`params.speed` 只改变 action key。

## 状态

| 部分 | 状态 |
| --- | --- |
| Mission、principles、ADR | 已写入 |
| Score / scion / time-map schema | 格式 id `0.1.0` |
| Signal → dirty-set（`hook_rate` 不脏 body） | `ref/` + Rust |
| Action graph + action cache | `graft-compile` / `graft-cas` |
| 帧粒度（`graft-intra`） | 在仓库内；dest 是 `GFI1`，不是播放器文件 |
| Long-GOP x264 mp4 | 系统 ffmpeg；closed-GOP 的 slot 文件；concat `-c copy` |
| NLE 适配器 / 预览 / S3 | 本版本没有 |

schema 格式 id `0.1.0` 不是 crate 版本。编译器的第一个 GitHub 标签是 `v0.2.0`（不要复用 spec 标签 `v0.1.0`）。破坏性 schema 变更走 RFC。

## graft 不是什么

- 不是对像素做 git。不要对成片 mp4 做 xdelta。
- 不是通往每个 NLE 的无损往返。适配器是客人；损耗写在文档里。
- 不是锁服务器、DAM 或审片工具。

邻近工具（git、OTIO、IMF、ffmpeg concat）见 [docs/comparison.md](docs/comparison.md)。

## 命令面

```text
graft init
graft slot hook --window 0-3
graft bind hook ./hooks/v3.mov
graft scion 9x16 --dest 1080x1920 --encoder x264
graft compile --out ad.mp4
graft dirty
graft signal --kind hook_rate --t 0-3
```

`export` 尚未实现。`--encoder graft-intra` 是帧粒度后端（测试 / image-seq），不是 QuickTime dest。

## 文档

实现者文档为英文。[本 README 的翻译约定](docs/TRANSLATING.md)。

| 文档 | 解决什么 |
| --- | --- |
| [docs/mission.md](docs/mission.md) | 为什么有 graft |
| [docs/principles.md](docs/principles.md) | 不可妥协项与非目标 |
| [docs/glossary.md](docs/glossary.md) | score、slot、scion、kerf、dest |
| [docs/architecture.md](docs/architecture.md) | 分层、crate 图、编译管线 |
| [docs/schema.md](docs/schema.md) | 对规范 JSON Schema 的说明 |
| [docs/compile.md](docs/compile.md) | 缓存键、grain、smart concat |
| [docs/time-map.md](docs/time-map.md) | 指标如何寻址到 slot |
| [docs/adapters.md](docs/adapters.md) | 损耗矩阵 |
| [docs/comparison.md](docs/comparison.md) | git、OTIO、IMF、Vit、Aspect |
| [docs/roadmap.md](docs/roadmap.md) | 尚未完成的工作 |
| [docs/brand/](docs/brand/) | 标志：两段素材，一处接合 |
| [docs/adr/](docs/adr/) | 已经做出的决定 |

规范的机器契约：[`schema/`](schema/)。

## 贡献

请读 [CONTRIBUTING.md](CONTRIBUTING.md)。schema 变更需要 RFC。
所有提交必须带 Developer Certificate of Origin（`Signed-off-by`）。
请友善：[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。

## 许可

Copyright 2026 [eonik](https://www.eonik.ai/)（[github.com/eonik-ai](https://github.com/eonik-ai)）。

以 [Apache License, Version 2.0](LICENSE) 许可。
选择 Apache-2.0 而不是 MIT，是因为专利授权。
贡献者是一等公民：没有 CLA，也没有版权转让。
你保留补丁的版权；DCO 与 Apache §5 把它们许可进来。
见 [NOTICE](NOTICE) 和 [CONTRIBUTING.md](CONTRIBUTING.md)。
