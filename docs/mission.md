# Mission

graft is a compiler for video composition.

The score is source.
Essence is immutable.
The mp4 is a compile.

You graft a new hook. The body stays.

## The job

A team has **one concept**. They ship many **scions**: hook takes, aspect
ratios, languages, legal cuts. Platforms then return a signal on a *time
range of the shipped file* ("hook rate, 0–3s", "too slow around 8s").

Today that signal addresses pixels. It should address a **slot**.

graft's job is to make this sentence true:

> Change only the dirty slots. Reuse every clean encode. Name the join
> you must recut (the kerf). Never recut the tree.

## Why a compiler, not a VCS for video

git versions text. The score is text. Use git.

Pixels are not text. A delivery H.264 file is already a delta encoding
(CABAC). Byte-diffing two exports of the "same" cut misses. The unit of
reuse is a **cached slot encode** (and, at Long-GOP, a cached kerf GOP),
addressed by a key that includes the encoder fingerprint.

That is how IMF already ships masters (CPL + Track Files, supplemental
IMP). graft is that algebra for *authoring*, with named slots and a time
map so a metric can dirty a node.

## Success

graft is succeeding when:

1. `hook_v3 + body_v1 + cta_v1 @ 9x16` after `hook_v2` re-encodes the
   hook and the hook→body kerf, and bitstream-copies the body.
2. `graft signal --kind hook_rate --t 0-3` prints `{hook}` plus that kerf,
   not the body, because the hook window was declared as 3s.
3. A Resolve or CapCut user can **export out** and **import in** with an
   honest loss matrix — slots may flatten to markers — and the score in
   graft remains the source.
4. The same compiler runs on a laptop (device-first) and behind an object
   store (server). There is not a second IR for the cloud.

## Non-mission

graft is not a DAM, a project lock, a review player, or a replacement for
an editor. Those are products. This is the IR and the compile.

The north star is surgical variants, not "git for video."
