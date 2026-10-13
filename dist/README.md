# Packager recipes

Source recipes for distro packagers. A file here is not a listing in
that distro. Users who want a binary today: [install.md](../docs/install.md).

GitHub Releases attach tarballs, a musl Linux build, and `.deb` files.
Tag-archive hashes: [`SOURCE`](SOURCE).

| Recipe | Distro |
| --- | --- |
| [`../Formula/graft.rb`](../Formula/graft.rb) | Homebrew tap (this repo) |
| [`macports/Portfile`](macports/Portfile) | MacPorts |
| [`../flake.nix`](../flake.nix), [`nix/package.nix`](nix/package.nix) | Nix |
| [`aur/PKGBUILD`](aur/PKGBUILD) | AUR |
| [`alpine/APKBUILD`](alpine/APKBUILD) | Alpine |
| [`debian/`](debian/) | Debian / Ubuntu |
| [`rpm/graft.spec`](rpm/graft.spec) | Fedora and other rpm-based distros |
| [`void/template`](void/template) | Void |
| [`gentoo/`](gentoo/) | Gentoo |
| [`freebsd/Makefile`](freebsd/Makefile) | FreeBSD Ports |
| [`openbsd/Makefile`](openbsd/Makefile) | OpenBSD Ports |
| [`slackbuilds/graft.SlackBuild`](slackbuilds/graft.SlackBuild) | SlackBuilds |
| [`solus/package.yml`](solus/package.yml) | Solus |
| [`openwrt/Makefile`](openwrt/Makefile) | OpenWrt |

Until a distro accepts a recipe, install from GitHub Releases or
`cargo install --git`.
