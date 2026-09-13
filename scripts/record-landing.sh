#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
# Record the landing asciinema from the graft repo root.
#
#   cargo build --release --bin graft
#   sh scripts/record-landing.sh
#
# Writes docs/assets/landing.cast (source) and docs/assets/landing.gif (GitHub).
set -eu
cd "$(dirname "$0")/.."
export PATH="$PWD/target/release:$PATH"

need() {
  if ! command -v "$1" >/dev/null; then
    echo "need $1 on PATH ($2)" >&2
    exit 1
  fi
}

need graft "cargo build --release --bin graft"
need ffmpeg "brew install ffmpeg"
need asciinema "brew install asciinema"
need agg "brew install agg"

DEMO="$(sh scripts/landing-setup.sh)"
CAST=docs/assets/landing.cast
GIF=docs/assets/landing.gif

asciinema rec \
  --overwrite \
  --output-format asciicast-v2 \
  --command "cd \"$DEMO\" && sh \"$PWD/scripts/landing-session.sh\"" \
  --window-size 88x20 \
  --title "graft: compile, rebind the hook, body stays" \
  --idle-time-limit 1.2 \
  --headless \
  --return \
  --quiet \
  "$CAST"

agg "$CAST" "$GIF" \
  --theme github-dark \
  --font-size 18 \
  --line-height 1.35 \
  --speed 1.15 \
  --idle-time-limit 0.8 \
  --cols 88 \
  --rows 20 \
  --font-family "Menlo,SF Mono,JetBrains Mono,Uiua386,monospace"

echo "wrote $CAST"
echo "wrote $GIF"
