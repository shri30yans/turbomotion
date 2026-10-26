#!/bin/zsh
# usage: snap.sh BIN OUTDIR  -- renders stills of every existing example and film, prints "label<TAB>t<TAB>sha256"
BIN=$1
OUT=$2
REPO=/Users/shriyansgandhi/Documents/Code/YoutubeVideoMaker
mkdir -p $OUT/built
cd $REPO/turbomotion
comps=()
for f in examples/comp/*/comp.json examples/sdk/launch/film.comp.json examples/sdk/components/*.comp.json; do comps+=($REPO/turbomotion/$f); done
for f in motions/launch-film/film.ts motions/figma-anthem/film.ts motions/flow-state/film.ts motions/reel-clones/04-newspaper-match-cut/film.ts motions/examples/micro-courage-reel/film.ts motions/examples/snapgram-rebuild/snapgram/film.ts motions/examples/media-effects-reel/film.ts motions/examples/weave-looks/butterfly/film.ts motions/examples/weave-looks/bloom/film.ts motions/inspiration/artlist-ae-50/rebuilds/001-esports-momentum/film.ts; do
  name=$(echo $f | tr '/' '_')
  if $BIN sdk build $REPO/$f --out $OUT/built/$name.comp.json >$OUT/built/$name.log 2>&1; then
    comps+=($OUT/built/$name.comp.json)
    printf "%s\tbuilt-json\t%s\n" $name $(shasum -a 256 $OUT/built/$name.comp.json | cut -c1-16)
  else
    printf "%s\tbuild-failed\t-\n" $name
  fi
done
for c in $comps; do
  label=$(echo ${${c#$REPO/}#$OUT/} | tr '/' '_')
  d=$OUT/stills/$label
  rm -rf $d
  if $BIN comp still $c --out $d --times 0.5,1.2 >$OUT/stills_$label.log 2>&1; then
    for p in $d/*.png; do
      printf "%s\t%s\t%s\n" $label $(basename $p .png) $(shasum -a 256 $p | cut -c1-16)
    done
  else
    printf "%s\trender-failed\t%s\n" $label $(tail -1 $OUT/stills_$label.log | cut -c1-60)
  fi
done
