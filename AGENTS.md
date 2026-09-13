# AGENTS.md

Instructions for humans and coding agents working in this repository.

graft is a spec-first compiler for video composition. The north star is
[docs/mission.md](docs/mission.md). Non-negotiables are
[docs/principles.md](docs/principles.md). Read both before any edit.

The project name and CLI are **`graft`**, lowercase, like `git`. Do not
rename it Graft in prose, headings, or code.

## What this repo is

Until the Rust CLI exists, the deliverable is:

1. Principles and ADRs that must not be silently rewritten.
2. Normative JSON Schema in `schema/`.
3. The worked example in `examples/hook-v3-body-v1-9x16/`.
4. A tiny Python reference for signal → dirty-set in `ref/`.

Do not scaffold a fake encoder. Do not add a second reference language.
Do not import eonik GTM, lead lists, or "replace the editor" copy.

## Build and test

```sh
make test    # schema + examples + north-star unit tests
make lint    # JSON parse, python compileall
make fmt     # no-op until a formatter is pinned
```

CI must call these targets, not ad-hoc commands.

## Architecture (when code lands)

```
schema/     normative contract (JSON Schema 2020-12)
examples/   fixtures the compiler must match
ref/        Python reference for rules that can be unit-tested without ffmpeg
crates/     future Rust: graft (CLI), graft-score, graft-cas, graft-compile
docs/adr/   append-only decisions
```

Dependency direction: `graft` CLI → compile → cas → score. Adapters depend
on score, never the reverse. `ref/` must not import adapter code.

## Where new work goes

| Change | Directory |
| --- | --- |
| Object, field, enum on the IR | `schema/*.schema.json` + `docs/schema.md` |
| Signal addressing, spill, hook window | `ref/graft_ref/signal.py` + `docs/time-map.md` + example `dirty.json` |
| Cache / kerf / grain | `docs/compile.md` (and later `crates/graft-compile`) |
| Irreversible decision | new `docs/adr/NNNN-….md` — never edit an accepted ADR's decision section |
| Guest NLE | `docs/adapters.md` only, with loss. RFC if you claim less loss |
| Rust implementation | `crates/` as named above, Apache-2.0 SPDX headers |

## Tests

- Python: stdlib `unittest`, files `ref/tests/test_*.py`.
- The test `test_hook_rate_does_not_dirty_body` is load-bearing. Do not
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
