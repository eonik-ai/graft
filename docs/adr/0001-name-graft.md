# 0001 — the project is called graft

- Status: accepted
- Date: 2026-09-13

## Context

The work is a typed composition graph plus an incremental compiler for
video. Nearby names (vit, dits, "git for video") either collide or teach
the wrong physics. The product sentence is: change a hook, keep the body.

## Decision

The project and the command are **`graft`**, lowercase, like `git`.

Metaphor, held consistently:

| Word | Meaning |
| --- | --- |
| rootstock | concept / score |
| scion | variant: bindings + dest |
| slot | graft union (typed role) |
| material | CAS wood |
| kerf | millimetre lost at the join — the GOP you re-encode |
| compile | the take that lives |

Do not rebrand to a git-pun (vidgit, git-cut, vine).

## Consequences

CLI: `graft init | slot | bind | scion | compile | dirty | signal | export`.
Prose in this repo uses lowercase `graft` even at the start of a sentence
when referring to the command; "graft is…" is the README voice.
