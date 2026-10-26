Byte identity of default films, before and after WP-03.

`before.tsv` / `video-before.tsv` come from a binary built from the base commit 821a050 (`git archive HEAD` into a scratch
folder, built there). `after.tsv` / `video-after.tsv` come from the working tree. The two pairs are identical (`diff` is empty),
and were re-checked once more on the final binary.

- `snap.sh BIN OUTDIR`: one row per still, `label<TAB>time<TAB>sha256` of the PNG bytes. 88 rows: 39 compositions (every `examples/comp/*/comp.json`,
  `examples/sdk/launch/film.comp.json`, `examples/sdk/components/*.comp.json`), each as stills at t = 0.5 s and 1.2 s (78 rows), plus 10 built films (`sdk build`;
  those rows are `built-json` and hash the built composition JSON).
- `video.sh BIN OUTDIR`: five examples rendered to frames 0..12 with the software codec and with the alpha codec, one worker,
  10 rows, `label<TAB>codec<TAB>md5 of the video stream`. It holds the encoder path to byte identity too. The hardware (VideoToolbox)
  codec is not deterministic from run to run in the base build either, so it cannot be compared.

    zsh wp/WP-03/identity/snap.sh  $PWD/target/release/turbomotion /tmp/out > now.tsv && diff now.tsv wp/WP-03/identity/before.tsv
    zsh wp/WP-03/identity/video.sh $PWD/target/release/turbomotion /tmp/vid > now.tsv && diff now.tsv wp/WP-03/identity/video-before.tsv
