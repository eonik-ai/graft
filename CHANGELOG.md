# Changelog

All notable changes to this project are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

The `graft` field on JSON is a format id (`0.2.0`), not a crate version.
First compiler GitHub tag is `v0.2.0`. Do not reuse spec tag `v0.1.0`.
crates.io crate name `graft` is taken (orbitinghail storage engine).
This project's crates stay `publish = false`.

## [Unreleased]

### Added

- Generic filter lowerer, stored adjustment groups, and eased keys
  (RFC 0010). A filter-shaped op this ffmpeg has lowers as `key=value`
  plus `enable=`. A missing filter is `toolchain`. Unknown params fail
  closed. `adjustments[]` stores `op: group`; compile expands children
  and inherits `enable=`. Expression options ease in the filter string;
  numeric options segment with `enable=`. `loudnorm` keeps `dual_mono`.
- Motion, stepwise time remap, and a multi-cam switch (RFC 0009).
  `binding.motion` moves an overlay without recoding its source.
  `binding.remap` folds speed spans in the working path; intra refuses
  a curve. Nest `switch.cuts` is the host cut list. `graft swap overlay
  --x --y` edits place without a new file. `graft address` names
  `angles.switch` when that node covers the signal.
- Composition inventory: [docs/composition.md](docs/composition.md)
  (ADR 0010, principle 16). Living list of what a video is — graph,
  ffmpeg lowerer, families (time, color/LUT, transitions, composite,
  motion, titles, finish, audio, codecs). Append-only. Agents read it
  without being asked.
- Graph compiler (RFC 0003): working-depth nodes (ProRes HQ / PCM), overlap
  `xfade` / `acrossfade`, optional `scion.graph` (`lut3d`, overlay,
  gap, …), role `sfx`, compiled dest time map, one delivery encode.
  Working-source keys omit the delivery encoder. `graft-intra` stays
  a frame splice.

- Porcelain authors the composition, not only slots:
  `takes/composition.json` (joins, `dest.working`, graph nodes with
  `file` refs), plus `look.cube` / `legal.cube` / role media.
  `graft swap <node>` updates any graph node (LUT, params JSON, or
  essence). A child with `graph` omitted inherits the parent graph.
  `graft preview` is `compile`. Sidechain nodes whose inputs are audio
  slots duck **before** the mix. Finish/grade ops: `eq`, `hue`,
  `curves`, `colorbalance`, `colortemperature`, `vibrance`, `unsharp`,
  `vignette`, `noise`, `gblur`, `setrange`, `loudnorm`.
  Porcelain `composition.json` `slots[]` bind `file` + `in`/`out` on a
  library clip (C9). `graft swap <slot> file --in --src-out` rebinds a
  range (`--out` is the dest path). Named `takes/<role>.*` stays a
  crutch. OTIO sidecar preserves `scion.graph`; native OTIO effects stay lost.
  J/L: optional `binding.audio.place` and dest-placed spine audio.
  Keys/shapes: `chromakey`, `colorkey`, `lumakey`, `chromahold`,
  `alphamerge`, `drawbox`, `drawgrid`, overlay `opacity`. Captions mux
  into the dest (`mov_text`). RFC 0004.
  Role `overlay` (extra picture pads). Host join aliases
  (`dipToBlack` → `fadeblack`). `score.markers`. OTIO sidecar restores
  `scion.graph` and overlay bindings. `graft address` lists covering
  nodes. Intra fails closed on a graph. Geometry ops: `setsar`,
  `setdar`, `perspective`, `shear`, `scroll`, `premultiply`,
  `streamselect`. Stills `-loop 1`. Porcelain `dest` / `spans` /
  `markers`. RFC 0005.
  Three-pad `maskedmerge`, stacks/`mix`, host blend aliases, mixboard
  audio, chapter mux. OTIO import writes sidecar joins/markers onto
  the score. Porcelain graph `source` nodes take `in`/`out`. RFC 0006.
  Nested scion-as-pad: `binding.nested` / graph `nested` compile an
  inner score to its working `#picture` (not delivery). Porcelain
  `nests` + slot `"scion"`. Intra fails closed on a nest. RFC 0007.
  Adjustment groups, overlay place, nest address, color convert
  (RFC 0008): porcelain `adjustments[]` stamps `enable=` on a node
  group; `binding.x` / `y` / `opacity` fingerprint `overlay_mix` only;
  `graft address` names `plate.look` and next is
  `graft swap look --scion plate`; `colorspace` primaries/trc;
  `cdl` / `lgg` / `colorcorrect` / `bwdif` / `tile` / `haldclut`.

### Changed

- Public pages name graft as a **compiler hosts embed**, not a person's
  workspace. eonik is a host, not this project. Premiere / Resolve /
  DaVinci stay guests (lossy). English is the only README; locale siblings
  are removed. See [hosts.md](docs/hosts.md).
