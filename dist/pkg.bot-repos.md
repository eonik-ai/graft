# pkg.bot repositories

[pkg.bot/repos](https://pkg.bot/repos) indexes **129** repositories
(Repology data). This file maps **every** listed repo onto one graft
recipe. It is not a claim that graft is already in those indexes.

Install today: [docs/install.md](../docs/install.md) (GitHub Releases,
Homebrew tap, Nix flake, `cargo install --git`).

crates.io crate name **`graft` is taken**
([orbitinghail/graft](https://github.com/orbitinghail/graft), a storage
engine). This project's crates stay `publish = false`. Do not `cargo
install graft` from crates.io.

## Recipe keys

| Key | Path | Covers |
| --- | --- | --- |
| brew | [`../Formula/graft.rb`](../Formula/graft.rb) | Homebrew tap `eonik-ai/graft` (not on pkg.bot) |
| nix | [`../flake.nix`](../flake.nix), [`nix/package.nix`](nix/package.nix) | nixpkgs stable/unstable |
| apt | [`debian/`](debian/) + GitHub `.deb` | Debian family and clones |
| rpm | [`rpm/graft.spec`](rpm/graft.spec) | Fedora, EPEL, RHEL clones, openSUSE, Mageia, OpenMandriva, Amazon, Terra, RPM Sphere, openEuler |
| aur | [`aur/PKGBUILD`](aur/PKGBUILD) | AUR; Arch/Manjaro/Artix/Parabola take AUR or later extra |
| apk | [`alpine/APKBUILD`](alpine/APKBUILD) | Alpine; musl GitHub tarball is the stopgap |
| portage | [`gentoo/`](gentoo/) | Gentoo, GURU, LiGurOS |
| macports | [`macports/Portfile`](macports/Portfile) | MacPorts |
| pkg | [`freebsd/Makefile`](freebsd/Makefile) | FreeBSD Ports |
| openbsd | [`openbsd/Makefile`](openbsd/Makefile) | OpenBSD Ports |
| slackbuilds | [`slackbuilds/graft.SlackBuild`](slackbuilds/graft.SlackBuild) | SlackBuilds and Slackware current |
| void | [`void/template`](void/template) | Void |
| solus | [`solus/package.yml`](solus/package.yml) | Solus |
| openwrt | [`openwrt/Makefile`](openwrt/Makefile) | OpenWrt |
| binary | GitHub Release | overlays that should not get a graft package of their own |

Do **not** file 129 identical bugs. One Debian ITP, one Fedora review,
one aports MR, one nixpkgs PR, one AUR publish, one macports-ports PR.

## Every pkg.bot repo

| pkg.bot repo | Distro / family | Recipe |
| --- | --- | --- |
| nixpkgs unstable | nixos / nix | nix |
| nixpkgs stable 26.05 | nixos / nix | nix |
| nixpkgs stable 25.11 | nixos / nix | nix |
| nixpkgs stable 25.05 | nixos / nix | nix |
| nixpkgs stable 24.11 | nixos / nix | nix |
| nixpkgs stable 24.05 | nixos / nix | nix |
| Ubuntu 26.10 | ubuntu / debuntu | apt |
| Ubuntu 26.04 | ubuntu / debuntu | apt |
| Ubuntu 25.10 | ubuntu / debuntu | apt |
| Ubuntu 25.04 | ubuntu / debuntu | apt |
| Ubuntu 24.04 | ubuntu / debuntu | apt |
| Ubuntu 22.04 | ubuntu / debuntu | apt |
| Ubuntu 20.04 | ubuntu / debuntu | apt |
| Ubuntu 18.04 | ubuntu / debuntu | apt |
| Ubuntu 16.04 | ubuntu / debuntu | apt |
| Ubuntu 14.04 | ubuntu / debuntu | apt |
| Ubuntu 26.10 Proposed | ubuntu / debuntu | apt |
| Fedora 44 | fedora | rpm |
| Fedora 43 | fedora | rpm |
| Fedora 42 | fedora | rpm |
| Fedora 41 | fedora | rpm |
| Fedora 40 | fedora | rpm |
| Fedora 39 | fedora | rpm |
| Fedora Rawhide | fedora | rpm |
| EPEL 10 | fedora | rpm |
| EPEL 9 | fedora | rpm |
| EPEL 8 | fedora | rpm |
| EPEL 7 | fedora | rpm |
| EPEL 6 | fedora | rpm |
| RPM Sphere | fedora | rpm |
| Amazon Linux 2 | fedora | rpm |
| Amazon Linux 1 | fedora | rpm |
| Terra rawhide | fedora | rpm |
| Terra 41 | fedora | rpm |
| Terra 40 | fedora | rpm |
| Terra 39 | fedora | rpm |
| AUR | arch | aur |
| Arch Linux | arch | aur |
| Manjaro Stable | arch | aur |
| Manjaro Testing | arch | aur |
| Manjaro Unstable | arch | aur |
| Parabola | arch | aur |
| Artix | arch | aur |
| BioArch | arch | binary (bio overlay; install AUR/GitHub) |
| BlackArch | arch | binary (pentest overlay; install AUR/GitHub) |
| ArchPOWER powerpc | arch | binary (no GitHub ppc build; source) |
| ArchPOWER powerpc64le | arch | binary (no GitHub ppc build; source) |
| Debian Unstable | debian / debuntu | apt |
| Debian 14 | debian / debuntu | apt |
| Debian 13 | debian / debuntu | apt |
| Debian 12 | debian / debuntu | apt |
| Debian 11 | debian / debuntu | apt |
| Raspbian Testing | raspbian / debuntu | apt |
| Raspbian Stable | raspbian / debuntu | apt |
| Raspbian Oldstable | raspbian / debuntu | apt |
| PureOS landing | pureos / debuntu | apt |
| PureOS byzantium | pureos / debuntu | apt |
| PureOS amber | pureos / debuntu | apt |
| Alpine Linux Edge | alpine | apk |
| Alpine Linux 3.24 | alpine | apk |
| Alpine Linux 3.23 | alpine | apk |
| Alpine Linux 3.22 | alpine | apk |
| Alpine Linux 3.21 | alpine | apk |
| Alpine Linux 3.20 | alpine | apk |
| Alpine Linux 3.19 | alpine | apk |
| Alpine Linux 3.18 | alpine | apk |
| Alpine Linux 3.17 | alpine | apk |
| Devuan Unstable | devuan / debuntu | apt |
| Devuan 4.0 | devuan / debuntu | apt |
| deepin 25 | deepin / debuntu | apt |
| deepin 23 | deepin / debuntu | apt |
| deepin 20 | deepin / debuntu | apt |
| openEuler 25.03 | centos | rpm |
| openEuler 24.03-LTS-SP1 | centos | rpm |
| openEuler 24.09 | centos | rpm |
| openEuler 24.03-LTS | centos | rpm |
| openEuler 22.03-LTS-SP4 | centos | rpm |
| openEuler 22.03-LTS-SP3 | centos | rpm |
| openEuler 22.03-LTS-SP1 | centos | rpm |
| openEuler 20.03-LTS-SP4 | centos | rpm |
| openEuler 20.03-LTS-SP3 | centos | rpm |
| Rocky Linux 9 | centos | rpm |
| Rocky Linux 8 | centos | rpm |
| AlmaLinux 9 | centos | rpm |
| AlmaLinux 8 | centos | rpm |
| CentOS Stream 10 | centos | rpm |
| CentOS Stream 9 | centos | rpm |
| CentOS Stream 8 | centos | rpm |
| CentOS 8 | centos | rpm |
| CentOS 7 | centos | rpm |
| CentOS 6 | centos | rpm |
| EuroLinux 9 | centos | rpm |
| EuroLinux 8 | centos | rpm |
| Trisquel 11.0 | trisquel / debuntu | apt |
| Trisquel 10.0 | trisquel / debuntu | apt |
| Gentoo | gentoo | portage |
| Gentoo overlay GURU | gentoo | portage |
| LiGurOS stable | gentoo | portage |
| LiGurOS develop | gentoo | portage |
| OpenMandriva 6.0 | openmandriva | rpm |
| OpenMandriva Rolling | openmandriva | rpm |
| OpenMandriva Cooker | openmandriva | rpm |
| Mageia 10 | mageia | rpm |
| Mageia 9 | mageia | rpm |
| Mageia cauldron | mageia | rpm |
| openSUSE Tumbleweed | opensuse | rpm |
| openSUSE Leap 15.6 | opensuse | rpm |
| openSUSE Leap 15.5 | opensuse | rpm |
| Kali Linux Rolling | kali / debuntu | apt |
| MacPorts | macports | macports |
| Parrot | parrot / debuntu | apt |
| FreeBSD Ports | freebsd | pkg |
| Pardus 21 | pardus / debuntu | apt |
| Apertis v2027 Development | apertis / debuntu | apt |
| Apertis v2026 | apertis / debuntu | apt |
| Apertis v2025 | apertis / debuntu | apt |
| Apertis v2024 | apertis / debuntu | apt |
| Apertis v2023 | apertis / debuntu | apt |
| OpenBSD Ports | openbsd | openbsd |
| SlackBuilds | slackbuilds | slackbuilds |
| Void Linux x86_64 | void | void |
| openKylin 3.0 | openkylin / debuntu | apt |
| openKylin 2.0 | openkylin / debuntu | apt |
| openKylin 1.0 | openkylin / debuntu | apt |
| Solus | solus | solus |
| Slackware64 current | slackware | slackbuilds |
| Slackware current | slackware | slackbuilds |
| OpenWrt 24.10 x86_64 | openwrt | openwrt |
| OpenWrt 23.05 x86_64 | openwrt | openwrt |

129 pkg.bot rows. Homebrew is extra (macOS, not indexed there).
