# 0006 — action graph is the compile IR; action cache is the dirty oracle

- Status: accepted
- Date: 2026-09-13

## Context

ADR 0002 splits recipe, essence, and build. Mixing those layers is how
"git for video" fails: an action key (recipe + dest + encoder) is not the
hash of encoded bytes, and the shipped file is not essence.

A thin CLI that diffs two `scion.json` files treats "same binding text as
last time" as a cache hit. That is not an incremental compiler. IMF ST 2067
already keeps unchanged track files by id when the CPL changes. Bazel
reuses **actions**, not files: an action cache maps `action_key → blob_id`.
USD flattens layered opinions to one binding before any work. smartcut
says grain is physics (frame vs GOP); intra kerf may be empty.

Device and server must run the same graph (ADR 0004). Only the store
changes.

## Decision

The compile IR is an **action graph** lowered from a flattened scion:

1. One `SlotEncode` per spine slot. Output kind `slot_encode`.
2. One `Kerf` per adjacent spine pair (`transition = cut`). Intra +
   `Grain::Frame`: the kerf is a no-op splice (empty artifact, still a
   node so Long-GOP can fill it later).
3. One `Concat`. Output is a linker product. Stored as kind `concat` if
   stored at all. **Never** hashed as essence.

Dirty is: this `ActionKey` is absent from the action cache, or its input
blob is missing. It is not a diff of two scion documents. `--prev` may
remain as a debug overlay only.

Two hashes stay distinct:

- `ActionKey` — BLAKE3 of the canonical JSON in [compile.md](../compile.md)
  (`slot_encode`, `kerf`, `scion_hash`).
- `BlobId` — BLAKE3 of **bytes** (material, slot_encode file, kerf file).

CAS has two stores: namespaced blobs (`material` | `slot_encode` | `kerf`
| `concat`) and an action cache `ActionKey → { kind, blob }`.

`graft signal` remains a different dirty set (time map ∩ metric). Do not
collapse it into the action cache.

This implements ADR 0002. It does not change ADR 0002's Decision.

## Consequences

- Contributors add encode backends against `Action` + CAS. Stores
  implement blobs + action cache. Nobody adds a second IR or diffs two
  mp4s.
- Intra (`Grain::Frame`, image-seq / All-I) is proven before Long-GOP.
  A hook swap must keep the body `BlobId` and body frame bytes identical.
- Time map is a build artifact keyed by `scion_hash`, not an identity
  derived only for the CLI.
