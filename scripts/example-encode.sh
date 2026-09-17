#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
# Encode the north-star clock on generated media. Does not mutate example JSON.
# Hook 3s, body 17s, cta 3s (score frames 90 + 510 + 90 @ 30fps).
set -eu
cd "$(dirname "$0")/.."
GRAFT="$(sh scripts/graft-bin.sh)"
DEMO="${DEMO:-$(mktemp -d "${TMPDIR:-/tmp}/graft-example-encode.XXXXXX")}"
mkdir -p "$DEMO/takes/hooks"

lav() {
  ffmpeg -hide_banner -loglevel error -y -f lavfi -i "$3" -t "$2" -an \
    -c:v libx264 -preset ultrafast -pix_fmt yuv420p "$1"
}

lav "$DEMO/takes/hook.mov" 3 "color=c=red:s=64x64:r=30"
lav "$DEMO/takes/body.mov" 17 "color=c=green:s=64x64:r=30"
lav "$DEMO/takes/cta.mov" 3 "color=c=yellow:s=64x64:r=30"
lav "$DEMO/takes/hooks/v2.mov" 3 "color=c=blue:s=64x64:r=30"

cd "$DEMO"
"$GRAFT" ship --out ad.mp4 >/dev/null
"$GRAFT" swap hook ./takes/hooks/v2.mov --out ad-v2.mp4 >/dev/null
test -f ad.mp4 && test -f ad-v2.mp4
echo "north-star clock encoded in $DEMO"
echo "play $DEMO/ad.mp4 and $DEMO/ad-v2.mp4"
