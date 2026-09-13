# Governance

graft is a spec-first open source project. The schema and the principles are
the product until a compiler exists. Governance exists so those do not drift
by accident.

## Current model: project lead

Until there are **three** named maintainers in [MAINTAINERS.md](MAINTAINERS.md),
graft is project-lead led:

- The project lead accepts RFCs, merges PRs, and cuts tags.
- Decisions happen in public (issues, RFCs, ADRs). Private chat is not a
  source of truth.
- A documented ADR outranks a conversation.

This is the honest form of "benevolent dictator for now." It is not a
foundation. It will change when the roster does.

## After three maintainers

- **Lazy consensus.** A maintainer may merge after 72 hours with no
  outstanding `changes-requested`, unless it is an RFC.
- **RFCs** (see [CONTRIBUTING.md](CONTRIBUTING.md)) need approval from at
  least two maintainers, including one who did not author the RFC.
- **ADRs** are merged only with RFC acceptance when they change a principle.
- **Releases** are tagged by a maintainer from `main`. Schema version in
  `$id` / `"graft"` fields matches the git tag (`v0.1.0` → `0.1.0`).

## What is in-bounds for core

- score, scion, dest, time map
- signal → dirty set
- cache keys (slot_encode, kerf)
- the reference tests in `ref/`
- eventual Rust compiler / CLI
- documented adapters with a loss matrix

## What is out of bounds for core

- Pixel merge, NLE UI, review/comment products, DAM, storage mounts
- A claim of lossless round-trip to CapCut, Premiere, Resolve, or Avid
- Byte-delta of a delivery mp4 as a feature
- Pitching graft as a replacement for a human editor

Out-of-bounds work can exist as *guest* adapters or downstream products.
It does not land in `schema/` or `ref/` without an RFC that amends
[docs/principles.md](docs/principles.md).

## Intellectual property

- **Copyright** of the original work is held by **eonik**
  ([eonik.ai](https://www.eonik.ai/), org
  [github.com/eonik-ai](https://github.com/eonik-ai)).
- **License** is Apache License 2.0 (outbound = inbound).
- **Contributors** are first-class. There is **no CLA** and **no
  copyright assignment**. You keep copyright on your patches. DCO
  (`Signed-off-by`) plus Apache §5 is how they enter the tree.
- The intended GitHub home is the [eonik-ai](https://github.com/eonik-ai)
  organisation. Project lead is listed in [MAINTAINERS.md](MAINTAINERS.md).

## Assets

- The command and project name is `graft` (lowercase, like `git`).
- No trademark is registered. Do not imply you speak for the project
  without maintainer agreement.

## Conflict of interest

eonik ships a Mac ad-finishing product. That product is a *user* of this
IR, not a privileged adapter and not a reason to close the tree. The IR
stays vendor-neutral: no NLE, including anything eonik ships, gets a
lossless claim without an RFC and a fixture.

A maintainer who works at eonik, or who ships a product on graft, must
say so on RFCs that touch adapters or the loss matrix.
