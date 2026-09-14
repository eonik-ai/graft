<div align="center">
  <a href="https://github.com/eonik-ai/graft">
    <img src="docs/brand/mark.png" alt="클립 둘, 이음 하나." width="120" />
  </a>
  <h1>graft</h1>
  <p><strong>hook을 바꾼다. body는 남긴다.</strong></p>
  <p>로컬 우선 컴포지션 워크스페이스. 증분 컴파일러가 빌드 엔진이다.</p>
  <p>
    <a href="#시작하기">시작하기</a> ·
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
    <strong>한국어</strong> ·
    <a href="README.de.md">Deutsch</a> ·
    <a href="README.ru.md">Русский</a>
  </p>
</div>

---

score가 소스다. essence는 불변이다. mp4는 컴파일 결과다.

창립 루프는 concept → scions → 팀 반복 → 출고된 build → signal → 주소가
지정된 slot → 새 scion이다. Git은 레시피 이력을 소유하고, graft는 컴포지션
의미와 컴파일을 소유하며, CAS는 불변 소스 소재를 저장하고, action cache는
파생 encode를 유지한다.

hook을 바꾸고 다시 컴파일하면 `graft`는 hook을 encode하고, body를 bitstream
copy하며, 새 dest를 링크한다. 그 정확한 build에 대한 플랫폼 signal은 트리를
다시 자르지 않고 hook을 가리킨다.

![graft CLI가 컴파일하고 hook을 다시 바인드한 뒤 body와 cta를 깨끗하게 유지하는 모습](docs/assets/landing.gif)

_[asciinema](https://github.com/asciinema/asciinema)로 녹화한 실제 로컬 세션.
재생 소스는 [`landing.cast`](docs/assets/landing.cast)입니다._

## 오늘 graft가 하는 일

- Git이 추적하는 레시피로 하나의 concept와 여러 상속 scion을 유지한다
- scion과 layer로 bind하고 가장 강한 layer 의견을 평탄화한다
- 미디어를 병합하지 않고 의미 diff와 삼방 merge를 보여 준다
- 이름이 있는 `hook`, `body`, `proof`, `cta` slot을 dest로 컴파일한다
- `vo` / `bed`는 혼합 오디오, `captions`는 sidecar, `brand`는 그림 concat 위 overlay로 컴파일한다
- 인접 slot_encode를 다시 쓰지 않고 score에 `fade` join을 이름을 붙인다
- 동기화된 AAC를 따로 캐시하고 비디오 링크와 mux한다
- 기록된 `params.speed`를 적용한다(setpts/atempo 또는 intra 리샘플). 인접한 깨끗한 slot은 더럽히지 않는다
- 이음이 IDR에 맞지 않으면 Long-GOP kerf를 채운다. closed-GOP 파일 이음은 빈 kerf
- 바뀐 hook을 다시 encode하고 바뀌지 않은 body는 bitstream copy한다
- 선언된 창과 출고된 time map으로 `hook_rate`를 해석한다
- 범위가 있는 OTIO 부분집합을 손실 보고서와 함께 export/import한다
- decode와 composite로 선택한 scion을 미리 보고, 동기 오디오와 overlay mix를 실는다
- 경로, `https://`, 또는 `s3://`로 누락 blob 발견과 함께 동기화한다. remote compile은 없다.

## 하지 않는 일

graft는 임의의 mid-GOP 타임라인을 수리하지 않고, feedback에서 speed를
만들어 내지 않으며, NLE의 effects, grades, generators를 왕복하지 않는다.
픽셀 위의 Git도, Git 이력의 대체재도, 잠금 서버도, DAM도, 리뷰 도구도 아니다.

순차 컴파일러, action graph, 파일시스템 CAS, object-store 전송, 프레임 입자
백엔드, 시스템 ffmpeg/x264 경로는 트리 안에 있다.
schema format id `0.2.0`은 crate 버전이 아니다.
첫 컴파일러/워크스페이스 GitHub 태그는 `v0.2.0`이다. spec 태그 `v0.1.0`을
재사용하지 마라.
파괴적 schema 변경에는 RFC가 필요하다.

## 시작하기

런타임: `PATH`의 **ffmpeg**와 **ffprobe**(또는 `$FFMPEG` / `$FFPROBE`).
graft는 시스템을 호출한다. GPL x264를 링크하지 않는다. Rust 1.85+(`rustup`).

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --help
```

`v0.2.*` 태그를 자르면 GitHub Release 바이너리가 온다: macOS arm64와
Linux x64. crates.io는 아직 게시되지 않는다(`publish = false`).

클론에서:

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
```

첫 컴파일:

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

`ad.mp4`를 재생한다. hook scion을 fork하고 다른 테이크를 bind한다:

```sh
graft scion fork 9x16 hook-v2
graft bind hook ./hook-v2.mov --scion hook-v2
graft diff 9x16 hook-v2
graft compile --scion hook-v2 --out ad-v2.mp4
graft dirty --scion hook-v2
```

`graft dirty`는 `hook`과 hook→body kerf를 가리킨다. `body`와 `cta`는
깨끗하며 encode된 바이트가 재사용된다.

출고된 build를 통해 플랫폼 지표를 주소 지정한다:

```sh
graft signal --kind hook_rate --build <build-id>
graft iterate --from <build-id> --feedback <id> --scion hook-v3
```

이는 `hook`과 hook→body kerf를 더럽히고 `body`는 더럽히지 않는다.
`iterate`는 변경 요청을 fork한다. 대체 클립을 발명하지 않는다.

작업 예제는 JSON뿐이다(플레이스홀더 해시, git에 미디어 없음). 그 compile은
**plan**을 인쇄한다. 자신의 클립은 mp4로 컴파일된다.

## 보안

로컬 소재 바이트와 build 출력은 무시되는 `.graft/` 아래에 있다.
essence(`.mov`, `.mp4`, `.mxf`)나 자격 증명을 커밋하지 마라.
취약점은 [SECURITY.md](SECURITY.md)로 비공개 보고하라.

## 더 보기

| | |
|---|---|
| graft가 존재하는 이유 | [Mission](docs/mission.md) · [principles](docs/principles.md) |
| 컴파일러 계약 | [Architecture](docs/architecture.md) · [cache and kerfs](docs/compile.md) · [time map](docs/time-map.md) |
| 기계 계약 | [JSON Schema](schema/) · [worked example](examples/hook-v3-body-v1-9x16/) |
| 경계 | [git, OTIO, IMF, ffmpeg concat](docs/comparison.md) · [adapter loss matrix](docs/adapters.md) |
| 프로젝트 | [Roadmap](docs/roadmap.md) · [contributing](CONTRIBUTING.md) · [translations](docs/TRANSLATING.md) |

## 라이선스

Copyright 2026 [eonik](https://www.eonik.ai/) ([github.com/eonik-ai](https://github.com/eonik-ai)).

[Apache License, Version 2.0](LICENSE)로 라이선스된다.
특허 허여 때문에 MIT가 아니라 Apache-2.0이다.
기여자는 일급이다. CLA도 저작권 양도도 없다.
패치 저작권은 기여자에게 남고, DCO와 Apache §5가 인바운드를 라이선스한다.
[NOTICE](NOTICE)와 [CONTRIBUTING.md](CONTRIBUTING.md)를 보라.
