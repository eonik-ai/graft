# hook_v3 + body_v1 + cta_v1 @ 9x16

North-star fixture. Previous ship was `hook_v2` on the same body and cta.

Score clock 23s @ 30fps (690 frames). Dest `9x16` = 1080×1920, x264
`keyint=30`, `sc_threshold=0`. Hook window is **declared** as 90 frames.

| Slot | Score frames | Binding | After hook_v3 |
| --- | --- | --- | --- |
| hook | `[0, 90)` | hook_v3 start 12 duration 90 | encode MISS |
| body | `[90, 600)` | body_v1 | HIT |
| cta | `[600, 690)` | cta_v1 | HIT |
| kerf hook→body | join at frame 90 | — | MISS |
| kerf body→cta | join at frame 600 | — | HIT |

`dirty.json` is the expected result of

```text
graft signal --kind hook_rate --build <shipped-build>
```

`make test` fails if that result ever includes `body`.

To **encode** this clock on generated media (no files in git):

```sh
make example-encode
```
