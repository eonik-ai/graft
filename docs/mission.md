# Mission

graft is a compiler and recipe IR for versioning ads.

Hosts write the recipe. graft compiles the dest. Unchanged **subgraphs**
keep their working-cache hits. A new hook must not re-run a clean body
pad. A look, overlap transition, or spanning mix after the join may
still re-filter those pixels. Bitstream-copy of an 8-bit body GOP is
not the rule. See [composition.md](composition.md).

It is not eonik. eonik is a host that should call this compiler. Other
editors may sit on the same IR later as **guests** — compile-out and
lossy import, like USD versus Maya — not as lossless plugins.

The CLI is a reference host. The compiler is the product.

## The compile loop

One **concept**, many **scions**: hook takes, destinations, languages,
legal cuts. A shipped build can carry a platform signal on a time range
("hook rate, 0–3s"). graft resolves that range through the exact build
time map to a slot. The host binds a new take. The compiler rebuilds
only what changed.

> concept → scions → compile → shipped build → signal → addressed slot → new scion

Name the semantic change. Preserve the other scions. Reuse every clean
**subgraph**. Name the join you must recut. Never recut the tree
(principle 14: clean subgraph, not delivery GOP copy).

The reference CLI exposes that as `graft ship`, `graft swap`,
`graft address`. Plumbing (`bind`, `compile`, `signal`, `iterate`) is
what those commands call. A host should call the crates. See
[hosts.md](hosts.md), [loop.md](loop.md), and ADR 0009.

graft is a public open source project. Pages a stranger reads describe
the software, not eonik's GTM tracker. See [principle 15](principles.md).

## Division of responsibility

Git already versions and transports text, so Git owns commits, branches, and
recipe history. graft does not build another commit store.

graft owns the video-aware meaning Git lacks: scions and inheritance, ordered
layer opinions, semantic diff and conflicts, build provenance, exact time maps,
signal-to-slot resolution, and lowering a selected scion into an action graph.

Pixels are not text. Immutable source material belongs in CAS. Derived slot
encodes and kerfs belong in the build/action cache. A delivery H.264 file is
already delta encoded; byte-diffing exports is not the reuse system.

OTIO, NLE formats, and mastering formats remain guests. They may exchange a
documented subset, but graft's native composition remains authoritative.

## Success

graft succeeds when a host can:

1. preserve one concept and multiple inherited scions as Git-tracked recipes;
2. show and merge semantic changes by slot and layer without merging media;
3. compile synchronized video and audio while reusing unchanged slot resources;
4. tie a shipped build to its exact scion, destination, action provenance, and
   time map;
5. ingest a platform signal, resolve it to the intended slot and kerf, and fork
   a new scion without guessing the replacement creative;
6. exchange a supported OTIO/NLE subset with an explicit loss report; and
7. run the same compiler from a host process that depends on `graft-compile`,
   not only from the reference CLI.

The reference CLI also:

8. ships a first dest from a named `takes/` folder in one command;
9. swaps one slot onto a new take, reuses every clean `slot_encode`, and
   prints a reuse ledger a human can read;
10. addresses a named signal to the slot that must change, then waits for a
    take — it does not invent the clip.

## Current state

Schema `0.2.0` implements the IR: multiple tracked scions, layer flattening,
semantic diff/merge, exact build provenance, declared-window feedback,
synchronized audio on the ffmpeg path, scoped OTIO interchange, local
preview, and object-store transport. The only shipping host is the `graft`
CLI (`ship`, `swap`, `address` on that IR). No other app — including eonik —
compiles through these crates yet. Guests are the four interchange files
from one flattened timeline (OTIO, FCPXML, FCP7 XML, MLT), with a loss
report. The compiler is sequential. It does not remotely execute,
lock projects, round-trip NLE effects, infer slots from pixels, or generate
replacement creative.

## Non-mission

graft is not a DAM, a project lock, a review player, an ad publisher, an
NLE, or a replacement for an editor. It is not eonik. The north star is a
compiler hosts embed for surgical ad variants, not "git for video."
