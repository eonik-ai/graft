# Publish graft to the AUR

Needs an AUR account and an SSH key uploaded at
https://aur.archlinux.org/account/.

```sh
# once
ssh-keyscan aur.archlinux.org >> ~/.ssh/known_hosts

git clone ssh://aur@aur.archlinux.org/graft.git
cd graft
cp /path/to/eonik/graft/dist/aur/PKGBUILD .
cp /path/to/eonik/graft/dist/aur/.SRCINFO .
# or: makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "graft 0.2.2"
git push
```

PKGBUILD is already hashed against
https://github.com/eonik-ai/graft/archive/refs/tags/v0.2.2.tar.gz
(`aa736b49bd825eaabcd0d713b3c0e502f794766c1c65cf061d18963daa69bf3b`).
