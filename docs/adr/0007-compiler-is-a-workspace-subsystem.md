# 0007 — the incremental compiler is a workspace subsystem

- Status: accepted
- Date: 2026-09-14

## Context

graft was narrowed in its early implementation to a typed composition graph
and incremental compiler. That compiler is necessary, but it is not the whole
founding job.

The founding loop is:

> concept → scions → team iteration → shipped build → platform signal →
> addressed slot → new scion

Git can preserve recipe documents and their temporal history, but it does not
understand scions, layer precedence, video-aware conflicts, shipped-build time
maps, or which slot a platform signal addresses. Conversely, graft must not
grow a second commit database or confuse recipe history with media and build
storage.

ADRs 0002, 0003, 0004, and 0006 established the correct boundaries for the
three delta layers, guest adapters, device-first operation, and action-cache
compilation. This decision restores the product boundary around those
decisions rather than replacing them.

## Decision

graft is a local-first composition workspace whose incremental compiler is one
subsystem.

- **Git owns recipe history and transport:** commits, branches, merges of text,
  and collaboration through normal Git repositories.
- **graft owns composition semantics:** concepts, scions and inheritance,
  ordered layer opinions and flattening, semantic diff and conflict reporting,
  build provenance, exact signal-to-slot resolution, and the action graph used
  to compile a selected scion.
- **CAS owns immutable essence.** The build/action cache owns derived slot
  encodes, kerfs, concat products, and action results. These stores remain
  distinct from Git history and from each other as established by ADR 0002 and
  ADR 0006.
- **Adapters remain lossy guests.** OTIO and NLE formats may import or export a
  supported subset with an explicit loss report; they do not become graft's
  native IR.
- **Device-first remains authoritative.** A future server uses the same score,
  scion, flattening, signal, and compile code with a different store.

The native workflow must preserve multiple named scions without overwriting
one another, relate every shipped build and its exact time map to the compiled
scion, and let feedback identify a semantic change target without inventing
the replacement creative.

This ADR supersedes the compiler-only product boundary stated in
`docs/comparison.md`. It does not supersede any accepted ADR decision.

## Consequences

- Compiler work is evaluated as part of the end-to-end iteration loop, not as
  the complete product by itself.
- Git integration stays porcelain-light; graft may report semantic changes and
  conflicts but does not hide or replace Git commands.
- Schema work must define multiple scions, inheritance, real layer overrides,
  stable build provenance, feedback identity, and exact time before the first
  schema freeze.
- Signal dirtiness and action-cache dirtiness remain distinct: feedback says
  what should change; the cache says what must rebuild after it changes.
- Documentation and releases must distinguish implemented compiler capability
  from the intended workspace. The current implementation is not yet the full
  founding loop.

## Notes

2026-09-14: the founding loop is tagged `v0.2.0`. This note does not change
the Decision. Overlay actions and named fades are ADR 0008.
