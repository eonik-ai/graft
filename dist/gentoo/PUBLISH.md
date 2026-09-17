# Gentoo GURU

GitHub PRs on https://github.com/gentoo/guru need GURU contributor
access. Apply: https://wiki.gentoo.org/wiki/Project:GURU

The ebuild is already on
https://github.com/techievena/guru/tree/graft-0.2.2

After access is granted:

```sh
gh pr create --repo gentoo/guru --head techievena:graft-0.2.2 \
  --title "media-video/graft: new package" \
  --body "Local-first composition workspace. Incremental compiler for video. Runtime ffmpeg (shell-out)."
```
