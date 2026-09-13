#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
# Build the temp project the landing GIF records. Prints the directory path.
set -eu

GRAFT="${GRAFT:-graft}"
DEMO="${DEMO:-$(mktemp -d "${TMPDIR:-/tmp}/graft-landing.XXXXXX")}"
cd "$DEMO"

lav() {
  # $1 out $2 seconds $3 lavfi source
  ffmpeg -hide_banner -loglevel error -y -f lavfi -i "$3" -t "$2" -an \
    -c:v libx264 -preset ultrafast -pix_fmt yuv420p "$1"
}

lav hook.mov 3 "testsrc2=size=480x854:rate=30"
lav body.mov 3 "color=c=0x3D4458:s=480x854:r=30"
lav cta.mov 2 "color=c=0x9BB0FF:s=480x854:r=30"
lav hook-v2.mov 3 "color=c=0x3ED89A:s=480x854:r=30"

"$GRAFT" init
"$GRAFT" slot body --span 3-6
"$GRAFT" slot cta --span 6-8 --role cta
"$GRAFT" scion create 9x16 --dest 480x854 --encoder x264
"$GRAFT" bind hook ./hook.mov
"$GRAFT" bind body ./body.mov
"$GRAFT" bind cta ./cta.mov

printf '%s\n' "$DEMO"
