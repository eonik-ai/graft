# graft v0.2.2 — submission board

Living status. Update this file when a PR merges, an ITP gets a bug number, or a listing appears on [pkg.bot/repos](https://pkg.bot/repos).

Checked **2026-09-17**. Tag `v0.2.2`. Release: https://github.com/eonik-ai/graft/releases/tag/v0.2.2 (13 assets).

crates.io name `graft` is taken (orbitinghail). This CLI stays `publish = false`.

## Your actions (blocked on you)

Do these in this order. Each one unblocks a whole pkg.bot family. Paste-ready files are linked.

| # | You do | Unblocks | Ready file |
| --- | --- | --- | --- |
| 1 | From **Gmail** (`abinashsena@gmail.com`), send [`dist/debian/ITP`](../dist/debian/ITP) to `submit@bugs.debian.org`. Local postfix queued a copy as `abinash@Abinashs-MacBook-Air.local`; Debian will bounce that. Reply here with the BTS number (`#nnnnnn`). | Debian, Ubuntu, Devuan, Kali, Trisquel, PureOS, Raspbian, Parrot, Pardus, deepin, openKylin, Apertis | [`dist/debian/PUBLISH.md`](../dist/debian/PUBLISH.md) |
| 2 | Create a [Fedora FAS](https://accounts.fedoraproject.org/) account. Open a Package Review with [`dist/rpm/REVIEW-REQUEST.txt`](../dist/rpm/REVIEW-REQUEST.txt). | Fedora, EPEL, Rocky, Alma, CentOS Stream, Amazon, EuroLinux, openEuler, then openSUSE/Mageia/OpenMandriva from the same spec | [`dist/rpm/PUBLISH.md`](../dist/rpm/PUBLISH.md) |
| 3 | Create an [AUR account](https://aur.archlinux.org/register), add an SSH public key, then run the commands in [`dist/aur/PUBLISH.md`](../dist/aur/PUBLISH.md). | AUR → Arch, Manjaro, Artix, Parabola | [`dist/aur/PUBLISH.md`](../dist/aur/PUBLISH.md) |
| 4 | Create a [gitlab.alpinelinux.org](https://gitlab.alpinelinux.org) account, fork `alpine/aports`, add `community/graft/` from [`dist/alpine/`](../dist/alpine/). | Alpine edge + stables | [`dist/alpine/PUBLISH.md`](../dist/alpine/PUBLISH.md) |
| 5 | Apply as a [GURU contributor](https://wiki.gentoo.org/wiki/Project:GURU), then open a PR from https://github.com/techievena/guru/tree/graft-0.2.2 | Gentoo overlay GURU, LiGurOS | [`dist/gentoo/PUBLISH.md`](../dist/gentoo/PUBLISH.md) |
| 6 | OpenBSD ports needs a ports CVS login. Recipe is ready. | OpenBSD Ports | [`dist/openbsd/PUBLISH.md`](../dist/openbsd/PUBLISH.md) |
| 7 | Optional: Homebrew core. Tap already works. PR https://github.com/Homebrew/homebrew-core/pull/311719 is open; remaining is a local `brew install --build-from-source` so they will bottle. | brew-core bottles | tap is already live |

Do **not** file BioArch, BlackArch, or ArchPOWER overlays.

## Live today (users can install)

| Surface | How |
| --- | --- |
| GitHub Releases | https://github.com/eonik-ai/graft/releases/tag/v0.2.2 |
| `install.sh` | `curl -fsSL https://raw.githubusercontent.com/eonik-ai/graft/main/scripts/install.sh \| sh` |
| Homebrew tap | `brew trust --tap eonik-ai/graft && brew tap eonik-ai/graft https://github.com/eonik-ai/graft && brew install graft` |
| Nix flake | `nix run github:eonik-ai/graft -- --help` |
| cargo git | `cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft` |
| GitHub `.deb` | `graft_0.2.2_amd64.deb` / `_arm64.deb` on the Release |

## Filed, waiting on distro review (not you)

| Family | State | Link | Notes |
| --- | --- | --- | --- |
| nixpkgs | open, lint follow-up pushed | https://github.com/NixOS/nixpkgs/pull/564184 | treefmt + `__structuredAttrs` on the branch |
| MacPorts | open | https://github.com/macports/macports-ports/pull/34726 | |
| Void | open | https://github.com/void-linux/void-packages/pull/62574 | |
| OpenWrt | open, single Signed-off-by commit | https://github.com/openwrt/packages/pull/30553 | replaces #30552 (unsigned first commit) |
| Solus | open, review required | https://github.com/getsolus/packages/pull/10718 | |
| FreeBSD Ports | open | https://github.com/freebsd/freebsd-ports/pull/624 | |
| SlackBuilds | open | https://github.com/SlackBuildsOrg/slackbuilds/pull/17783 | |
| Homebrew core | open | https://github.com/Homebrew/homebrew-core/pull/311719 | formula must live at `Formula/g/graft.rb`; tap is the supported macOS path until they bottle |
| GURU | branch only | https://github.com/techievena/guru/tree/graft-0.2.2 | GitHub PRs need contributor access — see action 5 |

## Cannot list

| Surface | Why |
| --- | --- |
| crates.io `graft` | name taken by orbitinghail |
| pkg.bot row itself | appears after a parent distro **merges** |
| BioArch / BlackArch / ArchPOWER | overlays; install AUR or GitHub |

When a PR merges, mark the family **listed** here and in [`dist/pkg.bot-repos.md`](../dist/pkg.bot-repos.md) only after it shows on pkg.bot/Repology.
