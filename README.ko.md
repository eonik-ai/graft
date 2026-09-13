<div align="center">
  <a href="https://github.com/eonik-ai/graft"><img src="docs/brand/mark.png" alt="클립 둘, 이음 하나." width="120" /></a>
  <h1>graft</h1>
  <p><strong>hook을 바꾼다. body는 남긴다.</strong></p>
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
hook을 바꿔도 body는 남는다.

git이 **score**(JSON)를 버전한다. CAS가 essence를 버전한다. action cache가 인코드를 버전하므로 hook을 바꿔도 body는 bitstream copy가 된다. graft는 NLE가 아니다. 런타임은 `PATH`의 **ffmpeg**와 **ffprobe**다.

![graft CLI가 컴파일하고 hook을 다시 바인드한 뒤 body와 cta를 깨끗하게 유지하는 모습](docs/assets/landing.gif)

_[asciinema](https://github.com/asciinema/asciinema)로 녹화한 실제 로컬 세션.
재생 소스는 [`landing.cast`](docs/assets/landing.cast)입니다._

## 시작하기

### 요구 사항

graft는 시스템 ffmpeg를 호출한다. GPL x264를 링크하지 않는다. Rust 1.85+(`rustup`).
`$FFMPEG` / `$FFPROBE`로 `PATH`의 바이너리를 덮어쓴다.

### 설치

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --help
```

GitHub Release 바이너리(`v0.2.*` 태그를 자른 뒤): macOS arm64와 Linux x64.
crates.io는 아직 게시하지 않는다(`publish = false`).

클론에서:

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
cargo run -- -C examples/hook-v3-body-v1-9x16 signal --kind hook_rate --t 0-3
```

작업 예제는 JSON뿐이다(플레이스홀더 해시, git에 미디어 없음).
거기에서 `graft compile`은 **plan**만 출력한다. 당신의 클립이 mp4로 컴파일된다.

### 첫 컴파일 실행하기

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

`ad.mp4`를 재생한다. hook을 다시 bind하고 컴파일하면 `graft dirty`가 `body` **hit**을 보여 준다.
body 인코드는 bitstream copy다. dest는 링커 산물이며 essence가 아니다.

```sh
graft signal --kind hook_rate --t 0-3
```

`{hook}`과 hook→body kerf를 출력하고 `body`는 출력하지 않는다.

아직 없음: speed/retime, 레이어 오버레이, 오디오, NLE 내보내기.
`params.speed`는 action key만 바꾼다.

## 프로젝트 상태

| 조각 | 상태 |
| --- | --- |
| Mission, principles, ADR | 문서화됨 |
| Score / scion / time-map schema | 포맷 id `0.1.0` |
| Signal → dirty-set (`hook_rate`는 body를 dirty로 만들지 않음) | `ref/` + Rust |
| Action graph + action cache | `graft-compile` / `graft-cas` |
| 프레임 입자 (`graft-intra`) | 저장소 안. dest는 `GFI1`이지 플레이어 파일이 아님 |
| Long-GOP x264 mp4 | 시스템 ffmpeg. closed-GOP slot 파일. concat `-c copy` |
| NLE 어댑터 / 프리뷰 / S3 | 이 릴리스에 없음 |

schema 포맷 id `0.1.0`은 crate 버전이 아니다. 컴파일러의 첫 GitHub 태그는 `v0.2.0`이다(spec 태그 `v0.1.0`을 재사용하지 말 것). 파괴적 schema 변경은 RFC다.

## graft가 아닌 것

- 픽셀에 대한 git이 아니다. 납품 mp4에 xdelta를 쓰지 마라.
- 모든 NLE로의 무손실 왕복이 아니다. 어댑터는 손님이다. 손실은 문서화한다.
- 락 서버, DAM, 리뷰 도구가 아니다.

가까운 도구(git, OTIO, IMF, ffmpeg concat)는 [docs/comparison.md](docs/comparison.md).

## 명령 면

```text
graft init
graft slot hook --window 0-3
graft bind hook ./hooks/v3.mov
graft scion 9x16 --dest 1080x1920 --encoder x264
graft compile --out ad.mp4
graft dirty
graft signal --kind hook_rate --t 0-3
```

`export`는 구현되지 않았다. `--encoder graft-intra`는 프레임 입자 백엔드(테스트 / image-seq)이지 QuickTime dest가 아니다.

## 다음 단계

구현자 문서는 영어다. [이 README의 번역 규칙](docs/TRANSLATING.md).

| 문서 | 정하는 것 |
| --- | --- |
| [docs/mission.md](docs/mission.md) | graft가 있는 이유 |
| [docs/principles.md](docs/principles.md) | 타협하지 않는 것과 비목표 |
| [docs/glossary.md](docs/glossary.md) | score, slot, scion, kerf, dest |
| [docs/architecture.md](docs/architecture.md) | 계층, crate 그래프, 컴파일 파이프라인 |
| [docs/schema.md](docs/schema.md) | 규범 JSON Schema 해설 |
| [docs/compile.md](docs/compile.md) | 캐시 키, grain, smart concat |
| [docs/time-map.md](docs/time-map.md) | 메트릭이 slot을 가리키는 방법 |
| [docs/adapters.md](docs/adapters.md) | 손실 행렬 |
| [docs/comparison.md](docs/comparison.md) | git, OTIO, IMF, Vit, Aspect |
| [docs/roadmap.md](docs/roadmap.md) | 남은 일 |
| [docs/brand/](docs/brand/) | 마크: 클립 둘, 이음 하나 |
| [docs/adr/](docs/adr/) | 이미 내린 결정 |

규범 기계 계약: [`schema/`](schema/).

## 기여

[CONTRIBUTING.md](CONTRIBUTING.md)를 읽으십시오. schema 변경은 RFC가 필요합니다.
모든 커밋에 Developer Certificate of Origin(`Signed-off-by`)이 필요합니다.
친절하십시오: [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## 라이선스

Copyright 2026 [eonik](https://www.eonik.ai/) ([github.com/eonik-ai](https://github.com/eonik-ai)).

[Apache License, Version 2.0](LICENSE)으로 라이선스한다.
Apache-2.0를 고른 이유는 특허 허여이며, MIT가 아니다.
기여자는 일등이다. CLA도 저작권 양도도 없다.
패치의 저작권은 당신이 갖는다. DCO와 Apache §5가 인바운드 라이선스다.
[NOTICE](NOTICE)와 [CONTRIBUTING.md](CONTRIBUTING.md)를 보라.
