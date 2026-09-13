## Summary

<!-- Conventional commit title on the PR. Maintainers squash-merge. -->

## Kind

- [ ] spec / schema (RFC issue #)
- [ ] reference test
- [ ] docs only
- [ ] meta (CI, license, templates)

## Checklist

- [ ] `make test` passes
- [ ] `make lint` passes
- [ ] docs sync table in CONTRIBUTING.md / AGENTS.md followed
- [ ] commits will be signed off (`git commit -s`); DCO
- [ ] no essence files (mp4/mov/mxf)

If this changes signal → dirty set, `test_hook_rate_does_not_dirty_body` still
passes **or** an accepted RFC explains why not.
