# Packager recipes

These files are **source recipes**. They are how graft enters the
repositories indexed on [pkg.bot/repos](https://pkg.bot/repos). They are
not a claim that graft is already listed there. Each distro reviews
packages on its own schedule.

Users who want a binary today: [install.md](../docs/install.md).
GitHub Releases attach tarballs, a musl Linux build, and `.deb` files.
The GitHub tag archive hashes are in [`SOURCE`](SOURCE).

| Recipe | Family (pkg.bot) | Submit |
| --- | --- | --- |
| [`../Formula/graft.rb`](../Formula/graft.rb) | Homebrew (not on pkg.bot) | this repo is the tap |
| [`macports/Portfile`](macports/Portfile) | MacPorts | [macports-ports](https://github.com/macports/macports-ports) |
| [`../flake.nix`](../flake.nix) | nixpkgs | [NixOS/nixpkgs](https://github.com/NixOS/nixpkgs) |
| [`aur/PKGBUILD`](aur/PKGBUILD) | AUR | `aur publish` |
| [`alpine/APKBUILD`](alpine/APKBUILD) | Alpine apk | [aports](https://gitlab.alpinelinux.org/alpine/aports) |
| [`debian/`](debian/) | Debian / Ubuntu / Devuan / Kali / Trisquel / PureOS / Raspbian / Parrot / Pardus / deepin / openKylin / Apertis | Debian ITP, then syncs |
| [`rpm/graft.spec`](rpm/graft.spec) | Fedora / EPEL / RHEL clones / openSUSE / Mageia / OpenMandriva | Fedora review + OBS |
| [`void/template`](void/template) | Void | [void-packages](https://github.com/void-linux/void-packages) |
| [`gentoo/`](gentoo/) | Gentoo / GURU / LiGurOS | [gentoo](https://github.com/gentoo/gentoo) or GURU |
| [`freebsd/Makefile`](freebsd/Makefile) | FreeBSD Ports | [ports](https://github.com/freebsd/freebsd-ports) |
| [`openbsd/Makefile`](openbsd/Makefile) | OpenBSD Ports | `ports/` CVS |
| [`slackbuilds/graft.SlackBuild`](slackbuilds/graft.SlackBuild) | SlackBuilds | [slackbuilds.org](https://slackbuilds.org/) |
| [`solus/package.yml`](solus/package.yml) | Solus | [solus-packages](https://github.com/getsolus/packages) |
| [`openwrt/Makefile`](openwrt/Makefile) | OpenWrt | [openwrt/packages](https://github.com/openwrt/packages) |
| [`nix/package.nix`](nix/package.nix) | nixpkgs | [NixOS/nixpkgs](https://github.com/NixOS/nixpkgs) |
| [`pkg.bot-repos.md`](pkg.bot-repos.md) | all 129 | mapping, not a listing |

Every pkg.bot repository is listed in [`pkg.bot-repos.md`](pkg.bot-repos.md).
OpenWrt: [`openwrt/Makefile`](openwrt/Makefile). Nixpkgs-shaped
derivation: [`nix/package.nix`](nix/package.nix).

Until those land, the same users install from GitHub Releases (`.tar.gz` /
`.deb`) or `cargo install --git`. That is the supported path. Do not file
a hundred identical ITPs; one Debian ITP covers the debuntu family.
