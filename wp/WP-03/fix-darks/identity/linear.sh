#!/bin/zsh
# usage: linear.sh BIN OUTDIR  -- stills at 0.5 s and 1.2 s of every probe film that has the colour key, as "label<TAB>t<TAB>sha256"
# (the films that may legitimately change with the darks fix) and, for each, the red channel of the centre pixel
BIN=$1
OUT=$2
REPO=/Users/shriyansgandhi/Documents/Code/YoutubeVideoMaker
mkdir -p $OUT
cd $REPO/turbomotion
for f in wp/WP-03/probes/comp_linear.json wp/WP-05/probes/light.comp.json wp/WP-05/probes/old-modes-light.comp.json wp/WP-03/fix-darks/swatches-solid-light.comp.json wp/WP-03/fix-darks/swatches-shape-light.comp.json wp/WP-03/fix-darks/ramp-light.comp.json; do
  label=$(echo $f | tr '/' '_')
  d=$OUT/$label
  rm -rf $d
  if $BIN comp still $REPO/turbomotion/$f --out $d --times 0.5,1.2 >$OUT/$label.log 2>&1; then
    for p in $d/*.png; do
      printf "%s\t%s\t%s\n" $label $(basename $p .png) $(shasum -a 256 $p | cut -c1-16)
    done
  else
    printf "%s\trender-failed\t%s\n" $label $(tail -1 $OUT/$label.log | cut -c1-60)
  fi
done
