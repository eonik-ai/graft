# Packaging

graft is Apache-2.0. The CLI is one binary plus a **runtime** ffmpeg.
It does not vendor or link x264.

A recipe in this tree is packager source. It is not a distro listing.
Users install from [install.md](install.md) until a distro ships the
package. See [principle 15](principles.md).

## Published install paths

| Channel | Where |
| --- | --- |
| GitHub source | `main` and tag `v0.2.2` |
| GitHub Releases | [release.yml](../.github/workflows/release.yml): macOS arm64 native, macOS x86_64 cross from `macos-latest`, Linux gnu arm64/x86_64, Linux musl x86_64, `.deb` |
| Homebrew tap | [`Formula/graft.rb`](../Formula/graft.rb) — `url`/`sha256` for `v0.2.2`; Homebrew 7: `brew trust --tap eonik-ai/graft` then `brew tap eonik-ai/graft https://github.com/eonik-ai/graft` |
| Nix flake | [`flake.nix`](../flake.nix) — `nix run github:eonik-ai/graft` |
| `scripts/install.sh` | matching Release tarball, else `cargo --git` |
| crates.io | **not this CLI**. Crate name `graft` is [orbitinghail/graft](https://github.com/orbitinghail/graft). Workspace stays `publish = false`. Install with `cargo install --git`. |

## Packager recipes

| Distro / channel | Manager | Recipe |
| --- | --- | --- |
| Homebrew | brew | `Formula/graft.rb` |
| MacPorts | macports | `dist/macports/Portfile` |
| Nix | nix | `flake.nix` + `dist/nix/package.nix` |
| Arch (AUR) | pacman | `dist/aur/PKGBUILD` |
| Alpine | apk | `dist/alpine/APKBUILD` |
| Debian / Ubuntu | apt | `dist/debian/` + Release `.deb` |
| Fedora / RHEL clones | dnf | `dist/rpm/graft.spec` |
| openSUSE / Mageia / OpenMandriva | rpm | same spec |
| Void | xbps | `dist/void/template` |
| Gentoo | portage | `dist/gentoo/` |
| FreeBSD | pkg | `dist/freebsd/Makefile` |
| OpenBSD | ports | `dist/openbsd/Makefile` |
| Slackware | slackbuilds | `dist/slackbuilds/` |
| Solus | eopkg | `dist/solus/package.yml` |
| OpenWrt | opkg | `dist/openwrt/Makefile` |

Submit through that community's process. Tag-archive hashes for
`v0.2.2` are in [`dist/SOURCE`](../dist/SOURCE).

## Cutting a tag

1. `make test && make lint && make demo`.
2. Docs: [install.md](install.md), [loop.md](loop.md), this page.
3. Merge to `main`.
4. Tag the crate version (`v0.2.2`, …). Do not reuse spec `v0.1.0`.
5. GitHub Actions attaches binaries, tarballs, `.deb`, `SHA256SUMS`.
   Intel macOS is `--target x86_64-apple-darwin` on `macos-latest`.
6. Pin the Homebrew formula `url`/`sha256` to that tag
   ([`dist/SOURCE`](../dist/SOURCE)).
