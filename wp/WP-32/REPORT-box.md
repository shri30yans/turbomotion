WP: WP-32 step 1 (measured text bounds and the text `box` layer key). Steps 2-4 are not started.

Status: done for step 1. Nothing committed or pushed (HEAD is still c94ea9c on `workspace-split`).

Changed files:
- Engine, new: `crates/comp/src/schema/text_box.rs`, `text/bounds.rs`, `text/boxfit.rs`, `eval/text/bounds.rs`, `eval/text/mark.rs` and `eval/text/source.rs` (the last two moved out of `eval/text.rs` to keep it under 300 lines), `catalogue/text_box.rs`, `check/text_backing.rs`.
- Engine, modified: `schema/{mod,kinds,layer_keys}.rs`, `eval/{mod,text,salt}.rs`, `text/{layout,measure,place,mod}.rs`, `render/text.rs`, `check/{mod,lint}.rs`, `catalogue.rs`, `crates/cli/src/docs.rs`.
- SDK: `crates/script/sdk/{types,measure,index}.ts`, `crates/script/tests/sdk_option_types.rs` (field scan strips `r#`).
- Generated: `docs/reference/{layers,sdk,parity}.md`.
- Parity: `parity/specs/text_box.md` (new), `parity/MATRIX.tsv` (the row "Text measured bounds | none" is now four rows: bounds, box fitted to revealed characters, box drawn as measured in both effect spaces = `done`; `sourceRectAtTime` called from expressions = `none`, WP-12).
- Outside `turbomotion/`: `scripts/capability_index.py` (kind `text-key`, entries `text.box` and `text.box.<key>`), `motions/lib/ae_terms.json`, `motions/lib/CAPABILITIES.md` (regenerated, 740 entries).
- Probes and report: `wp/WP-32/probes/*` (README, `make.py`, 4 films, 5 PNGs, `data.tsv`), `wp/WP-32/REPORT-box.md`.
- Line counts: the largest new or changed file is `eval/text.rs` at 298 lines. Net change in `eval/mod.rs`, `schema/mod.rs` and `check/lint.rs` is 0 lines each; `plan/mod.rs`, `fxlib/mod.rs` and `render/mod.rs` are untouched.

Tests added:
- `crates/comp/tests/text_box/` (39 tests): width (36 string x font cases, 3 bundled fonts, kerning pairs, multi-line, tracking), fit (`min_width`, `align`, `radius`, `per_line`, keyed padding), reveal (204 typed-prefix samples), pixels (drawn box scanned from the render), layer_space (scale 2, 30 and 90 degrees), rect (`source_rect_at_time` and `comp inspect`), schema (strict keys, did-you-mean, negative values), lint (contrast against the box fill).
- Unit tests: `schema/text_box.rs` (3), `text/bounds.rs` (3), `text/boxfit.rs` (3), `catalogue/text_box.rs` (1: the example plans and its keys are all documented), `check/text_backing.rs` (1), `schema/layer_keys.rs` (+1).
- `crates/script/tests/sdk_text_box.rs` (3): option reaches the film, engine error carries the script line, `measure().bounds`.
- The examples harness (`examples_check`, 7 tests) runs the new catalogue example.

Gate output:
- `cargo test --release --workspace --no-fail-fast`: exit 0, 43 result lines, 703 passed, 0 failed, 2 ignored (both pre-existing: `ae_rotation`, `typecheck`).
- `docs --write`: `wrote 9 pages to docs/reference`; `docs --check`: `docs are current (9 pages in docs/reference)`; `git diff --stat docs/`: layers.md +83, parity.md 1 line, sdk.md +15 (98 insertions, 2 deletions).
- `python3 scripts/capability_index.py`: `motions/lib/CAPABILITIES.md: 740 entries`; the file exists (86,933 bytes) and has 10 `text.box` lines. `python3 -m unittest scripts.test_capability_index scripts.test_capability_sdk scripts.test_artlist_queue scripts.test_artlist_triage`: `Ran 47 tests ... OK`.
- `parity check`: `parity check: ok (86 core rows, 264 effect rows, 97 catalogue effects)`, exit 0.
- `comp check` on the four probe films: `0 errors, 0 warnings` for each.
- Determinism (software codec, frames 0-45, one worker, rendered twice, `shasum` of the mp4): fonts f23992e7df56c514 = f23992e7df56c514; typed 00809faf571062ca = 00809faf571062ca; layer-space 6e80bd5becbf9466 = 6e80bd5becbf9466; fit b3e88ff0342cd838 = b3e88ff0342cd838.
- Byte identity (baseline = `git archive HEAD` built in `/tmp/tb-base`, checked file by file against HEAD): the 96 still hashes (every example, every built film, at 0.5 s and 1.2 s) are identical, with `diff` empty and 0 failed renders. The 10 video-stream hashes are identical:

