# Install graft

graft is a compiler for versioned ads. This page installs the reference CLI.
A host that embeds the compiler depends on the crates from this git remote
([hosts.md](hosts.md)).

Runtime: **ffmpeg** and **ffprobe** on `PATH` (or `$FFMPEG` / `$FFPROBE`).
graft shells out; it does not link GPL x264.

## macOS

Homebrew (this repository is the tap):

```sh
brew trust --tap eonik-ai/graft
brew tap eonik-ai/graft https://github.com/eonik-ai/graft
brew install graft
```

Homebrew 7 refuses unofficial taps until they are trusted. Stable is the
`v0.2.2` source tarball (`url`/`sha256` in the formula).
`brew install --HEAD graft` tracks `main`. ffmpeg is a dependency.
GitHub Releases also attach `aarch64-apple-darwin` and
`x86_64-apple-darwin` tarballs.

MacPorts: GitHub Releases attach `aarch64-apple-darwin` and
`x86_64-apple-darwin` tarballs until a port is in the tree.

## Linux

GitHub Releases attach:

- `graft-x86_64-unknown-linux-gnu.tar.gz`
- `graft-aarch64-unknown-linux-gnu.tar.gz`
- `graft-x86_64-unknown-linux-musl.tar.gz` (`install.sh` picks this on Alpine / musl)
- `graft_*_amd64.deb` / `graft_*_arm64.deb` (`Depends: ffmpeg`)

```sh
curl -fsSL https://raw.githubusercontent.com/eonik-ai/graft/main/scripts/install.sh | sh
```

Or:

```sh
sudo dpkg -i graft_*_amd64.deb
```

Nix (this flake):

```sh
nix run github:eonik-ai/graft -- --help
```

Packager recipes (AUR, Alpine, Fedora, Debian, Void, and others) live in
[`dist/`](../dist/README.md). A file there is not `dnf install graft`.
Install from this page until a distro ships the package.

## From source

Rust 1.85+ (`rustup`). ffmpeg on `PATH`.

```sh
cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft
graft --version
```

From a clone:

```sh
git clone https://github.com/eonik-ai/graft.git
cd graft
make test
cargo build --release --bin graft
```

crates.io crate name **`graft` is taken** (orbitinghail storage engine).
This project stays `publish = false`. Do not `cargo install graft` from
crates.io; that is a different program. Use `cargo install --git` above.

## First dest

```
takes/
  hook.mov
  body.mov
  cta.mov
```

```sh
graft ship --out ad.mp4
graft swap hook ./takes/hooks/v2.mov --out ad-v2.mp4
```

`graft ship` fails if it cannot encode. Play both files. The reuse ledger
names `hook` dirty and `body` / `cta` clean. Visual walk:
[walkthrough.md](walkthrough.md). Loop: [loop.md](loop.md).
