# Security policy

## Supported versions

| Version | Supported |
| --- | --- |
| spec `0.1.x` (this tree) | yes |
| unreleased compiler | not shipped; report design issues as RFCs |

When a compiler exists, only the latest minor on the current major receives
fixes. Pre-1.0, that is the latest `0.x`.

## Report a vulnerability

**Do not open a public issue.**

Use GitHub's **private vulnerability reporting** on this repository
(Security → Report a vulnerability). Triage is
[@techievena](https://github.com/techievena) on behalf of eonik.
If that UI is unavailable, email [connect@eonik.ai](mailto:connect@eonik.ai)
with subject `[graft security]`.

Conduct reports use the same private path, subject or title `[graft conduct]`.

## What we will do

- Acknowledge within **3 days**.
- Triage within **7 days** (in-scope / out-of-scope, severity).
- Coordinated disclosure: we aim to ship a fix or a public advisory within
  **90 days**. We will not publish an exploit.

## Scope

In scope, once code exists:

- path traversal or hash confusion in the CAS
- cache-key collisions that cause a wrong slot to be concatenated
- untrusted score/scion documents that crash or execute code
- adapter parsers that execute embedded project-file scripts

Out of scope:

- "someone with write access to the score can change the video" — that is
  the point of a compiler
- encoder quality / bitrate
- NLE bugs in guest adapters
- social engineering of maintainers

## Threat model (north star)

The score is trusted like source code. Essence is untrusted bytes addressed
by hash. A compile must never substitute essence that does not match the
binding hash. Two machines with the same encoder fingerprint and the same
scion hash must produce concat-compatible slot encodes.
