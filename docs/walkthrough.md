# Walkthrough: input takes → swapped dest

No media is stored in git. This page is the picture of the founding loop.
Generate the film on your machine:

```sh
make walkthrough
```

That script writes labeled 9:16 takes, runs `graft ship` then `graft swap`,
and builds two artifacts (mp4 is gitignored; the GIF is the README still):

1. **Inputs** — hook / body / cta as named color plates.
2. **Output v1** — `ad.mp4`, the first dest.
3. **Output v2** — `ad-v2.mp4` after `graft swap hook`. The hook plate
   changes. The body plate does not. The reuse ledger prints `clean body`
   with a stable BlobId. A bitstream copy of the body GOP inside both dests
   must match.

![Side-by-side dest: hook swapped, body kept](assets/walkthrough.gif)

The CLI session (typed `ship` / `swap`, reuse table) is still
[`assets/landing.gif`](assets/landing.gif).

Optional overlay: `takes/vo.wav` is a named take. `graft ship` binds it as
an overlay that spans the dest. A hook swap must not recode `body`.

Worked JSON examples stay plan-only. To encode the north-star clock on
generated media (not the placeholder hashes):

```sh
make example-encode
```
