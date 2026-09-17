# User loop

Ship the next cut without recutting the tree. Drop a new hook, VO, or dest.
The clean slots stay. You play both.

The IR for this loop is [mission](mission.md) and [ADR 0007](adr/0007-compiler-is-a-workspace-subsystem.md).
The verbs a person (or an agent) actually runs are porcelain on top of
plumbing. See [ADR 0009](adr/0009-porcelain-closes-the-founding-loop.md).
Flags live in [README.md](../README.md).

## Takes

A take directory is named files, not scene detection:

```
takes/hook.mov
takes/body.mov
takes/cta.mov
takes/vo.wav           # optional overlay; spans the whole dest
takes/hooks/v2.mov     # pool for later swaps; not bound on ship
```

Unknown names are listed and ignored. graft does not infer `hook` / `body`
from pixels.

## Porcelain

```
takes/  →  graft ship     →  first dest + reuse ledger
                │
                ├──────── →  graft swap hook file   →  next dest, body HIT
                │
                └──────── →  graft address --kind hook_rate --build <id>
                                   →  empty child + “graft swap hook <file>”
```

`swap --from` compiles every take in a folder against the same parent so
the body encode is paid once.

Plumbing (`scion fork`, `bind`, `compile`, `signal`, `iterate`) is still
there. Porcelain calls it.

## Ledger

A compile that encoded prints dirty slots, kerfs, clean slots with BlobIds,
encode wall-clock, dest path, and build id. `ship` fails if it cannot
encode a dest. `swap` fails if a sibling `slot_encode` recoded when only
that slot changed.
