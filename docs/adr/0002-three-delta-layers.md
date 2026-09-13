# 0002 — three delta layers, not git-on-mp4

- Status: accepted
- Date: 2026-09-13

## Context

"Git for video" usually means: put an mp4 in git or rsync/xdelta two
exports. H.264 Long-GOP is already a delta encoding. CABAC means a small
picture change rewrites the slice. Two x264 encodes of the same pictures
are different bytes. Content-defined chunking on transcoded `mdat` usually
finds nothing.

IMF already splits recipe (CPL) from essence (Track Files) and ships
supplementals that reuse unchanged UUIDs.

## Decision

graft has three layers that must not be mixed:

- **A. Recipe** — score / scion. Text. git.
- **B. Essence** — immutable CAS materials. Relink by hash.
- **C. Build** — `slot_encode` + `kerf` + concat. Encoder fingerprint
  is part of the key.

Byte-delta of a delivery mp4 is a non-goal. The shipped file is never
essence.

## Consequences

Features that "version the export" are rejected. Cache hits are proven
with hashes of slot encodes, not with `diff` of two mp4s.
