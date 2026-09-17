#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
# Typed session captured by asciinema. Run inside the demo dir from landing-setup.sh.
set -eu
export PS1='$ '
PROMPT='$ '

type_line() {
  printf '%s' "$PROMPT"
  sleep 0.35
  printf '%s' "$1" | awk '{
    n = split($0, a, "")
    for (i = 1; i <= n; i++) {
      printf "%s", a[i]
      fflush()
      system("sleep 0.038")
    }
  }'
  printf '\n'
  sleep 0.15
}

type_line "graft ship --out ad.mp4"
graft ship --out ad.mp4 >/dev/null
sleep 0.9

type_line "graft swap hook ./takes/hooks/v2.mov --out ad-v2.mp4"
graft swap hook ./takes/hooks/v2.mov --out ad-v2.mp4 >/dev/null
sleep 4
