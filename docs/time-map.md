# Time map

A platform metric is a time range on the **shipped** file. A slot lives
on the **score clock**. The time map is the join.

```
dest_t  →  slot_id
```

It is an output of compile, stored next to the build under
`.graft/builds/<scion_hash>/time-map.json`. Today the map is identity
(score clock = dest clock). Retiming is not implemented; when it is, the
compiler regenerates this file. Without a time map, "hook is weak" cannot
address a node.

## Signal

```
signal := (kind, [t0, t1), dest_id)
```

Dirty slots = time-map entries whose span intersects `[t0, t1)`.

### hook_rate rule

`kind = hook_rate` **always** includes the slot with `role: hook`.

If the 3s window bleeds into body, **drop body** from the dirty set when
the overlap is strictly less than `spill_threshold_s` (default **0.35**).
Warn when overlap ≥ threshold: pad the hook to the declared window, or
accept a dirty body.

This is the rule that keeps a 2.8s creative hook from recutting the body
because Meta still scores 0–3s.

The window is **declared** on the score (`slot.window`). It is not
inferred from the binding's `out_s - in_s`.

## Worked map (23s @ 30fps, 9x16)

| Dest time | Slot | Typical signal |
| --- | --- | --- |
| `[0.00, 3.00)` | hook | hook_rate, thumbstop |
| `[3.00, 20.00)` | body | hold / "too slow" on 8–12s |
| `[20.00, 23.00)` | cta | click-through, end-card skip |

`graft signal --kind hook_rate --t 0-3` → slots `{hook}`, kerfs
`{hook→body}`. `body_v1` stays clean.

Reference implementation: [`ref/graft_ref/signal.py`](../ref/graft_ref/signal.py).
The test in `ref/tests/test_signal.py` is part of the spec.
