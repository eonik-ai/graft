# Architecture

graft is a typed composition graph plus an incremental compiler.
It is not a video VCS.

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
              │slot_encode │  C. build cache
              │   kerf     │
              │  concat    │
              └────────────┘
```

Mixing A/B/C into one "file version" is how "git for video" fails.

## Device vs server

**Device** (authoritative for edit): score + local CAS + preview composite
(decode, like any NLE). Team iteration is recipe commits and lock-per-slot,
not lock-per-timeline-file. `graft compile` computes the dirty set from the
last scion hash, encodes misses, concats.

**Server**: the same compiler. Object store of materials, slot_encodes, and
kerfs. Optional mastering backend: emit IMF CPL + MXF so a supplemental
package is a new CPL plus changed track files. No custom entropy-coded
mp4 delta. No CapCut cloud render — there is no public API.

## Where things will live in code

```
schema/                 JSON Schema (now; stays the contract)
ref/                    Python reference for signal → dirty set
crates/graft-score      parse / validate / hash the IR
crates/graft-cas        blake3 put/get, no path relink
crates/graft-compile    dirty set, ffmpeg/x264, smart concat
crates/graft            CLI
```

Adapters are separate crates or binaries that depend on `graft-score`
only. They must not become the IR.

## Stand on, do not fork

- **OTIO** for interchange of cuts + media refs. graft adds slots,
  variants, cache, time map (OTIO SchemaDef or sibling JSON).
- **IMF ST 2067** as a *finish* backend, not an authoring IR.
- **smartcut / LosslessCut / ffmpeg concat** for grain-aware joins.
- **OpenUSD** for the *algebra* (layers, variants, strength). Not the file
  format.

See [comparison.md](comparison.md).
