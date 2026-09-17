#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
# Labeled takes → ship → swap → side-by-side film of dest v1 vs dest v2.
# Mp4s are gitignored. The GIF is written to docs/assets/walkthrough.gif.
#
#   cargo build --release --bin graft
#   sh scripts/walkthrough.sh
set -eu
cd "$(dirname "$0")/.."
GRAFT="$(sh scripts/graft-bin.sh)"
ROOT="$PWD"

DEMO="${DEMO:-$(mktemp -d "${TMPDIR:-/tmp}/graft-walkthrough.XXXXXX")}"
mkdir -p "$DEMO/takes/hooks"
ASSET="${ROOT}/docs/assets/walkthrough.gif"

FONT=""
if ffmpeg -hide_banner -filters 2>/dev/null | grep -q ' drawtext '; then
  for f in \
    /System/Library/Fonts/Supplemental/Arial.ttf \
    /usr/share/fonts/truetype/dejavu/DejaVuSans.ttf \
    /usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf
  do
    if [ -f "$f" ]; then
      FONT="$f"
      break
    fi
  done
fi

vf_label() {
  if [ -n "$FONT" ]; then
    printf "scale=%s,drawtext=fontfile=%s:text='%s':fontsize=36:fontcolor=white:borderw=3:x=(w-text_w)/2:y=h-72" "$1" "$FONT" "$2"
  else
    printf "scale=%s" "$1"
  fi
}

plate() {
  out="$1"
  seconds="$2"
  color="$3"
  label="$4"
  ffmpeg -hide_banner -loglevel error -y -f lavfi \
    -i "color=c=${color}:s=240x426:r=30:d=${seconds}" \
    -vf "$(vf_label 240:426 "$label")" -an -c:v libx264 -pix_fmt yuv420p \
    -preset ultrafast "$out"
}

plate "$DEMO/takes/hook.mov" 1 red HOOK
plate "$DEMO/takes/body.mov" 2 green BODY
plate "$DEMO/takes/cta.mov" 1 yellow CTA
plate "$DEMO/takes/hooks/v2.mov" 1 blue HOOK-v2

ffmpeg -hide_banner -loglevel error -y -f lavfi \
  -i "sine=frequency=440:duration=4" -c:a pcm_s16le "$DEMO/takes/vo.wav"

cd "$DEMO"
"$GRAFT" ship --out ad.mp4 >/dev/null
"$GRAFT" swap hook ./takes/hooks/v2.mov --out ad-v2.mp4 >/dev/null

title() {
  ffmpeg -hide_banner -loglevel error -y -f lavfi \
    -i "color=c=0x111111:s=480x426:r=30:d=2" \
    -vf "$(vf_label 480:426 "$1")" -an -c:v libx264 -pix_fmt yuv420p \
    -preset ultrafast "$2"
}

title "takes" "$DEMO/title-in.mp4"
title "swap hook" "$DEMO/title-swap.mp4"

ffmpeg -hide_banner -loglevel error -y \
  -i "$DEMO/takes/hook.mov" -i "$DEMO/takes/body.mov" -i "$DEMO/takes/cta.mov" \
  -filter_complex "[0:v][1:v][2:v]hstack=inputs=3,scale=720:426,setsar=1,fps=12[v]" \
  -map "[v]" -an -t 2 -c:v libx264 -pix_fmt yuv420p -preset ultrafast "$DEMO/inputs.mp4"

ffmpeg -hide_banner -loglevel error -y \
  -i ad.mp4 -i ad-v2.mp4 \
  -filter_complex "[0:v]scale=240:426,setsar=1[l];[1:v]scale=240:426,setsar=1[r];[l][r]hstack=inputs=2,fps=12[v]" \
  -map "[v]" -an -c:v libx264 -pix_fmt yuv420p -preset ultrafast "$DEMO/side.mp4"

scale() {
  ffmpeg -hide_banner -loglevel error -y -i "$1" \
    -vf "scale=480:426:force_original_aspect_ratio=decrease,pad=480:426:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=12" \
    -an -c:v libx264 -pix_fmt yuv420p -preset ultrafast "$2"
}

scale "$DEMO/title-in.mp4" "$DEMO/s1.mp4"
scale "$DEMO/inputs.mp4" "$DEMO/s2.mp4"
scale ad.mp4 "$DEMO/s3.mp4"
scale "$DEMO/title-swap.mp4" "$DEMO/s4.mp4"
scale "$DEMO/side.mp4" "$DEMO/s5.mp4"

printf "file '%s'\n" "$DEMO/s1.mp4" "$DEMO/s2.mp4" "$DEMO/s3.mp4" "$DEMO/s4.mp4" "$DEMO/s5.mp4" > "$DEMO/list.txt"
ffmpeg -hide_banner -loglevel error -y -f concat -safe 0 -i "$DEMO/list.txt" \
  -c copy "$DEMO/walkthrough.mp4"

ffmpeg -hide_banner -loglevel error -y -i "$DEMO/side.mp4" \
  -vf "fps=12,scale=480:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse" \
  "$ASSET"

for dest in ad.mp4 ad-v2.mp4 walkthrough.mp4; do
  if [ ! -f "$DEMO/$dest" ]; then
    echo "walkthrough missing $dest" >&2
    exit 1
  fi
done

echo "demo dir $DEMO"
echo "wrote $ASSET"
echo "play $DEMO/walkthrough.mp4 (gitignored) and $DEMO/ad.mp4 vs $DEMO/ad-v2.mp4"
