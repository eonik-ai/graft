# README assets

`landing.cast` is the source recording for the CLI demo. `landing.gif` is
the GitHub-compatible rendering.

The recording runs the actual release binary against generated H.264 clips:

1. compile `hook + body + cta` to `ad.mp4`;
2. bind `hook-v2.mov`;
3. run `graft dirty`, which reports `hook` dirty and `body` / `cta` clean.

Regenerate both files from the repository root:

```sh
brew install ffmpeg asciinema agg
make readme-demo
```

The recording scripts are [`../../scripts/landing-setup.sh`](../../scripts/landing-setup.sh),
[`../../scripts/landing-session.sh`](../../scripts/landing-session.sh), and
[`../../scripts/record-landing.sh`](../../scripts/record-landing.sh).
