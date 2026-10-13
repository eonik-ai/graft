# graft v0.2.2 — submission board

Working notes. Not linked from README. Update when a PR merges, an ITP
gets a bug number, or a listing appears on [pkg.bot/repos](https://pkg.bot/repos).

Checked **2026-09-18**. Tag `v0.2.2`. Account `techievena`.
github.com/eonik-ai/graft is **0 stars / 0 forks / created 2026-09-13**.

**Freeze.** Do not push, force-push, comment, reopen, or open another
packaging PR. The 2026-09-17 burst (nine GitHub PRs in fifteen minutes)
is what looks like spam. GraphQL `HEAD_REF_FORCE_PUSHED_EVENT` counts on
those PRs were a false total (unfiltered timeline size). REST timelines
show **zero** force-pushes. The damage was volume, a duplicate OpenWrt
PR, Cursor trailers, and CI retriggers.

crates.io name `graft` is taken (orbitinghail). This CLI stays `publish = false`.

## Your actions (blocked on you)

Do **not** send the three mails while the GitHub spray is still warm.
Same account, same five-day-old repo. Wait until nixpkgs has a human
review or graft has actual users, then send in this order.

| # | You do | Unblocks | Ready file |
| --- | --- | --- | --- |
| 1 | **Send mail 1** from Gmail: [`dist/debian/MAIL.txt`](../dist/debian/MAIL.txt) to `submit@bugs.debian.org`. Plain text. Reply with the BTS `#nnnnnn`. | Debian, Ubuntu, Devuan, Kali, Trisquel, PureOS, Raspbian, Parrot, Pardus, deepin, openKylin, Apertis | [`dist/MAILS.md`](../dist/MAILS.md) |
| 2 | **Send mail 2** from Gmail: [`dist/openbsd/MAIL.txt`](../dist/openbsd/MAIL.txt) to `ports@openbsd.org`. No CVS login required to send. | OpenBSD Ports | [`dist/openbsd/MAIL.txt`](../dist/openbsd/MAIL.txt) |
| 3 | **Send mail 3** from Gmail: [`dist/gentoo/MAIL.txt`](../dist/gentoo/MAIL.txt) to `gentoo-guru@lists.gentoo.org`. Subscribe first if the list holds non-member posts. | GURU, LiGurOS | [`dist/gentoo/MAIL.txt`](../dist/gentoo/MAIL.txt) |
| 4 | Create a [Fedora FAS](https://accounts.fedoraproject.org/) account. Open a Package Review and paste [`dist/rpm/REVIEW-REQUEST.txt`](../dist/rpm/REVIEW-REQUEST.txt). This is Bugzilla, not mail. | Fedora, EPEL, Rocky, Alma, CentOS Stream, Amazon, EuroLinux, openEuler, then openSUSE/Mageia/OpenMandriva from the same spec | [`dist/rpm/PUBLISH.md`](../dist/rpm/PUBLISH.md) |
| 5 | Create an [AUR account](https://aur.archlinux.org/register), add an SSH public key, then run [`dist/aur/PUBLISH.md`](../dist/aur/PUBLISH.md). Not mail. | AUR → Arch, Manjaro, Artix, Parabola | [`dist/aur/PUBLISH.md`](../dist/aur/PUBLISH.md) |
| 6 | Create a [gitlab.alpinelinux.org](https://gitlab.alpinelinux.org) account, fork `alpine/aports`, add `community/graft/` from [`dist/alpine/`](../dist/alpine/). Not mail. | Alpine edge + stables | [`dist/alpine/PUBLISH.md`](../dist/alpine/PUBLISH.md) |
| 7 | Do **not** reopen brew-core. [SMillerDev closed #311719](https://github.com/Homebrew/homebrew-core/pull/311719) for notability. Tap already works. | — | tap is already live |

Do **not** file BioArch, BlackArch, or ArchPOWER overlays.

## Close (only if you say so this session)

| Family | Link | Why close |
| --- | --- | --- |
| OpenWrt | https://github.com/openwrt/packages/pull/30553 | BKPepe asked for a router use-case. There isn't one. 10/10 arches fail (`MAKE_PATH` is workspace root). Duplicate of #30552, which we closed instead of amending. `Made with Cursor` in the body. |
| MacPorts | https://github.com/macports/macports-ports/pull/34726 | oytech: 4 days old, 0 stars, LLM-speak README. Do not argue. |
| Solus | https://github.com/getsolus/packages/pull/10718 | Already **draft**. davidjharder: file a package-request issue first. |

One short honest comment, then close. No patches. No force-push. No second PR.

## Open — do not touch

| Family | State | Link | Notes |
| --- | --- | --- | --- |
| nixpkgs | open, ofborg green, no human review | https://github.com/NixOS/nixpkgs/pull/564184 | 2 commits (format follow-up retriggered CI). Wait. |
| FreeBSD Ports | open, silent | https://github.com/freebsd/freebsd-ports/pull/624 | No comments. Do not ping. |

## Closed — do not comment or reopen

| Family | Closed by | Link | Why |
| --- | --- | --- | --- |
| Homebrew core | SMillerDev 2026-09-18 | https://github.com/Homebrew/homebrew-core/pull/311719 | Not notable. Tap is the path. Second commit cancelled bottle jobs. |
| Void | ahesford 2026-09-18 | https://github.com/void-linux/void-packages/pull/62574 | Too nascent; Void does not want upstream authors packaging their own software. |
| SlackBuilds | cwilling 2026-09-17 (`rejected`) | https://github.com/SlackBuildsOrg/slackbuilds/pull/17783 | New packages go through https://slackbuilds.org/submit/ |
| OpenWrt (first) | us, then BKPepe | https://github.com/openwrt/packages/pull/30552 | Bot said amend + force-push **this** PR. We opened #30553 instead. BKPepe: "Why you couldn't force push it?" |

## Not a GitHub PR

| Family | State | Notes |
| --- | --- | --- |
| GURU | branch only, no GitHub PR | https://github.com/techievena/guru/tree/graft-0.2.2 — do not open a GitHub PR. Mail later. |

## Live today (users can install)

| Surface | How |
| --- | --- |
| GitHub Releases | https://github.com/eonik-ai/graft/releases/tag/v0.2.2 |
| `install.sh` | `curl -fsSL https://raw.githubusercontent.com/eonik-ai/graft/main/scripts/install.sh \| sh` |
| Homebrew tap | `brew trust --tap eonik-ai/graft && brew tap eonik-ai/graft https://github.com/eonik-ai/graft && brew install graft` |
| Nix flake | `nix run github:eonik-ai/graft -- --help` |
| cargo git | `cargo install --git https://github.com/eonik-ai/graft.git --locked --bin graft` |
| GitHub `.deb` | `graft_0.2.2_amd64.deb` / `_arm64.deb` on the Release |

## Cannot list

| Surface | Why |
| --- | --- |
| crates.io `graft` | name taken by orbitinghail |
| pkg.bot row itself | appears after a parent distro **merges** |
| BioArch / BlackArch / ArchPOWER | overlays; install AUR or GitHub |

When a PR merges, mark the family **listed** here and in [`dist/pkg.bot-repos.md`](../dist/pkg.bot-repos.md) only after it shows on pkg.bot/Repology.
