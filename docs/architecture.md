# Architecture

graft is a local-first composition workspace. Its typed composition graph and
incremental compiler are the build subsystem, not the complete product. It is
not a video VCS: Git owns recipe history.

This page is the map. New work follows it. If code and this page disagree,
fix the code or file an RFC — do not invent a parallel layout.

## Founding loop and ownership

```
concept → scions → team iteration → shipped build
   ▲                                      │
   └── new scion ← addressed slot ← signal
```

| Plane | Owner | Job |
| --- | --- | --- |
| History | Git | Commits, branches, textual merge, recipe transport |
| Workspace | graft | Concepts, scions, layers, semantic diff/conflicts |
| Build | graft compiler | Flatten, action graph, schedule, encode, exact time map |
| Feedback | graft | Build provenance, raw signal, resolved slots and kerfs |
| Interchange | guest adapters | Supported import/export subset plus loss report |

Scion inheritance is a relationship between variants. Git history is a
relationship between revisions. They are intentionally separate. See ADR 0007.

## Three delta layers inside the build boundary

```
                    git
                     │
                     ▼
              ┌────────────┐
              │   score    │  A. recipe
              │   scion    │
              └─────┬──────┘
                    │ bindings (hashes)
                    ▼
              ┌────────────┐
              │    CAS     │  B. essence (immutable)
              │  materials │
              └─────┬──────┘
                    │ compile
                    ▼
              ┌────────────┐
              │slot_encode │  C. build: action cache
              │audio_encode│     ActionKey → ActionResult
              │   kerf     │
              │  concat    │
              └────────────┘
```

Mixing A/B/C into one "file version" is how "git for video" fails.
`ActionKey` identifies requested work; `BlobId` identifies bytes. The shipped
file is never essence. Recipe Git history, CAS blobs, and action-cache entries
are separate stores. See ADR 0002, ADR 0006, and ADR 0007.

## Stand on, do not fork

- **IMF ST 2067:** CPL (playback timeline) references track files by id.
  A supplemental package is a new CPL plus only changed tracks. graft:
  scion = CPL, materials = source tracks, `slot_encode` / `kerf` =
  derived tracks, concat = assembly.
- **Bazel:** two stores. Content-addressed blobs, and an **action cache**
  mapping `action_key → blob_id`. Hit = artifact already exists for that
  action. Device and server run the same graph; only the store changes
  (ADR 0004).
- **USD:** layers are opinions with strength. Flatten to one binding per
  slot **before** the action graph. Not a pixel blend.
- **smartcut / LosslessCut:** grain is physics (frame vs GOP). Intra kerf
  may be empty. Long-GOP fills the same kerf node later.

## Device vs server

**Device** is authoritative for editing: tracked recipe documents, a local
store, and decode-and-composite preview. Team iteration uses normal Git plus
graft's semantic diff/conflict model; graft does not introduce a project lock.
`graft compile` lowers the selected scion, asks the local action cache what is
dirty, encodes supported misses, and concatenates.

**Server** uses the same compiler and IR with a different store. `graft store`
pushes and pulls namespaced blobs and action results to an object-store root
with resumable writes and missing-blob discovery. There is no remote
execution, DAM, or review product. No custom entropy-coded mp4 delta and no
CapCut cloud render.

## Crate graph

Dependency direction is one way:

```
graft (CLI)
  ├─► graft-compile
  │     └─► graft-cas
  │           └─► graft-score
  └─► graft-otio ──► graft-score
```

Adapters (OTIO, FCPXML, guests) depend on **`graft-score` only**. They
must not import compile or cas. `ref/` is the Python signal rule; it
must not import adapter or crate code.

| Crate | Layer | Job |
| --- | --- | --- |
| `graft-score` | A | Parse, validate, canonically hash the IR. Schema is normative. |
| `graft-cas` | B + C store | Namespaced blobs + action cache. `ActionKey` ≠ `BlobId`. |
| `graft-compile` | C | Flatten → action graph → schedule. Intra encode. Signal dirty-set. |
| `graft-otio` | guest | Scoped OTIO export/import plus loss report. |
| `graft` | — | CLI. Calls the crates. Does not reimplement rules. |

There is no product version. Crates are unpublished `0.2.0` workspace
versions until a GitHub tag is cut. Do not publish crates.io.

New encode work implements `EncodeBackend` against `Action` + CAS. New
stores implement blobs + action cache. Nobody adds a second IR or diffs
two mp4s.

## Compiler subsystem

```
score.json + selected scion
        │
        ▼
   1. load / validate          graft-score
        │
        ▼
   2. flatten layers           one binding per slot (USD strength)
        │
        ▼
   3. lower action graph       SlotEncode, Kerf, Concat
        │
        ├──────────────────────► time map artifact
        │                         keyed by scion_hash
        │                         graft signal uses this ∩ metric
        │
        ▼
   4. schedule                 action cache: hit / miss
        │
        ▼
   5. encode misses            EncodeBackend (`graft-intra` or ffmpeg/x264)
        │                       put BlobId, record ActionKey
        ▼
   6. concat                   hits + misses → dest file
        │                       kind concat, never essence
        ▼
      dest file
```

Two dirty sets, do not collapse them:

| Command | Input | Meaning |
| --- | --- | --- |
| `graft signal` / `graft feedback` | `(kind, declared window, build)` + that build's time map | which **slots** a metric addresses |
| `graft dirty` / `graft compile` | action cache vs this graph | which **actions** have no artifact |

