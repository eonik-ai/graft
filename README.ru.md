<div align="center">
  <a href="https://github.com/eonik-ai/graft">
    <img src="docs/brand/mark.png" alt="Два клипа. Одно соединение." width="120" />
  </a>
  <h1>graft</h1>
  <p><strong>Меняйте hook. Сохраняйте body.</strong></p>
  <p>Локальный workspace композиции. Инкрементальный компилятор — его движок сборки.</p>
  <p>
    <a href="#начало-работы">Начало работы</a> ·
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
    <a href="README.de.md">Deutsch</a> ·
    <strong>Русский</strong>
  </p>
</div>

---

score — источник. essence неизменна. mp4 — результат компиляции.

Основательский цикл: concept → scions → итерация команды → отгруженный
build → signal → адресованный slot → новый scion. Git владеет историей
рецепта; graft владеет семантикой композиции и компиляцией; CAS хранит
неизменный исходный материал; action cache хранит производные encodes.

Смените hook и скомпилируйте снова: `graft` кодирует hook, копирует body
битстримом и связывает новый dest. Платформенный signal против этого
точного build называет hook, не перерезая дерево.

![graft CLI компилирует, привязывает другой hook и сохраняет body и cta чистыми](docs/assets/landing.gif)

_Настоящий локальный сеанс, записанный с [asciinema](https://github.com/asciinema/asciinema).
Исходник воспроизведения — [`landing.cast`](docs/assets/landing.cast)._

## Что graft умеет сегодня

- Хранить один concept и много унаследованных scion как рецепты в Git
- Делать bind по scion и layer; сглаживать мнения самой сильной layer
- Показывать семантический diff и трёхсторонний merge без слияния медиа
- Компилировать именованные слоты `hook`, `body`, `proof` и `cta` в dest
- Кэшировать синхронизированный AAC отдельно и muxить его с видеосвязкой
- Применять записанный `params.speed` (setpts/atempo или intra-ресемпл),
  не пачкая соседние чистые slot
- Заполнять Long-GOP kerf, если стык не выровнен по IDR; пустой kerf на
  closed-GOP файловых стыках
- Перекодировать изменённый hook и копировать неизменный body битстримом
- Разрешать `hook_rate` через объявленное окно и отгруженную time map
- Экспортировать/импортировать ограниченное подмножество OTIO с отчётом о потерях
- Предпросматривать выбранный scion через decode и composite, с синхронным звуком
- Синхронизировать blob в корень object-store с поиском недостающих blob

## Чего он не умеет

graft не чинит произвольные mid-GOP таймлайны, не выдумывает speed из
feedback и не гоняет туда-обратно эффекты, grades или generators NLE.
Это не Git-на-пикселях, не замена истории Git, не сервер блокировок,
не DAM и не инструмент рецензии.

Последовательный компилятор, граф действий, файловый CAS, транспорт
object-store, кадровый backend и системный путь ffmpeg/x264 уже в дереве.
format id схемы `0.2.0` — не версия crate.
Первый тег GitHub компилятора/workspace — `v0.2.0`; не используйте повторно
спецификационный тег `v0.1.0`.
Несовместимые изменения схемы идут через RFC.

## Начало работы

Среда: **ffmpeg** и **ffprobe** в `PATH` (или `$FFMPEG` / `$FFPROBE`).
graft вызывает систему; GPL x264 не линкуется. Rust 1.85+ (`rustup`).

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --help
```

Бинарники GitHub Release появятся при теге `v0.2.*`: macOS arm64 и Linux x64.
crates.io пока не опубликован (`publish = false`).

Из клона:

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
```

Первая компиляция:

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

Воспроизведите `ad.mp4`. Сделайте fork scion hook и bind другой дубль:

```sh
graft scion fork 9x16 hook-v2
graft bind hook ./hook-v2.mov --scion hook-v2
graft diff 9x16 hook-v2
graft compile --scion hook-v2 --out ad-v2.mp4
graft dirty --scion hook-v2
```

`graft dirty` называет `hook` и kerf hook→body. `body` и `cta` чистые;
их закодированные байты переиспользуются.

Адресуйте платформенную метрику через отгруженный build:

```sh
graft signal --kind hook_rate --build <build-id>
graft iterate --from <build-id> --feedback <id> --scion hook-v3
```

Это пачкает `hook` плюс kerf hook→body, не `body`. `iterate` делает fork
запроса на изменение; он не изобретает заменяющий клип.

Рабочий пример — только JSON (заглушки хешей, без медиа в git), поэтому
его compile печатает **plan**. Ваши клипы компилируются в mp4.

## Безопасность

Локальные байты материала и выходы сборки живут в `.graft/`, который
игнорируется. Не коммитьте essence (`.mov`, `.mp4`, `.mxf`) и учётные данные.
О уязвимостях сообщайте приватно через [SECURITY.md](SECURITY.md).

## Также

| | |
|---|---|
| Зачем существует graft | [Mission](docs/mission.md) · [principles](docs/principles.md) |
| Контракт компилятора | [Architecture](docs/architecture.md) · [cache and kerfs](docs/compile.md) · [time map](docs/time-map.md) |
| Машинный контракт | [JSON Schema](schema/) · [worked example](examples/hook-v3-body-v1-9x16/) |
| Границы | [git, OTIO, IMF, ffmpeg concat](docs/comparison.md) · [adapter loss matrix](docs/adapters.md) |
| Проект | [Roadmap](docs/roadmap.md) · [contributing](CONTRIBUTING.md) · [translations](docs/TRANSLATING.md) |

## Лицензия

Copyright 2026 [eonik](https://www.eonik.ai/) ([github.com/eonik-ai](https://github.com/eonik-ai)).

Лицензировано по [Apache License, Version 2.0](LICENSE).
Патентная лицензия — причина Apache-2.0, а не MIT.
Участники первого класса: нет CLA и нет уступки авторских прав.
Вы сохраняете авторство патчей; DCO и Apache §5 лицензируют их внутрь.
См. [NOTICE](NOTICE) и [CONTRIBUTING.md](CONTRIBUTING.md).
