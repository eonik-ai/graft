# crates

Canonical map: [docs/architecture.md](../docs/architecture.md).

```
graft → graft-compile → graft-cas → graft-score
graft → graft-otio → graft-score
```

| Crate | Start here |
| --- | --- |
| `graft-score` | IR. Schema wins. |
| `graft-cas` | Namespaced blobs + action cache. `ActionKey` ≠ `BlobId`. |
| `graft-compile` | Flatten → graph → schedule. Intra + ffmpeg/x264. Signal dirty-set. |
| `graft-otio` | Lossy OTIO guest. Depends on score only. |
| `graft` | CLI only. |

Do not add another core crate. Adapters stay separate and depend on
`graft-score` only.
