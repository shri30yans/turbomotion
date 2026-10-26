Byte-identity of the original films and modes, before WP-05 against after.

`snap.sh BIN OUTDIR` (WP-03's, plus the four probes made only of the original 17 blend modes: `old-modes`, `old-modes-light`, `display`, `light`) renders stills at 0.5 s and 1.2 s of every example, every SDK component and film, and the ten built films, and prints `label, time, sha256 (first 16)`. `video.sh` renders frames 0..12 of five examples with the software and alpha codecs on one worker and prints the md5 of the video stream.

- `before.tsv`: baseline binary built from `git archive HEAD` extracted into an empty directory (the commit WP-05 starts from).
- `after.tsv`: this working tree. `diff before.tsv after.tsv`: empty. 96 rows: 43 compositions x 2 stills, plus 10 built-film JSONs.
- `video-before.tsv`, `video-after.tsv`: the same, 10 rows (5 examples x software, alpha); `diff` empty.
