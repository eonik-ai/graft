# RFC 0001 — workspace schema and provenance

- Status: accepted
- Date: 2026-09-14
- Implements: ADR 0007
- Format id: `0.2.0`

## Problem

Schema `0.1.0` could describe one score and one overwritten `scion.json`.
Its layer list was descriptive rather than a set of binding opinions, time was
stored as floating-point seconds, and feedback was not tied to the exact build
that was shipped. Changing those identities after release would silently change
scion and action hashes.

## Decision

Format id `0.2.0` is the first workspace schema:

1. One root `score.json` per concept and tracked `scions/<id>.json` documents.
   A scion may name one parent. Git commits remain temporal history.
2. Ordered layer opinions on the scion override bindings and parameters.
   Flattening is deterministic strongest-layer-wins.
3. Source and destination ranges are rational/frame-based
   (`{rate:{num,den}, range:{start,duration}}`).
4. Immutable build identity ties the flattened scion, destination, action
   identity, output, and exact compiled time map.
5. Feedback documents preserve the raw platform signal, reference that build,
   and store resolved slots/kerfs separately.
6. A signal kind addresses the declared window through the exact build time
   map. Callers do not have to restate hidden `0-3` policy.
7. Validation requires unique IDs, acyclic parentage, explicit spine/track
   roles, compatible binding durations, and action keys that include slot
   identity so distinct slots cannot collide.

`0.1.0` documents are not reinterpreted in place. `graft migrate` rewrites
them and keeps backups. Fixtures live in `fixtures/migration-0.1.0/`.

## Compatibility

- Recipe, essence, and build storage stay distinct.
- Signal dirtiness stays separate from action-cache dirtiness.
- Guest adapters remain lossy and depend on `graft-score` only.
