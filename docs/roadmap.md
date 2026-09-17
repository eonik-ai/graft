# Roadmap

The first compiler GitHub tag is `v0.2.0`. Do not reuse spec tag
`v0.1.0`. The `graft` field on JSON is a **format id** (`0.2.0` today),
not a crate version. crates.io stays unpublished (`publish = false`).

## Founding release gate

A fresh local workspace must complete:

> concept → scions → team iteration → shipped build → platform signal →
> addressed slot → new scion

The **IR** for that loop is tagged `v0.2.0`. Overlay roles, named fades,
and HTTP/S3 store transport are tagged `v0.2.1`. That tag is user-false
without porcelain: eight plumbing verbs, `iterate` with empty layers, and
plan-only examples. ADR 0009 closes the **user** loop on the same schema:
`ship` / `swap` / `address` plus a reuse ledger. crates.io crate name
`graft` is taken (orbitinghail storage engine), so crates stay
`publish = false`. Distro listings wait on each community
([install.md](install.md), [packaging.md](packaging.md)).

## Implemented

- Mission, principles, accepted ADRs, RFC 0001, normative `0.2.0` schemas.
- Tracked `scions/<id>.json`, parent inheritance, real layer opinions.
- Rational/frame time and `graft migrate` from `0.1.0`.
- Semantic diff and three-way merge; porcelain-light `graft status`.
- Build records, exact time maps, feedback ingest, `graft iterate`.
- Sequential compiler: `graft-intra`, closed-GOP x264, synchronized AAC,
  applied `params.speed`, Long-GOP kerf fill for non-IDR joins,
  `ActionResult` provenance, concat verification, object-store sync.
- Scoped OTIO export/import with a machine-readable loss report.
- Local decode-and-composite preview with synced audio and overlay mix.
- Named fade kerfs; HTTP/S3 object-store transport (same compiler).
- Porcelain user loop (ADR 0009): `takes/` on-ramp, `graft ship`,
  `graft swap` (single take or `--from` pool), `graft address`, reuse
  ledger on every encode. `swap` fails if a sibling `slot_encode` recodes.

## Still later / never in core

- Arbitrary-source mid-GOP repair
- Premiere/Avid/Resolve/CapCut as first-class lossless integrations
- Review UI, DAM, storage mount, project lock
- Remote execution or a cloud-only IR
- A graft replacement for Git commits and branches
- Auto-publish to ad platforms
