# README assets

[`example-hook.png`](example-hook.png) and
[`example-body.png`](example-body.png) are frames from dest v1 | dest v2
after a real `graft ship` + `graft swap`. [`example.gif`](example.gif) is
the same pair, first four seconds. Essence (the takes and dest mp4s) is
not in git.

Hook / body / CTA were cut from a local 9:16 product clip. The swapped
hook is a second take of that product. Color tags on every take must
match (`yuv420p` / `bt709` / `tv`) or concat refuses the join.

```sh
HOOK=/path/to/clip.mov \
BODY=/path/to/clip.mov \
CTA=/path/to/clip.mov \
HOOK2=/path/to/other-hook.mp4 \
make readme-example
```

`make walkthrough` still writes labeled color plates for a no-footage
machine. Do not put those plates on the README.
