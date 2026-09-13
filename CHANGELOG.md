# Changelog

All notable changes to this project are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

The `graft` field on JSON is a format id (`0.2.0`), not a crate version.
First compiler GitHub tag is `v0.2.0`. Do not reuse spec tag `v0.1.0`.
crates.io is not published (`publish = false`).

## [Unreleased]

## [0.2.0] - 2026-09-14

First workspace/compiler GitHub tag. Schema format id `0.2.0`. crates.io
stays unpublished (`publish = false`).

### Added

- Schema `0.2.0`: tracked `scions/<id>.json`, inheritance, layer opinions,
  rational time, build records, and feedback documents (RFC 0001).
- Workspace CLI: `scion create|fork|list|show|use`, layered bind, semantic
  `diff` / `merge`, `feedback ingest|list|show`, `iterate`, `status`.
- Declared-window signal resolution against an exact shipped build.
- Synchronized AAC on the ffmpeg path; `ActionResult` provenance; concat
  verification; object-store missing/push/pull.
- Scoped OTIO export/import (`graft-otio`) with a machine-readable loss
  report, plus local decode-and-composite preview.
- `graft migrate` and `fixtures/migration-0.1.0/` for 0.1.0 documents.
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
- Reproducible asciinema CLI demo: compile, `scion fork`, bind a new hook,
  then show `body` and `cta` clean in the action-cache dirty set.
- Persisted founding-loop CLI test on generated media: init, inherited
  scions, compile, declared-window `hook_rate`, iterate, OTIO guest,
  preview, and object-store push.

### Changed

- README follows the eonik organization structure: centered identity,
  crawlable `graft` H1, promise, navigation, flat-square badges, languages,
  repository boundary, capabilities, non-capabilities, install, security,
  related docs, license. The real CLI movie and eight first-party siblings
  remain.
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

[Unreleased]: https://github.com/eonik-ai/graft/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/eonik-ai/graft/releases/tag/v0.2.0
[0.1.0]: https://github.com/eonik-ai/graft/releases/tag/v0.1.0
