# 0003 — adapters are guests; lossless NLE round-trip is a non-goal

- Status: accepted
- Date: 2026-09-13

## Context

A universal composition system is often sold as "lossless adapters for
every NLE." OTIO's adapter matrix still cannot carry most effects. AAF
spent decades on that problem. CapCut has no public edit/render API;
on-disk drafts are reverse-engineered and version-fragile.

OpenUSD did not become Maya. It compiles out, and imports what Maya can
express.

## Decision

graft's IR is native. OTIO, FCPXML, MLT, IMF, CapCut JSON, Resolve via
OTIO/FCPXML are **guests**. Loss is documented in `docs/adapters.md`.
Slots may flatten to markers on export.

A claim of lossless round-trip requires an RFC and a fixture. The default
is loss.

## Consequences

Core releases are not blocked on Premiere/CapCut/Resolve. An adapter bug
is not an IR bug. CapCut lock-and-key collab is out of scope.