- README / loop / hosts describe the working graph and inherited look,
  not bitstream-copy of H.264 cells.
- README example is a real `ship` + `swap` on 9:16 product takes (hook
  changes, body/cta cache-hit). Color-plate GIFs are no longer the demo.
- GitHub Release intel macOS binary is cross-compiled on `macos-latest`
  (`--target x86_64-apple-darwin`) so the cut is not blocked on `macos-13`
  runners. `gh release upload --clobber` if the tag already has a release.
- Dist recipes carry the `v0.2.2` GitHub tag-archive hashes
  ([`dist/SOURCE`](dist/SOURCE)). Homebrew formula has `url`/`sha256` for
  that tag. Homebrew 7 needs `brew trust --tap eonik-ai/graft` before the
  tap will load.

## [0.2.2] - 2026-09-16

Porcelain closes the founding user loop. Schema format id stays `0.2.0`.
crates.io crate name `graft` is taken; this CLI is not published there.

### Added

- Porcelain user loop (ADR 0009): `graft ship` from `takes/`, `graft swap`
  of one slot (or `--from` a take pool), `graft address` to prime the next
  swap. Every encode prints a reuse ledger. `ship` fails if it cannot
  encode a dest. `swap` fails if a sibling `slot_encode` recoded. See
  [`docs/loop.md`](docs/loop.md). Locale README siblings are stale for the
  `ship` encode-fail sentence and the install block.
- Install surface for a public cut: Homebrew tap in this repo
  (`Formula/graft.rb`), `scripts/install.sh`, Nix flake, GitHub Release
  tarballs for macOS arm64/x86_64 and Linux gnu+musl, `.deb` packages.
  Packager recipes live in [`dist/`](dist/README.md); a recipe is not a
  distro listing. See [`docs/install.md`](docs/install.md) and
  [`docs/packaging.md`](docs/packaging.md).
- Visual walkthrough: `make walkthrough` writes labeled takes, ships,
  swaps the hook, and records a side-by-side GIF
  ([`docs/assets/walkthrough.gif`](docs/assets/walkthrough.gif)). Body GOP
  bytes inside the linked dest are compared across the swap.
- crates.io name `graft` is taken; GitHub Releases, the Homebrew tap, and
  the Nix flake are the published install paths.
- Founding-loop history walk: Git owns recipe revisions, scion `parent` is
  variant derivation, `.graft/` stays out of Git. See
  [`docs/history.md`](docs/history.md). CLI test
  `founding_loop_history_is_not_an_mp4`.
- Worked example [`examples/dub-en-9x16/`](examples/dub-en-9x16/): 10s
  clock dirty-set contract (picture → `dub-en` / `dub-hi` inherit,
  `vo_hold`, explicit 3–5s `note`, `hold` on body). Not a dubbed film:
  `audio_mix` is spine AAC plus overlay. Bindings are BLAKE3 of local
  essence; mp4s stay out of git.

## [0.2.1] - 2026-09-14

Compiler tag after the founding loop. Schema format id stays `0.2.0`.
crates.io stays unpublished (`publish = false`).

### Added

- Optional `score.joins` (`cut` / `fade`). Fade fills a real kerf; changing
  duration misses that kerf only. Closed-GOP `cut` stays empty at IDR.
- Overlay plane: `vo`/`bed` `audio_mix`, `captions` sidecar, `brand`
  `overlay_mix`. A hook or caption swap does not recut body `slot_encode`.
- HTTP and S3 object-store backends on the same blob/action layout.
  `graft store` `--remote` accepts a path, `https://`, or `s3://`.
- `encoder.level` and `rate_control.mode=bitrate` reach ffmpeg. The dest
  encoder fingerprint is shared, so a level/bitrate change misses every
  `slot_encode`. Unknown binding `params` besides `speed` are rejected.
  `scion create` defaults `--dest-id` from `score.dest_default`.
- OTIO import restores bound spine audio from `metadata.graft.audio`.
- Applied `params.speed`: `graft-intra` resamples dest frames; ffmpeg uses
  `setpts` / `atempo`. Adjacent clean slots stay hits.
- Long-GOP kerf fill for non-IDR joins; empty kerf remains the closed-GOP
  IDR path. Unproven repair fails with `kerf_unproven`.
- Preview muxes synced audio (silence-padded when a spine slot lacks it).
  Linked dest audio duration is checked against picture.

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

[Unreleased]: https://github.com/eonik-ai/graft/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/eonik-ai/graft/releases/tag/v0.2.2
[0.2.1]: https://github.com/eonik-ai/graft/releases/tag/v0.2.1
[0.2.0]: https://github.com/eonik-ai/graft/releases/tag/v0.2.0
[0.1.0]: https://github.com/eonik-ai/graft/releases/tag/v0.1.0
