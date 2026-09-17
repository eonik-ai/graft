# ship → swap on generated takes

JSON fixtures under [`hook-v3-body-v1-9x16/`](../hook-v3-body-v1-9x16/) and
[`dub-en-9x16/`](../dub-en-9x16/) are dirty-set contracts. They do not
contain media.

This example is the **user** walk: named files in `takes/`, `graft ship`,
`graft swap`. No clips are stored here.

```sh
# from the graft repo root
make demo          # ship, swap, address, swap; three dests
make walkthrough   # labeled 9:16 plates + side-by-side GIF
make example-encode  # 3s + 17s + 3s clock (north-star durations)
```

Put your own files in `takes/`:

```
takes/hook.mov
takes/body.mov
takes/cta.mov
takes/vo.wav          # optional overlay
takes/hooks/v2.mov    # not bound on ship
```

```sh
graft ship --out ad.mp4
graft swap hook ./takes/hooks/v2.mov --out ad-v2.mp4
```

See [docs/loop.md](../../docs/loop.md) and [docs/walkthrough.md](../../docs/walkthrough.md).
