# 0009 — porcelain closes the founding loop

- Status: accepted
- Date: 2026-09-15
- Extends: ADR 0007 (does not edit its Decision)

## Context

ADR 0007 restored the product boundary: graft is a local-first composition
workspace whose incremental compiler is a subsystem. Schema `0.2.0` and the
`v0.2.0` tag made that loop **describable** — tracked scions, exact builds,
declared-window signals, `iterate` as an empty change request.

A stranger still could not **do** the job. The metaphor never became a verb.
The founding walk was eight plumbing commands. `iterate` stopped at
`creative_replacement: required`. Compile printed a plan, not “body kept.”
In-repo examples cannot encode. That is IR-true and user-false.

Nearby tools that close a loop with two commands solve a different job
(numeric hill-climb on a codebase). graft’s physics is surgical variants:
name a slot, recode it and its kerf, bitstream-copy the rest.

## Decision

1. **Plumbing stays.** `init`, `slot`, `scion`, `bind`, `compile`, `signal`,
   `iterate`, `dirty` remain the verbs the porcelain calls. They do not go
   away and they do not grow a second IR.
2. **Porcelain is the user loop.** `graft ship` turns a `takes/` folder into
   the first dest. `graft swap <slot> <file>` forks (or binds an addressed
   child), compiles, and **fails the run** if a sibling `slot_encode` recoded
   when the flattened delta is only that slot. `graft address` resolves a
   named signal onto an exact build, iterates an empty child, and names the
   next `swap`. Batch `swap --from` compiles a take pool against one body.
3. **The reuse ledger is how magic is proven.** Every encode compile prints
   dirty slots, clean slots with stable BlobIds, encode wall-clock, dest
   path, and build id. A web dashboard, review player, or experiment host is
   out of scope.
4. **Takes are named files, not inferred pictures.** `takes/hook.mov` binds
   `hook`. Unknown files are listed, not guessed. The compiler still does
   not infer roles from pixels or silence.
5. **Creative replacement stays human (or a guest agent).** Porcelain does
   not invent a take, call a generator, or ingest live ad platforms.

## Consequences

- Compiler work is evaluated by a walkable ship → swap → address loop, not
  only by JSON dirty-set fixtures.
- `v0.2.0` remains the workspace/compiler tag. Porcelain is subsequent work
  on the same schema.
- Load-bearing CLI test: `swap` of a hook on generated media keeps the body
  BlobId. Do not delete it to make a change pass.
- Agents may wrap ship / swap / address. They are users of porcelain, not a
  runtime inside graft.
