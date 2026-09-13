# 0004 — device-first; server is the same compiler

- Status: accepted
- Date: 2026-09-13

## Context

Teams iterate on laptops. Servers are useful for CAS and encode farms.
Two IRs (a "cloud graph" and a "local graph") will diverge. Preview does
not need GOP delta: it is decode + composite, like every NLE.

## Decision

The device holds the authoritative score and a local CAS. `graft compile`
runs there. A server, if any, runs **the same compiler** against an object
store of materials, slot_encodes, and kerfs.

No cloud-only objects in `schema/`. No "preview codec" in the IR — dest
and encoder fingerprint already name the bytes.

## Consequences

0.2–0.5 of the roadmap are local. Server is an operator of the cache,
not a product fork.