`hook_rate` on the declared hook window dirties `hook` and the hook→body
kerf. It does not dirty `body`. That rule lives in `ref/graft_ref/signal.py`
and `crates/graft-compile/src/signal.rs`. Both must agree. The Python test
`test_hook_rate_uses_declared_window_and_does_not_dirty_body` is load-bearing.

`--prev scion.json` is a debug overlay. It is not the dirty oracle.

The workspace and feedback planes wrap this pipeline:

```
tracked score + selected scion
        │ semantic flatten/diff
        ▼
compiler pipeline ──► build provenance + exact time map
                              │
platform signal ──────────────┘
        │ resolve addressed slots/kerfs
        ▼
explicit new scion and human creative change
```

Schema `0.2.0` is the workspace contract. RFC 0001 is accepted. The CLI
preserves multiple scions, flattens layers, diffs/merges semantically,
compiles through exact build provenance, and resolves declared windows
against that build's time map.

## Module map

### `graft-score`

Mirrors `schema/`. Do not add a field here that is not in the schema.

| Module | Schema / job |
| --- | --- |
| `score` | `score.schema.json` — clock, slots, windows, layer order |
| `scion` | `scion.schema.json` — parent, layers, dest, encoder fingerprint |
| `time` | rational rate + frame ranges |
| `time_map` | `time-map.schema.json` keyed by build |
| `provenance` | `build.schema.json`, `feedback.schema.json` |
| `workspace` | inheritance, flatten, semantic diff/merge |
| `role` | slot roles |
| `material` | `blake3:<64 hex>` |
| `canonical` | BLAKE3 of sorted-key JSON |
| `io` | load / save |
| `span` | `T0-T1`, `WIDTHxHEIGHT` (`9:16` is not a dest) |

### `graft-cas`

| Module | Job |
| --- | --- |
| `Kind` | `material` \| `slot_encode` \| `audio_encode` \| `kerf` \| `concat` |
| `BlobId` | BLAKE3 of bytes (`blake3:<hex>`) |
| `ActionKey` | BLAKE3 of canonical action JSON (hex) |
| `Store` | namespaced `put_blob` / `get_blob` + `get_action` / `put_action` |
| `Memory` | tests and ephemeral device stores |
| `Fs` | `.graft/blobs/<kind>/<aa>/<hex>` and `.graft/actions/<aa>/<hex>` |
| `Object` | same layout on a remote root; resumable put; missing-blob discovery |

Object-store backends implement `Store`. They do not change the IR.
Local store lives at `<project>/.graft` and is gitignored.

### `graft-compile`

| Module | Job |
| --- | --- |
| `flatten` | layer strength → one binding per slot |
| `graph` | lower Score + Scion to the action DAG |
| `schedule` | hits/misses from the action cache |
| `keys` | `slot_encode` / `audio_encode` / `kerf` / `scion_hash` → `ActionKey` |
| `grain` | frame vs GOP; whether a kerf is a no-op |
| `encode` | `EncodeBackend` against `Action` + bytes |
| `concat` | `ConcatBackend`; concat is a linker product |
| `intra` | `FrameIntra`: copy `[in, out)` frames; kerf no-op; splice concat |
| `ffmpeg` | `FfmpegX264`: system ffmpeg/libx264; closed-GOP slot files; concat `-c copy` |
| `pipeline` | `compile()`: lower, schedule, encode misses |
| `signal` | platform metric → dirty slots + kerfs |
| `plan` | JSON view of a schedule |

`graft-intra` is the first row of the grain table (image-seq / All-I).
Long-GOP uses the same graph: `FfmpegX264` shells out to system ffmpeg
(Apache-2.0 graft does not link GPL x264). Slot encodes are closed-GOP
(IDR at the start of each slot file). Concat bitstream-copies them.
Kerf is an empty node at an IDR join; mid-GOP splice can fill that node
later.

The compiler is sequential. The ffmpeg path caches synchronized AAC
independently and muxes it after a verified video link. `ActionResult`
records blob, size, metadata, and provenance. Arbitrary-source mid-GOP
repair remains later.

### `graft` CLI

| Module | Job |
| --- | --- |
| `main` | clap surface |
| `paths` | `score.json` / `scions/` / `feedback/` / `.graft/` |
| `preview` | decode + composite outside the delivery cache |
| `cmd` | one function per subcommand; no rules of its own |

Flags belong in `README.md` only.

The CLI creates, forks, diffs, merges, compiles, ingests feedback, iterates,
exports/imports OTIO, previews, and syncs an object-store root. Git remains
porcelain-light via `graft status`.

## Where to put a change

| You are changing | You edit |
| --- | --- |
| A JSON field on score / scion / time-map | `schema/` + `docs/schema.md` + `graft-score` + example if behaviour changed |
| Signal → dirty-set / spill / hook window | `ref/graft_ref/signal.py` **and** `graft-compile` `signal.rs` + `docs/time-map.md` + example `dirty.json` |
| Cache-key formula | `docs/compile.md` + `graft-compile` `keys.rs` (RFC first) |
| put/get, action cache, path relink | `graft-cas` |
| Grain / kerf policy | `docs/compile.md` + `graft-compile` `grain.rs` |
| An encoder | `EncodeBackend` impl against `Action` + CAS. Never in `graft-score`. Never a second IR. |
| A guest NLE | `docs/adapters.md` + a crate that depends on `graft-score` only |
| A CLI verb | `crates/graft/src/cmd.rs` + `README.md` command surface |
| An irreversible decision | new `docs/adr/NNNN-….md` |

See [comparison.md](comparison.md).
