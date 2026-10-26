#!/bin/zsh
# usage: video.sh BIN OUTDIR  -- renders frames 0..12 of three examples to mp4 (software and alpha codecs, one worker) and prints
# "label<TAB>codec<TAB>sha256 of the file's video stream", so the encoder path is held to byte identity too
BIN=$1
OUT=$2
mkdir -p $OUT
cd /Users/shriyansgandhi/Documents/Code/YoutubeVideoMaker/turbomotion
for ex in 01-shapes 02-type 03-media 04-plugins 09-look; do
  c=$(ls -d examples/comp/$ex*/comp.json | head -1)
  for codec in software alpha; do
    ext=mp4; [ $codec = alpha ] && ext=mov
    f=$OUT/$ex-$codec.$ext
    rm -f $f
    if $BIN comp render $c --from 0 --to 12 --workers 1 --codec $codec --out $f >$OUT/$ex-$codec.log 2>&1; then
      printf "%s\t%s\t%s\n" $ex $codec $(ffmpeg -v error -i $f -map 0:v -c copy -f md5 - | cut -c5-20)
    else
      printf "%s\t%s\trender-failed\n" $ex $codec
    fi
  done
done
