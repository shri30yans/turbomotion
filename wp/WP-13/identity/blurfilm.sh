#!/bin/zsh
# usage: blurfilm.sh BIN OUTDIR  -- renders frames 0..40 of the four built films that use the older mblur key (software codec,
# one worker) and prints "film<TAB>seconds<TAB>sha256 of the video stream", so the blurred path and its cost are compared
BIN=$1
OUT=$2
BUILT=$3   # directory of built *.comp.json from snap.sh
mkdir -p $OUT
cd /Users/shriyansgandhi/Documents/Code/YoutubeVideoMaker/turbomotion
for n in motions_figma-anthem_film.ts motions_flow-state_film.ts motions_examples_micro-courage-reel_film.ts motions_examples_media-effects-reel_film.ts; do
  f=$OUT/$n.mp4
  rm -f $f
  start=$(python3 -c 'import time;print(time.time())')
  $BIN comp render $BUILT/$n.comp.json --from 0 --to 40 --workers 1 --codec software --out $f >$OUT/$n.log 2>&1 || { printf "%s\trender-failed\n" $n; continue; }
  end=$(python3 -c 'import time;print(time.time())')
  printf "%s\t%.2f\t%s\n" $n $(python3 -c "print($end-$start)") $(ffmpeg -v error -i $f -map 0:v -c copy -f md5 - | cut -c5-20)
done
