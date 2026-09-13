# AGENTS.md

Instructions for humans and coding agents working in this repository.

graft is a local-first composition workspace whose incremental compiler is a
subsystem. Git owns recipe history; graft owns scions, layers, semantic
diff/conflicts, build provenance, signal resolution, and compile lowering. The
north star is [docs/mission.md](docs/mission.md). Non-negotiables are
[docs/principles.md](docs/principles.md). Read both before any edit.

The project name and CLI are **`graft`**, lowercase, like `git`. Do not
rename it Graft in prose, headings, or code.

## What this repo is

The current implementation is a local-first workspace: tracked multi-scion
documents, layer flattening, semantic diff/merge, build provenance,
declared-window feedback, a sequential compiler with synchronized audio on
the ffmpeg path, a scoped OTIO adapter, local preview, and object-store
transport. Do not claim lossless NLE round-trips, remote execution, DAM,
review UI, or a published crates.io release. Do not import eonik GTM, lead
lists, or "replace the editor" copy. First compiler GitHub tag is `v0.2.0`
(not spec `v0.1.0`). crates.io stays unpublished until that tag.

## Build and test

```sh
make test # schema + examples + north-star unit tests + cargo test
make lint # JSON parse, python compileall, rustfmt, clippy
make fmt  # cargo fmt
```

CI must call these targets, not ad-hoc commands.

## Architecture

```
schema/     normative contract (JSON Schema 2020-12)
examples/   fixtures the compiler must match
ref/        Python reference for rules that can be unit-tested without ffmpeg
crates/     graft → graft-compile → graft-cas → graft-score
             (see docs/architecture.md)
docs/adr/   append-only decisions
docs/rfc/   proposed contract changes; no implementation claims
```

Dependency direction: `graft` CLI → compile → cas → score. Adapters depend
on score, never the reverse. `ref/` must not import adapter code.

## Where new work goes

| Change | Directory |
| --- | --- |
| Object, field, enum on the IR | `schema/*.schema.json` + `docs/schema.md` |
| Signal addressing, spill, hook window | `ref/graft_ref/signal.py` + `docs/time-map.md` + example `dirty.json` |
| Cache / kerf / grain | `docs/compile.md` + `crates/graft-compile` |
| Irreversible decision | new `docs/adr/NNNN-….md` — never edit an accepted ADR's decision section |
| Guest NLE | `docs/adapters.md` only, with loss. RFC if you claim less loss |
| Rust implementation | `crates/` as named in [docs/architecture.md](docs/architecture.md). Encode implements `EncodeBackend` against Action + CAS. Stores implement blobs + action cache. |

## Tests

- Python: stdlib `unittest`, files `ref/tests/test_*.py`.
- Rust: `crates/*/src` and `crates/graft/tests`. `cargo test` must keep
  `hook_rate` on `[0, 3)` from dirtying `body` in the worked example.
- The Python test `test_hook_rate_does_not_dirty_body` is load-bearing. Do not
  delete it to make a change pass. If the rule changes, an RFC comes first
  and the test changes in the same PR.
- No media files in git. Fixtures are JSON.

## Commits and PRs

- Conventional Commits. PRs are squash-merged; the **PR title** is the commit.
- Every commit: `Signed-off-by` (DCO).
- Do not commit `.env`, credentials, or essence (`.mp4` `.mov` `.mxf`).

## Documentation sync

| Change | Also update |
| --- | --- |
| schema | `docs/schema.md`, Unreleased in `CHANGELOG.md`, examples if behaviour changes |
| dirty-set rule | `docs/time-map.md`, `examples/**/dirty.json` |
| cache key | `docs/compile.md` |
| a principle | this file, in one sentence if the agent-routing changes |
| intended CLI | `README.md` only — do not invent flags in random docs |

## Invariants (fail the PR)

- Three delta layers stay distinct: recipe (git), essence (CAS), build (encode cache).
- Git owns recipe history; graft owns composition semantics. Scion inheritance is not commit history.
- 9:16 is a **dest**, not a slot.
- The shipped mp4 is never hashed as essence.
- Adapters are lossy guests. No "lossless CapCut/Premiere/Resolve" claims.
- Device-first: preview is decode+composite; server is the same compiler.
- Do not propose rsync/xdelta on re-exported H.264 as the delta system.

## License

Apache-2.0. Copyright **eonik** (https://www.eonik.ai/,
https://github.com/eonik-ai). No CLA. Contributors keep copyright on
their patches. SPDX: `Apache-2.0`. New source files get

```
SPDX-License-Identifier: Apache-2.0
```

when the language has comments.
