#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
# Generate takes and walk ship → swap → address → swap. No media is committed.
#
#   cargo build --release --bin graft
#   sh scripts/demo.sh
set -eu
cd "$(dirname "$0")/.."
GRAFT="$(sh scripts/graft-bin.sh)"

DEMO="${DEMO:-$(mktemp -d "${TMPDIR:-/tmp}/graft-demo.XXXXXX")}"
mkdir -p "$DEMO/takes/hooks"

lav() {
  ffmpeg -hide_banner -loglevel error -y -f lavfi -i "$3" -t "$2" -an \
    -c:v libx264 -preset ultrafast -pix_fmt yuv420p "$1"
}

lav "$DEMO/takes/hook.mov" 1 "color=c=red:s=64x64:r=30"
lav "$DEMO/takes/body.mov" 2 "color=c=green:s=64x64:r=30"
lav "$DEMO/takes/cta.mov" 1 "color=c=yellow:s=64x64:r=30"
lav "$DEMO/takes/hooks/v2.mov" 1 "color=c=blue:s=64x64:r=30"
lav "$DEMO/takes/hooks/v3.mov" 1 "color=c=black:s=64x64:r=30"
ffmpeg -hide_banner -loglevel error -y -f lavfi \
  -i "sine=frequency=440:duration=4" -c:a pcm_s16le "$DEMO/takes/vo.wav"

cd "$DEMO"
"$GRAFT" ship --out ad.mp4 >ship.json
BUILD="$(python3 -c 'import json; print(json.load(open("ship.json"))["build"]["id"])')"
"$GRAFT" swap hook ./takes/hooks/v2.mov --out ad-v2.mp4 >/dev/null
"$GRAFT" address --kind hook_rate --build "$BUILD" >/dev/null
"$GRAFT" swap hook ./takes/hooks/v3.mov --out ad-v3.mp4 >/dev/null

for dest in ad.mp4 ad-v2.mp4 ad-v3.mp4; do
  if [ ! -f "$dest" ]; then
    echo "demo missing $dest" >&2
    exit 1
  fi
done

echo "demo dir $DEMO"
echo "play $DEMO/ad.mp4, $DEMO/ad-v2.mp4, and $DEMO/ad-v3.mp4"
