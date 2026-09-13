# Roadmap

The first compiler GitHub tag is `v0.2.0`. Do not reuse spec tag
`v0.1.0`. The `graft` field on JSON is a **format id** (`0.2.0` today),
not a crate version. crates.io stays unpublished (`publish = false`).

## Founding release gate

A fresh local workspace must complete:

> concept → scions → team iteration → shipped build → platform signal →
> addressed slot → new scion

That loop is implemented and tagged. Remaining honesty work is crates.io
still unpublished.

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
- Local decode-and-composite preview with synced audio.

## Still later / never in core

- Arbitrary-source mid-GOP repair
- Premiere/Avid/Resolve/CapCut as first-class lossless integrations
- Review UI, DAM, storage mount, project lock
- Remote execution or a cloud-only IR
- A graft replacement for Git commits and branches
- Auto-publish to ad platforms
