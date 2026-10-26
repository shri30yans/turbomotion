Default-mode byte identity (WP-01). `snap.sh BIN OUTDIR > table.tsv` builds the ten `film.ts` of `motions/` named in it with `sdk build`
(sha256 of the JSON) and renders stills at 0.5 s and 1.2 s of every `examples/comp/*/comp.json`, the SDK examples and the built
films (sha256 of each PNG). `before.tsv` is the baseline binary (commit 5dff2ab, built before any change); `after.tsv` is this tree.
`diff before.tsv after.tsv` is empty (88 lines: 10 built films, 78 stills).
