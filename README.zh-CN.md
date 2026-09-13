<div align="center">
  <a href="https://github.com/eonik-ai/graft">
    <img src="docs/brand/mark.png" alt="两段素材，一处接合。" width="120" />
  </a>
  <h1>graft</h1>
  <p><strong>换掉 hook，保留 body。</strong></p>
  <p>本地优先的构图工作区。增量编译器是它的构建引擎。</p>
  <p>
    <a href="#开始使用">开始使用</a> ·
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
    <strong>简体中文</strong> ·
    <a href="README.ja.md">日本語</a> ·
    <a href="README.ko.md">한국어</a> ·
    <a href="README.de.md">Deutsch</a> ·
    <a href="README.ru.md">Русский</a>
  </p>
</div>

---

score 是源。essence 不可变。mp4 是一次编译。

奠基循环是 concept → scions → 团队迭代 → 已交付 build → signal → 被寻址的
slot → 新 scion。Git 拥有配方历史；graft 拥有构图语义与编译；CAS 存放不可变
源素材；action cache 保存派生 encode。

换一段 hook 再编译：`graft` 编码 hook，bitstream 拷贝 body，并链接新 dest。
针对该精确 build 的平台 signal 会点名 hook，而无需重切整棵树。

![graft CLI 编译、重新绑定 hook，并保持 body 与 cta 干净](docs/assets/landing.gif)

_真实的本地会话，由 [asciinema](https://github.com/asciinema/asciinema) 录制。
回放源文件是 [`landing.cast`](docs/assets/landing.cast)。_

## 今天 graft 能做什么

- 用 Git 跟踪的配方保存一个 concept 和多个继承 scion
- 按 scion 与 layer 做 bind；展平最强 layer 的意见
- 显示语义 diff 与三方 merge，不合并媒体
- 把名为 `hook`、`body`、`proof`、`cta` 的 slot 编译到 dest
- 独立缓存同步 AAC，并与视频链路 mux
- 应用已记录的 `params.speed`（setpts/atempo 或 intra 重采样），且不弄脏相邻干净 slot
- 在非 IDR 对齐的接合处填充 Long-GOP kerf；closed-GOP 文件接合仍为空 kerf
- 重编码变更的 hook，同时 bitstream 拷贝未变的 body
- 通过声明窗口与已交付 time map 解析 `hook_rate`
- 导出/导入有范围的 OTIO 子集，并给出损失报告
- 通过 decode 与 composite 预览所选 scion，并带同步音频
- 用缺失 blob 发现把 blob 同步到 object-store 根目录

## 它做不到什么

graft 不修复任意 mid-GOP 时间线，不从 feedback 猜测 speed，也不把 NLE 的
effects、grades、generators 无损往返。它不是像素上的 Git、Git 历史的替代、
锁服务器、DAM 或审片工具。

顺序编译器、action graph、文件系统 CAS、object-store 传输、帧粒度后端以及
系统 ffmpeg/x264 路径都在仓库里。
Schema 的 format id `0.2.0` 不是 crate 版本。
第一个编译器/工作区 GitHub 标签是 `v0.2.0`；不要复用 spec 标签 `v0.1.0`。
破坏性 schema 变更需要 RFC。

## 开始使用

运行时：`PATH` 上的 **ffmpeg** 和 **ffprobe**（或 `$FFMPEG` / `$FFPROBE`）。
graft 调用系统程序，不链接 GPL 的 x264。需要 Rust 1.85+（`rustup`）。

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --help
```

切出 `v0.2.*` 标签后会有 GitHub Release 二进制：macOS arm64 与 Linux x64。
crates.io 尚未发布（`publish = false`）。

从克隆开始：

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
```

第一次编译：

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

播放 `ad.mp4`。fork 一个 hook scion 并 bind 另一条素材：

```sh
graft scion fork 9x16 hook-v2
graft bind hook ./hook-v2.mov --scion hook-v2
graft diff 9x16 hook-v2
graft compile --scion hook-v2 --out ad-v2.mp4
graft dirty --scion hook-v2
```

`graft dirty` 会点名 `hook` 及其 hook→body kerf。`body` 和 `cta` 是干净的；
编码字节会被复用。

通过已交付 build 寻址平台指标：

```sh
graft signal --kind hook_rate --build <build-id>
graft iterate --from <build-id> --feedback <id> --scion hook-v3
```

这会弄脏 `hook` 以及 hook→body kerf，而不是 `body`。`iterate` 会 fork
一项变更请求；它不会发明替换片段。

工作示例只有 JSON（占位哈希，git 中没有媒体），因此那里的 compile 只打印
一份 **plan**。你自己的素材才会编译成 mp4。

## 安全

本地素材字节与 build 输出位于被忽略的 `.graft/`。不要提交 essence
（`.mov`、`.mp4`、`.mxf`）或凭据。通过 [SECURITY.md](SECURITY.md) 私下报告漏洞。

## 另见

| | |
|---|---|
| 为什么存在 graft | [Mission](docs/mission.md) · [principles](docs/principles.md) |
| 编译器契约 | [Architecture](docs/architecture.md) · [cache and kerfs](docs/compile.md) · [time map](docs/time-map.md) |
| 机器契约 | [JSON Schema](schema/) · [worked example](examples/hook-v3-body-v1-9x16/) |
| 边界 | [git, OTIO, IMF, ffmpeg concat](docs/comparison.md) · [adapter loss matrix](docs/adapters.md) |
| 项目 | [Roadmap](docs/roadmap.md) · [contributing](CONTRIBUTING.md) · [translations](docs/TRANSLATING.md) |

## 许可

Copyright 2026 [eonik](https://www.eonik.ai/) ([github.com/eonik-ai](https://github.com/eonik-ai)).

基于 [Apache License, Version 2.0](LICENSE)。
选择 Apache-2.0 而非 MIT，是因为专利授权。
贡献者是一等公民：没有 CLA，也没有版权转让。
你保留补丁版权；DCO 与 Apache §5 将其许可进来。
见 [NOTICE](NOTICE) 与 [CONTRIBUTING.md](CONTRIBUTING.md)。
