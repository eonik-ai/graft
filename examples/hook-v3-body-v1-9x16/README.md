# hook_v3 + body_v1 + cta_v1 @ 9x16

North-star fixture. Previous ship was `hook_v2` on the same body and cta.

Score clock 23s @ 30fps. Dest `9x16` = 1080×1920, x264 `keyint=30`,
`sc_threshold=0`. Hook window is **declared** as 3s.

| Slot | Score span | Binding | After hook_v3 |
| --- | --- | --- | --- |
| hook | `[0, 3)` | hook_v3 `in=0.4 out=3.4` | encode MISS |
| body | `[3, 20)` | body_v1 | HIT |
| cta | `[20, 23)` | cta_v1 | HIT |
| kerf hook→body | join at 3.0s | — | MISS |
| kerf body→cta | join at 20.0s | — | HIT |

`dirty.json` is the expected result of

```text
graft signal --kind hook_rate --t 0-3
```

`make test` fails if that result ever includes `body`.
