# Architecture

graft is a typed composition graph plus an incremental compiler.
It is not a video VCS.

This page is the map. New work follows it. If code and this page disagree,
fix the code or file an RFC — do not invent a parallel layout.

## Three delta layers

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
              │   kerf     │     ActionKey → BlobId
              │  concat    │
              └────────────┘
```

Mixing A/B/C into one "file version" is how "git for video" fails.
`ActionKey` is recipe + dest + encoder. `BlobId` is bytes. The shipped
file is never essence. See ADR 0002 and ADR 0006.

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

**Device** (authoritative for edit): score + local store + preview
composite (decode, like any NLE). Team iteration is recipe commits and
lock-per-slot, not lock-per-timeline-file. `graft compile` lowers the
action graph, asks the action cache what is dirty, encodes misses, concats.

**Server**: the same compiler. Object store of materials, slot_encodes, and
kerfs. Optional mastering backend: emit IMF CPL + MXF so a supplemental
package is a new CPL plus changed track files. No custom entropy-coded
mp4 delta. No CapCut cloud render — there is no public API.

## Crate graph

Dependency direction is one way:

```
graft (CLI)
  └─► graft-compile
        └─► graft-cas
              └─► graft-score
```

Adapters (OTIO, FCPXML, guests) depend on **`graft-score` only**. They
must not import compile or cas. `ref/` is the Python signal rule; it
must not import adapter or crate code.

| Crate | Layer | Job |
| --- | --- | --- |
| `graft-score` | A | Parse, validate, canonically hash the IR. Schema is normative. |
| `graft-cas` | B + C store | Namespaced blobs + action cache. `ActionKey` ≠ `BlobId`. |
| `graft-compile` | C | Flatten → action graph → schedule. Intra encode. Signal dirty-set. |
| `graft` | — | CLI. Calls the crates. Does not reimplement rules. |

There is no product version. Crates are unpublished `0.0.0` until a
release is cut. Do not tag CLI or crate milestones.

New encode work implements `EncodeBackend` against `Action` + CAS. New
stores implement blobs + action cache. Nobody adds a second IR or diffs
two mp4s.

## Compile pipeline

```
score.json + scion.json
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
| `graft signal` | `(kind, [t0, t1), dest)` + time map | which **slots** a metric addresses |
| `graft dirty` / `graft compile` | action cache vs this graph | which **actions** have no artifact |

`hook_rate` on `[0, 3)` dirties `hook` and the hook→body kerf. It does
not dirty `body`. That rule lives in `ref/graft_ref/signal.py` and
`crates/graft-compile/src/signal.rs`. Both must agree. The Python test
`test_hook_rate_does_not_dirty_body` is load-bearing.

`--prev scion.json` is a debug overlay. It is not the dirty oracle.

## Module map

### `graft-score`

Mirrors `schema/`. Do not add a field here that is not in the schema.

| Module | Schema / job |
| --- | --- |
| `score` | `score.schema.json` — clock, slots, windows, layers |
| `scion` | `scion.schema.json` — bindings, dest, encoder fingerprint |
| `time_map` | `time-map.schema.json` |
| `role` | slot roles |
| `material` | `blake3:<64 hex>` |
| `canonical` | BLAKE3 of sorted-key JSON (millisecond seconds) |
| `io` | load / save |
| `span` | `T0-T1`, `WIDTHxHEIGHT` (`9:16` is not a dest) |

### `graft-cas`

| Module | Job |
| --- | --- |
| `Kind` | `material` \| `slot_encode` \| `kerf` \| `concat` |
| `BlobId` | BLAKE3 of bytes (`blake3:<hex>`) |
| `ActionKey` | BLAKE3 of canonical action JSON (hex) |
| `Store` | namespaced `put_blob` / `get_blob` + `get_action` / `put_action` |
| `Memory` | tests and ephemeral device stores |
| `Fs` | `.graft/blobs/<kind>/<aa>/<hex>` and `.graft/actions/<aa>/<hex>` |

Object-store backends implement `Store`. They do not change the IR.
Local store lives at `<project>/.graft` and is gitignored.

### `graft-compile`

| Module | Job |
| --- | --- |
| `flatten` | layer strength → one binding per slot |
| `graph` | lower Score + Scion to the action DAG |
| `schedule` | hits/misses from the action cache |
| `keys` | `slot_encode` / `kerf` / `scion_hash` → `ActionKey` |
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

### `graft` CLI

| Module | Job |
| --- | --- |
| `main` | clap surface |
| `paths` | `score.json` / `scion.json` / `time-map.json` / `.graft/` |
| `cmd` | one function per subcommand; no rules of its own |

Flags belong in `README.md` only.

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
