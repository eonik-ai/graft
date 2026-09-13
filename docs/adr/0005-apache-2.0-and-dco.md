# 0005 — Apache-2.0 and DCO, no CLA

- Status: accepted
- Date: 2026-09-13

## Context

graft is infrastructure: a schema others will implement, eventually a
compiler. MIT is simpler. Apache-2.0 adds an explicit patent grant, which
matters if this IR is embedded in commercial NLEs or finishing tools.
A CLA slows first-time contributors and is unnecessary under Apache §5.

The Linux kernel's DCO is enough provenance for git-like tooling.

## Decision

- Outbound license: **Apache License 2.0** (this tree, including schema
  and documentation).
- Inbound: same license, via Apache §5.
- Every commit: **Developer Certificate of Origin** (`Signed-off-by`).
- No CLA. No copyright assignment. Contributors keep copyright on their
  patches. That is how a company-copyrighted Apache project stays open:
  eonik owns the original work; the community licenses new work in.
- Copyright notice: **eonik** ([eonik.ai](https://www.eonik.ai/),
  [github.com/eonik-ai](https://github.com/eonik-ai)), year of first
  publication 2026.

## Consequences

Downstream products may use graft in closed tools (Apache permits that).
They do not get to relicense the IR. Trademark of the name `graft` is
not registered; see GOVERNANCE.md.
