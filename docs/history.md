# Two histories

graft is not a video VCS. It keeps **two** histories on purpose, and a
third store that is not history at all.

| Store | What it records | Tool |
| --- | --- | --- |
| Recipe revisions | `score.json`, `scions/*.json`, `feedback/*.json` | **Git** |
| Variant derivation | `scion.parent`, layers, semantic diff/merge | **graft** |
| Bytes | CAS materials, action cache, `.graft/builds/` | **not Git** |

Git answers “what changed in the text, in order.” graft answers “which
named slot on which derived scion.” Mixing those into one “file version”
is how “git for video” fails. See [ADR 0007](adr/0007-compiler-is-a-workspace-subsystem.md)
and [principle 1–5](principles.md).

`.graft/` is local (CAS, action cache, build records, `HEAD`). Essence
(`.mp4` `.mov` `.mxf`) is never a recipe. Both are gitignored.

## Founding loop as inspectable artifacts

```
concept → scions → git commit of recipes → shipped build + time map
   ▲                                                │
   └── new scion ← iterate ← addressed slot ← signal
```

Walk one concept with placeholder material hashes (`compile` prints a
**plan**; no media). The CLI test
`founding_loop_history_is_not_an_mp4` is this walk. Porcelain (`graft ship`,
`graft swap`, `graft address`) calls the same verbs and still keeps `.graft/`
and dest files out of Git.

1. `graft init`, declare slots, `scion create picture`, `bind` hashes.
   `git add` the recipe files. `graft status` lists those files and does
   not replace `git commit`.
2. `graft compile --scion picture` writes `.graft/builds/<id>/build.json`
   and a time map. Git’s index does not change. A later fork does not
   rewrite that build.
3. `graft scion fork picture hook-v2` then `bind` only hook. The child
   document has `"parent": "picture"` and an empty inherited body. That
   parent is **not** the Git commit parent (`git log -1 --format=%P`).
   `graft diff picture hook-v2` is `slots.hook.binding` only.
4. Fork a second child that binds body. `graft merge picture hook-v2
   body-v2 --id merged` is conflict-free. `diff merged hook-v2` names
   body; `diff merged body-v2` names hook.
5. `graft signal --kind hook_rate --build <picture-build>` dirties hook,
   not body. `graft iterate` writes a child with `parent`,
   `change_request.slots`, **empty `layers`**, and no `params.speed`.
   Someone still `bind`s the next take. Commit the new scion and
   feedback JSON. `git log --name-only` never lists `.graft/` or an mp4.

## Dirty-set fixtures are not films

[`examples/hook-v3-body-v1-9x16/`](../examples/hook-v3-body-v1-9x16/) is
the load-bearing `hook_rate` contract: the declared window must not dirty
`body`.

[`examples/dub-en-9x16/`](../examples/dub-en-9x16/) is the same kind of
contract on a 10s clock (inherit, `vo_hold`, explicit `note`, `hold`).
It is **not** a dubbed film. RFC 0002 mixes `vo` over spine AAC; it does
not replace picture audio. Bindings are BLAKE3 ids of local essence so
the compiler can match hashes. The mp4s stay out of git.

## Commands

| Command | History it touches |
| --- | --- |
| `git add` / `git commit` | Recipe documents |
| `graft status` | Lists recipe files Git should own |
| `graft scion fork` | Variant parentage (empty layers) |
| `graft diff` / `graft merge` | Semantic slot/layer delta |
| `graft compile` | Build id + time map under `.graft/` |
| `graft signal` / `feedback ingest` | Resolve a metric on an exact build |
| `graft iterate` | New scion: parent + change request, no creative |
| `graft ship` / `swap` / `address` | Porcelain: first dest, slot swap, primed iterate. Same three stores. |
