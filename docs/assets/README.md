# README assets

`landing.cast` is the source recording for the CLI demo. `landing.gif` is
the GitHub-compatible rendering.

The recording runs the actual release binary against generated H.264 clips:

1. `graft ship --out ad.mp4` from `takes/hook.mov`, `body.mov`, `cta.mov`;
2. `graft swap hook ./takes/hooks/v2.mov --out ad-v2.mp4`;
3. the reuse ledger reports `hook` dirty and `body` / `cta` clean.

Regenerate both files from the repository root:

```sh
brew install ffmpeg asciinema agg
make readme-demo
```

A local playable walk (no recording tools) is `make demo`: ship, swap a
hook, address `hook_rate`, swap again. Three dests must exist.

`make walkthrough` writes labeled 9:16 plates and
[`walkthrough.gif`](walkthrough.gif): dest v1 beside dest v2 after a hook
swap. The mp4 is gitignored.

The recording scripts are [`../../scripts/landing-setup.sh`](../../scripts/landing-setup.sh),
[`../../scripts/landing-session.sh`](../../scripts/landing-session.sh), and
[`../../scripts/record-landing.sh`](../../scripts/record-landing.sh).
