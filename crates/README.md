# crates

Canonical map: [docs/architecture.md](../docs/architecture.md).

```
graft → graft-compile → graft-cas → graft-score
```

| Crate | Start here |
| --- | --- |
| `graft-score` | IR. Schema wins. |
| `graft-cas` | Namespaced blobs + action cache. `ActionKey` ≠ `BlobId`. |
| `graft-compile` | Flatten → graph → schedule. Intra + ffmpeg/x264. Signal dirty-set. |
| `graft` | CLI only. |

Do not add a fifth core crate. Adapters are separate and depend on
`graft-score` only.
