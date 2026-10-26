#!/bin/zsh
# usage: run.sh BIN OUTDIR  -- renders the probe films with BIN and writes, into OUTDIR, one table per film
# (level<TAB>output for the swatch sheets, x<TAB>output for the gradients) and the summary of each sheet.
BIN=$1
OUT=$2
HERE=${0:A:h}
mkdir -p $OUT
for n in swatches-solid-light swatches-shape-light swatches-solid-default ramp-light ramp-default; do
  $BIN comp still $HERE/$n.comp.json --out $OUT/$n --times 0 >/dev/null || exit 1
  if [[ $n == ramp-* ]]; then
    python3 $HERE/levels.py ramp $OUT/$n/0000.00.png > $OUT/$n.tsv
  else
    python3 $HERE/levels.py swatches $OUT/$n/0000.00.png > $OUT/$n.tsv
    python3 $HERE/levels.py summary swatches $OUT/$n/0000.00.png > $OUT/$n.summary.txt
  fi
done
