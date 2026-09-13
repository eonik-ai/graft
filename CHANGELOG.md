# Changelog

All notable changes to this project are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

The `graft` field on JSON is a format id (`0.1.0`), not a crate version.
First compiler GitHub tag is `v0.2.0`. Do not reuse spec tag `v0.1.0`.
crates.io is not published (`publish = false`).

## [Unreleased]

### Added

- Install: `cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft`.
  GitHub Release workflow for `v0.2.*` binaries (macOS arm64, Linux x64).
- `graft scion --encoder x264|graft-intra`, `graft compile --out`, bind probe
  (ffprobe / GFI1).
- Long-GOP: system ffmpeg/x264. Closed-GOP slot encodes; concat `-c copy`.
  Hook swap keeps the body blob. Needs ffmpeg on `PATH`.
- ADR 0006: action graph is the compile IR; action cache is the dirty
  oracle. `ActionKey` is not `BlobId`.
- Namespaced CAS plus action cache under `.graft/`.
- Frame-grain backend `graft-intra` (algebra; dest is GFI1).
- JSON CLI. Encoder `profile` is optional on the scion schema.
- `CITATION.cff` (CFF 1.2). `docs/TRANSLATING.md` for README locales.
- Reproducible asciinema CLI demo: compile, bind a new hook, then show
  `body` and `cta` clean in the action-cache dirty set.

### Changed

- README is a tool-first compiler landing page: text H1 `# graft`, mark as
  a mark, real CLI movie, first compile, how it works, project status, next
  steps, language bar, and eight first-party siblings.
- README matches the product: mp4 via ffmpeg; example JSON is plan-only.
- MSRV 1.85; CI remains 1.98.1.

## [0.1.0] - 2026-09-13

North-star snapshot (format id `0.1.0` on documents). Not a compiler
release.

### Added

- Mission, principles, glossary, architecture, compile, time-map, adapters.
- ADRs 0001–0005 (name, three delta layers, adapters as guests, device-first,
  Apache-2.0 + DCO).
- Normative JSON Schema `0.1.0` for score, scion, and time map.
- Worked example `hook_v3 + body_v1 + cta_v1 @ 9x16`.
- Reference signal → dirty-set implementation and north-star test.

[Unreleased]: https://github.com/eonik-ai/graft/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/eonik-ai/graft/releases/tag/v0.1.0
