# Mission

graft is a local-first composition workspace for iterating video.
Its incremental compiler is the build engine, not the whole system.

The score is source.
Git is history.
Essence is immutable.
The mp4 is a compile.

You graft a new hook. The body stays.

## The founding loop

A team starts with **one concept** and maintains many **scions**: hook takes,
destinations, languages, legal cuts, and other derived compositions. Teammates
iterate on named slots and layers. A shipped build then receives a platform
signal on its own time range ("hook rate, 0–3s", "too slow around 8s").
graft resolves that signal through the exact build time map to a semantic slot,
the team creates a new scion, and the compiler rebuilds only what changed.

> concept → scions → team iteration → shipped build → platform signal →
> addressed slot → new scion

graft's job is to make that loop explicit, inspectable, and local-first:

> Name the semantic change. Preserve the other scions. Reuse every clean
> encode. Name the join you must recut (the kerf). Never recut the tree.

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

graft succeeds when a fresh local workspace can:

1. preserve one concept and multiple inherited scions as Git-tracked recipes;
2. show and merge semantic changes by slot and layer without merging media;
3. compile synchronized video and audio while reusing unchanged slot resources;
4. tie a shipped build to its exact scion, destination, action provenance, and
   time map;
5. ingest a platform signal, resolve it to the intended slot and kerf, and fork
   a new scion without guessing the replacement creative;
6. exchange a supported OTIO/NLE subset with an explicit loss report; and
7. run the same compiler locally and against a server store without a second IR.

## Current state

The repository implements the local workspace loop on schema `0.2.0`: multiple
tracked scions, layer flattening, semantic diff/merge, exact build
provenance, declared-window feedback, synchronized audio on the ffmpeg path,
scoped OTIO interchange, local preview, and object-store transport. The
compiler is still sequential. It does not remotely execute, lock projects, or
round-trip NLE effects.

## Non-mission

graft is not a DAM, a project lock, a review player, an ad publisher, or a
replacement for an editor. The north star is local, signal-addressed creative
iteration with surgical rebuilds, not "git for video."
