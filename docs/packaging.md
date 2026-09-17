# Packaging

graft is Apache-2.0. The CLI is one binary plus a **runtime** ffmpeg.
It does not vendor or link x264.

## What this cut actually publishes

| Channel | Status |
| --- | --- |
| GitHub source (`main` + tag `v0.2.2`) | this repo |
| GitHub Releases | tag `v0.2.*` runs [release.yml](../.github/workflows/release.yml): macOS arm64 native, macOS x86_64 cross from `macos-latest`, Linux gnu arm64/x86_64, Linux musl x86_64, `.deb` |
| Homebrew tap | [`Formula/graft.rb`](../Formula/graft.rb) — `url`/`sha256` for `v0.2.2`; Homebrew 7: `brew trust --tap eonik-ai/graft` then `brew tap eonik-ai/graft https://github.com/eonik-ai/graft` |
| Nix flake | [`flake.nix`](../flake.nix) — `nix run github:eonik-ai/graft` |
| `scripts/install.sh` | matching Release tarball, else `cargo --git` |
| crates.io | **blocked**. Crate name `graft` is [orbitinghail/graft](https://github.com/orbitinghail/graft). Workspace stays `publish = false`. Install with `cargo install --git`. |

## What this repo does not claim

[pkg.bot/repos](https://pkg.bot/repos) indexes **129** distro repositories.
Every one is mapped in [`dist/pkg.bot-repos.md`](../dist/pkg.bot-repos.md).
graft is **not listed** there until each community accepts a recipe. A
file under [`dist/`](../dist/README.md) is not `dnf install graft`.

## Launch checklist

1. Porcelain loop is green: `make test && make lint && make demo`.
2. Docs: [install.md](install.md), [loop.md](loop.md), this page.
3. Merge to `main`.
4. Tag `v0.2.2` (or the next crate version). Do not reuse spec `v0.1.0`.
5. GitHub Actions attaches binaries, tarballs, `.deb`, `SHA256SUMS`.
   Intel macOS is `--target x86_64-apple-darwin` on `macos-latest` (do
   not wait on `macos-13`).
6. Homebrew formula `url`/`sha256` for that tag ([`dist/SOURCE`](../dist/SOURCE)).
7. Open **one PR per family** from `dist/` after the tag exists (they need
   a tarball SHA): Debian ITP, Fedora review, aports, nixpkgs, AUR,
   macports-ports, void-packages, GURU, FreeBSD, OpenBSD, SlackBuilds,
   Solus, OpenWrt packages.

## pkg.bot family → recipe

See [`dist/pkg.bot-repos.md`](../dist/pkg.bot-repos.md) for the 129-row
list. Family summary:

| Family | Manager | Recipe |
| --- | --- | --- |
| Homebrew | brew | `Formula/graft.rb` |
| MacPorts | macports | `dist/macports/Portfile` |
| nix / nixos | nix | `flake.nix` + `dist/nix/package.nix` |
| AUR / Arch / Manjaro / Artix / Parabola | aur / pacman | `dist/aur/PKGBUILD` |
| Alpine | apk | `dist/alpine/APKBUILD` |
| Debian and debuntu clones | apt | `dist/debian/` + Release `.deb` |
| Fedora / EPEL / Amazon / Terra / Rocky / Alma / CentOS / EuroLinux / openEuler / RPM Sphere | dnf / yum | `dist/rpm/graft.spec` |
| openSUSE / Mageia / OpenMandriva | rpm / zypper | same spec |
| Void | xbps | `dist/void/template` |
| Gentoo / GURU / LiGurOS | portage | `dist/gentoo/` |
| FreeBSD | pkg | `dist/freebsd/Makefile` |
| OpenBSD | ports | `dist/openbsd/Makefile` |
| SlackBuilds / Slackware | slackbuilds | `dist/slackbuilds/` |
| Solus | eopkg | `dist/solus/package.yml` |
| OpenWrt | opkg | `dist/openwrt/Makefile` |
| BioArch, BlackArch, ArchPOWER | — | GitHub binary / AUR; do not file those overlays |
