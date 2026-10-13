#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
# Cut real takes, ship, swap the hook, write README stills + GIF.
# Essence stays out of git. Assets are raster only.
#
#   cargo build --release --bin graft
#   sh scripts/readme-example.sh
#
# Override sources:
#   HOOK= BODY= CTA= HOOK2= sh scripts/readme-example.sh
set -eu
cd "$(dirname "$0")/.."
GRAFT="$(sh scripts/graft-bin.sh)"
ROOT="$PWD"
ASSET="$ROOT/docs/assets"
DEMO="${DEMO:-$(mktemp -d "${TMPDIR:-/tmp}/graft-readme-example.XXXXXX")}"
mkdir -p "$DEMO/takes/hooks" "$ASSET"

if [ -z "${HOOK:-}" ] || [ -z "${BODY:-}" ] || [ -z "${CTA:-}" ] || [ -z "${HOOK2:-}" ]; then
  echo "set HOOK= BODY= CTA= HOOK2= to real video files" >&2
  exit 1
fi

for f in "$HOOK" "$BODY" "$CTA" "$HOOK2"; do
  if [ ! -f "$f" ]; then
    echo "missing source $f" >&2
    exit 1
  fi
done

# Closed-GOP 9:16 takes. Short enough to encode here; long enough to read.
cut_take() {
  in="$1"
  ss="$2"
  t="$3"
  out="$4"
  ffmpeg -hide_banner -loglevel error -y -ss "$ss" -t "$t" -i "$in" \
    -vf "scale=720:1280:force_original_aspect_ratio=increase,crop=720:1280,fps=30,setsar=1,format=yuv420p" \
    -an -c:v libx264 -pix_fmt yuv420p -preset veryfast -crf 20 \
    -colorspace bt709 -color_primaries bt709 -color_trc bt709 -color_range tv \
    -x264-params "keyint=30:min-keyint=30:scenecut=0" \
    "$out"
}

cut_take "$HOOK" 0 2 "$DEMO/takes/hook.mov"
cut_take "$BODY" 3 4 "$DEMO/takes/body.mov"
cut_take "$CTA" 8 2 "$DEMO/takes/cta.mov"
cut_take "$HOOK2" 0 2 "$DEMO/takes/hooks/v2.mov"

cd "$DEMO"
"$GRAFT" ship --dest 720x1280 --out ad.mp4 >"$DEMO/ship.json" 2>"$DEMO/ship.err"
"$GRAFT" swap hook ./takes/hooks/v2.mov --out ad-v2.mp4 >"$DEMO/swap.json" 2>"$DEMO/swap.err"

# Side-by-side dests (full length). Still of the hook and of the body.
ffmpeg -hide_banner -loglevel error -y -i ad.mp4 -i ad-v2.mp4 \
  -filter_complex "[0:v]scale=360:640,setsar=1[l];[1:v]scale=360:640,setsar=1[r];[l][r]hstack=inputs=2[v]" \
  -map "[v]" -an -c:v libx264 -pix_fmt yuv420p -preset veryfast "$DEMO/side.mp4"

ffmpeg -hide_banner -loglevel error -y -ss 0.5 -i "$DEMO/side.mp4" -frames:v 1 \
  "$ASSET/example-hook.png"
ffmpeg -hide_banner -loglevel error -y -ss 3.4 -i "$DEMO/side.mp4" -frames:v 1 \
  "$ASSET/example-body.png"

ffmpeg -hide_banner -loglevel error -y -t 4 -i "$DEMO/side.mp4" \
  -vf "fps=6,scale=400:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=64:stats_mode=diff[p];[s1][p]paletteuse=dither=bayer" \
  "$ASSET/example.gif"

echo "demo dir $DEMO"
echo "stills $ASSET/example-hook.png $ASSET/example-body.png"
echo "gif $ASSET/example.gif"
echo "----- ship ledger -----"
cat "$DEMO/ship.err"
echo "----- swap ledger -----"
cat "$DEMO/swap.err"
