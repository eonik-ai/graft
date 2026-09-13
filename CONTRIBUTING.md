# Contributing to graft

Thank you. graft is spec-first: a wrong sentence in `docs/principles.md` or a
wrong dirty-set in `ref/` is worse than a missing encoder.

This project adopts the [Contributor Covenant](CODE_OF_CONDUCT.md).
Report vulnerabilities via [SECURITY.md](SECURITY.md), never via a public issue.

## DCO

Every commit must be signed off.

```
Signed-off-by: Your Name <you@example.com>
```

`git commit -s` adds it. By signing off you certify the
[Developer Certificate of Origin](https://developercertificate.org/) v1.1.

There is no CLA and no copyright assignment. eonik holds copyright on
the original work ([NOTICE](NOTICE)). You keep copyright on your
contributions; by signing off you license them under Apache-2.0.
Apache-2.0 §5 is the inbound license. Contributors are the project,
not guests.

## Prerequisites

- `git`
- Python 3.10+
- `make`

## Workflow

1. Fork (or branch, if you have write access).
2. Branch: `feat/<topic>`, `fix/<topic>`, `docs/<topic>`, `rfc/<n>-<topic>`.
3. Keep the change one idea. Schema, docs commentary, and the reference test
   that proves the new rule land in the same PR.
4. `make test` and `make lint` must pass.
5. Open a PR. Title is a [Conventional Commit](https://www.conventionalcommits.org/):
   `feat(schema): declare hook window on the slot`
6. Maintainers squash-merge. The PR title becomes the commit on `main`.

## What needs an RFC

Open a `rfc` issue **before** writing a PR if you change any of:

- objects in the score / scion / time-map schema
- the signal → dirty-set rule
- cache-key or kerf-key formula
- slot roles
- adapter loss matrix (new guest, or a claim of losslessness)
- license or governance

The template is `.github/ISSUE_TEMPLATE/rfc.yml`. An RFC is accepted when a
maintainer labels it `rfc:accepted`. Until there are three maintainers, the
project lead decides (see [GOVERNANCE.md](GOVERNANCE.md)).

## What does not need an RFC

- Typos, glossary clarifications that do not change meaning
- Tests that lock an already-written rule
- CI / Makefile plumbing
- Adapter *stubs* that only document loss (no new lossless claim)

## Documentation sync

| If you change | Also update |
| --- | --- |
| `schema/*.schema.json` | `docs/schema.md`, `CHANGELOG.md` Unreleased, a worked example if behaviour changed |
| signal → dirty rule in `ref/` | `docs/time-map.md`, the worked example `dirty.json` |
| cache-key formula | `docs/compile.md` |
| a principle or non-goal | `AGENTS.md` (agents read it) |
| intended CLI | `README.md` command surface |
| an ADR | never rewrite history; add a new ADR that supersedes |

## Commit messages

```
<type>(<scope>): <summary>
```

Types: `feat` `fix` `docs` `test` `refactor` `chore` `ci` `build`.
Scopes: `schema` `ref` `docs` `examples` `adr` `meta`.

Breaking schema changes: `feat(schema)!: drop span tuples for {start,end}` plus a
`BREAKING CHANGE:` footer. Pre-1.0 this is still a minor version bump, but the
changelog must say it broke.

## Code (when it exists)

Implementation language for the compiler and CLI is **Rust**. Do not add a
second core language. Adapters may be in anything; they live behind a loss
matrix and must not leak into `ref/` or `schema/`.

Do not commit essence (`.mov` `.mp4` `.mxf`). Hashes and JSON only.

## Review

- One maintainer approval to merge, until GOVERNANCE says otherwise.
- Spec PRs: the north-star test still has to say that `hook_rate` on `[0, 3)`
  does not dirty `body` in the worked example, unless the RFC explicitly
  changed that rule.