| film | software (before = after) | alpha (before = after) |
| --- | --- | --- |
| 01-shapes | 4a3b0253712f3ea9 | 20c5f315ddfb91f1 |
| 02-type | c861873f3d6aea2c | 3d9c588da461cacc |
| 03-media | 335e344cb50bee45 | 431cbcd47342649e |
| 04-plugins | 1c2f5826ae888ecd | 196ff4e24fe8bf50 |
| 09-look | 5c4d1aeedf048445 | e845c309b93e4c57 |

- Acceptance numbers (all asserted in tests; the PNGs only confirm them):
  - Box width = measured width + 2 x padding.x: 36 cases, worst 0.000015 px (limit 0.5).
  - Typed reveal: 204 samples, all within 0.5 px of the same text typed alone, no step narrower than the one before.
  - Drawn box vs measured box (pixel scan): 0.003 px in the tests; 0.014 px worst of 21 rows in `data.tsv`.
  - Radius: area 494.46 vs 494.44 expected; pill 28357.4 vs 28356.8.
  - Scale 2: box 2.000 times the scale-1 box, edge error at most 0.014 px; the 30 and 90 degree cases hold.
  - Kerning is in the box: AVATAR / WAVE / "To Yo" save 17.6 / 11.2 / 19.2 px in Manrope and 18.4 / 11.7 / 11.4 in Instrument Serif, and 0 in JetBrains Mono.
- Probe images were opened after the last layout change. `fonts.png`: three kerned red boxes and a two-line gold per-line box. `typed.png`: the bar growing from nothing to "To Yo-yo Wave". `layer-space.png`: scale 1, scale 2 and scale 2 at 30 degrees, box and text turning together. `fit.png`: three `min_width` bars growing left, centre and right, then square, rounded and pill. `keyed-padding.png`: padding 4, 28 and 52 px. Each has a numbers-first line in `probes/README.md`.

Deviations from this plan:
- `schema/kinds.rs` grows by 5 lines (the `box` field and its validation call).
- `Glyph` gained a `trail` field in `text/layout.rs`. A typed prefix would otherwise be up to 8.6 px narrower than the same text typed alone, because the shaper kerns the last visible character toward a hidden one. `trail` is its unkerned advance (0 at the end of a ligature) and is added only when a later character on the line exists but is not showing.
- A final test run caught that this was also applied to the true last character of a line. In a fallback font that added 0.72 px to the measured width. Now only a hidden next character adds it (`text/bounds.rs::rest_box`); `sdk_text_box::measure_returns_the_bounds_a_box_is_fitted_to` covers it.
- `box` with no characters showing draws nothing, and the box follows drawn geometry, including animator mask clips.
- `check/text_backing.rs` is a small change to the contrast lint: it reads the text's own box fill as the background. Without it, dark text on its own gold box was flagged as low contrast. The rule itself is not loosened.
- `schema/layer_keys.rs` gained `belongs_to`: an unknown key that is valid on another layer type now says so ("`box` is a key of text layers").
- `padd` gets no did-you-mean (edit distance 3 is over the engine's limit); `paddin` does. The tests use the latter.
- `mark.rs` and `source.rs` were moved out of `eval/text.rs` so it stays under 300 lines.
- Probe films were laid out inside the 5% safe area so `comp check` shows no warnings.

Open questions for the owner:
- `ffi` typed as `ff` (a ligature cut in half) is up to 3.65 px off the text typed alone. It is pinned by a test (bound 0.5 to 4.0) and written down as a known limit in `parity/specs/text_box.md`. Should the box instead jump at the ligature boundary?
- `sourceRectAtTime` is a typed Rust function (and `comp inspect` output); binding it into expressions is WP-12. Is that split right?
- Should `box` with `per_line: false` and several lines ever draw a rect that follows the ragged right edge, or is one block always right? It is one block now.

Estimated cost to finish (if partial): not partial. Steps 2-4 of WP-32 are untouched. The next slice is the repeater palette cycle and shape taper from WP-30, then Motion Tile.
