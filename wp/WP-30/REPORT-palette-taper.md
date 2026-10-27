WP: WP-30 (slices only): repeater palette cycle, and shape stroke taper. Every other WP-30 step is untouched.

Status: done for the two slices. Fractional copies and the start/end opacity ramp of the repeater are not part of either slice and do not exist in the engine (see Deviations); the palette cycle works with the integer copies and the per-copy opacity factor that do exist.

Changed files (all under `turbomotion/` unless stated; nothing committed):

New
- `crates/core/src/color/mix.rs`: `Profile::mix`, the blend in the working profile's numbers (Skia's own sRGB curve).
- `crates/comp/src/anim/cycle.rs`: `NonEmpty`, `CycleMode`, `slot()` (which colour, or which two and how much of each, copy `i` gets).
- `crates/comp/src/schema/palette.rs`: `Cycling` (palette, cycle, paint), `Cycle`, `Target`, `Swatch`; the one typed definition both repeaters and the `swatch` layer key use.
- `crates/comp/src/schema/taper.rs`: `Taper`, `Tip`, `Extent` (px or fraction), with `validate()`.
- `crates/comp/src/schema/shape_tests.rs`, `schema/repeat_tests.rs` (tests moved out of the files they test, to keep those under 300 lines).
- `crates/comp/src/shape/mod.rs`, `shape/stroke/{mod,vec2,sample,pieces,taper,coverage,tests}.rs`: the one variable-width mechanism. `Profile` (width at a distance along the path), the walk (`sample.rs`), the pieces and their union (`pieces.rs`), the taper profile (`taper.rs`).
- `crates/comp/src/eval/palette.rs`, `eval/palette/tests.rs`: `Cycled`, `resolve`, `apply` (the colour of a copy at time t).
- `crates/comp/src/eval/shape/repeat.rs`, `eval/shape/taper.rs`: the shape repeat state (with its tint) and the taper state.
- `crates/comp/src/render/palette_tests.rs`, `render/taper_tests.rs`.
- `crates/comp/src/catalogue/palette_cycle.rs`, `catalogue/stroke_taper.rs`.
- `crates/script/sdk/palette-types.ts`, `crates/script/tests/sdk_palette_taper.rs`.
- `parity/specs/palette_cycle.md`, `parity/specs/stroke_taper.md`.
- `wp/WP-30/probes/` (`make.py`, 10 `*.comp.json`, 10 `*.png`, `data.tsv`, `README.md`), `wp/WP-30/identity/{before,after,video-before,video-after}.tsv`, this report.

Modified
- `crates/comp/src/anim/mod.rs`, `lib.rs`, `eval/mod.rs` (one hook line, and one `use` collapsed: 737 to 735 lines), `eval/salt.rs` (3 salts), `eval/shape.rs`, `plan/layer.rs` (checks a layer's `swatch`), `render/mod.rs` (2 test modules), `render/shape.rs` (copy tint; a tapered stroke is drawn as a filled outline), `schema/mod.rs` (`Common.swatch`; same length, 493), `schema/shape.rs` (`taper`, `ShapeItem::check`, `Repeat` read by hand so a stray key gets a did-you-mean), `schema/repeat.rs` (palette keys and swatch stamping; 254 to 204 lines), `schema/wrap.rs`, `catalogue.rs`, `catalogue/layer_keys.rs` (form of `Swatch`), `core/src/color/mod.rs`.
- `crates/cli/src/docs.rs` (a `feature()` helper used for `text_box`, `palette_cycle` and `stroke_taper`), `crates/script/sdk/types.ts` (`LayerOpts.swatch`, `repeat`), `crates/script/sdk/shape-types.ts` (`taper`, `repeat`).
- `docs/reference/{layers,sdk,parity}.md` (regenerated), `parity/MATRIX.tsv`.
- `scripts/capability_index.py`, `scripts/test_capability_index.py` (outer repo; the index gets `repeat-key` and `shape-key` entries), `motions/lib/CAPABILITIES.md` (regenerated), `motions/lib/layer_keys.json` (the `repeat` summary). The outer repo was already heavily dirty before this work; none of my edits are in the paths the brief forbids.

Tests added (all passing):
- core `color::mix`: a_display_profile_blends_the_numbers_as_written, a_linear_profile_blends_light_and_returns_a_colour_on_the_published_curve, the_ends_of_a_blend_are_the_colours_and_the_gamut_makes_no_difference_in_light.
- `anim::cycle`: step_repeats_the_palette_and_a_whole_offset_moves_every_copy_on, a_fractional_offset_holds_a_colour_until_the_offset_reaches_the_next_whole_number, ping_pong_runs_forwards_and_back_without_doubling_the_ends, a_gradient_blends_the_colours_either_side_and_wraps_the_last_into_the_first, a_non_empty_list_is_read_round_and_reversed_without_an_empty_case.
- `schema::palette`, `schema::taper`, `schema::shape_tests`, `schema::repeat_tests` (strict errors: empty palette, non-colour entry, negative length or width, taper on a fill-only shape, a dash with a taper, stray keys): a_palette_with_a_cycle_and_a_paint_is_read_and_removed, mistakes_say_what_to_write, a_swatch_needs_an_index_and_a_palette_and_names_a_stray_key, lengths_and_widths_are_pixels_or_fractions, mistakes_name_the_field, a_taper_and_a_palette_are_read_from_the_item, a_taper_needs_a_stroke_to_thin, a_taper_with_a_bad_number_a_misspelt_key_or_a_dash_says_what_to_change, a_palette_on_a_repeat_must_be_colours_and_must_have_something_to_colour, a_palette_stamps_a_swatch_on_the_original_and_every_copy_and_is_checked_where_it_is_written, mistakes_are_named.
- `shape::stroke::tests` (9): width at start, middle and end; ease from each tip; curved path; half-path clamp; closed path; Skia agreement for every cap and join; area = width x length; hairpin leaves no hole; a profile that is not a taper uses the same walk.
- `eval::palette::tests` (9): step for whole, negative and fractional offsets; ping_pong; display gradient; linear-light gradient (mid-point of black and white is level 188); alpha; keyed offset; non-finite offset named; layer repeater stamps; swatch on a layer with nothing to colour refused.
- `render::palette_tests` (9) and `render::taper_tests` (7): the same behaviours measured in rendered pixels (step in display and linear_light films over several N, n, offsets and reverse; closed-form gradient in both spaces; gradient against a Skia gradient in the same film; keyed offset; opacity ramp; stroke/both targets; solids, shapes and text; taper widths for three eases, a curve, fractions, clamp, closed path, trimmed outline and a copy's tint).
- catalogue: the_entry_names_every_key_the_parser_reads_and_its_example_plans_with_a_palette_on_both_repeaters, the_entry_names_every_key_the_parser_reads_and_its_example_plans_with_a_taper_on_each_item.
- script: a_repeat_with_a_palette_and_a_taper_reach_the_film_as_written, the_engine_checks_a_palette_and_a_taper_written_in_a_script_and_names_the_line. Existing `every_common_layer_field_has_an_sdk_option` now also covers `swatch`.
- `scripts/test_capability_index.py`: the_repeater_palette_cycle_and_the_stroke_taper_are_found_by_name_and_by_what_they_do.
- The catalogue examples run through `examples_check` (7 tests) with no lint rule loosened, no `#[ignore]`, no allow-list.

Gate output:

1. `cargo test --release --workspace --no-fail-fast`: exit 0, 44 test binaries, 768 passed, 0 failed, 2 ignored (both pre-existing: `ae_rotation` needs an After Effects dump; `typecheck` needs `TURBOMOTION_TEST_TSC`). Main groups: cli 19+1+3+4; comp lib 462; comp tests catalogue_keys 6, color_schema 5, effect_space_schema 3, examples_check 7, film_problems 6, repeat 3, text_box 39; core 69; script tests (sdk_option_types 6, sdk_palette_taper 2, and others).
   ```
   test result: ok. 462 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.04s   (turbomotion-comp lib)
   test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.41s   (examples_check)
   test result: ok. 69 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.25s   (turbomotion-core lib)
   ```
2. `cargo run --release -p turbomotion -- docs --write` then `--check`:
   ```
   wrote 9 pages to docs/reference
   docs are current (9 pages in docs/reference)
   ```
   `git diff --stat docs/`: `layers.md` +178, `parity.md` 2 lines, `sdk.md` +44 (the new entries and the matrix rows only).
3. `python3 scripts/capability_index.py` -> `motions/lib/CAPABILITIES.md: 751 entries`; `find taper` returns `shape-key | shape.taper | ...` and `find palette cycle` returns `repeat-key | repeat.palette`; its unit tests: `Ran 30 tests ... OK`.
4. `turbomotion parity check` -> `parity check: ok (90 core rows, 264 effect rows, 97 catalogue effects)`, exit 0.
5. `turbomotion comp check` on the 10 probe films: each `0 errors, 0 warnings`.
6. Determinism (`make.py`, software codec, frames 0 to 12, one worker, rendered twice): palette-step `2ea031693323c642` both times; taper-straight `c0b1ec4ea9ab7a79` both; taper-closed `a8053898d2da2423` both.
7. Probes and what they show: `wp/WP-30/probes/README.md` (one line each) and `data.tsv`.

Acceptance numbers (measured against values worked out in `make.py`, not read from the engine):

| check | measured error | limit |
| --- | --- | --- |
| step, 6 configs of N, n, offsets (0, 1, -1, 2.5, -0.5, 7), reverse, 48 copies | 0 levels | 1 |
| gradient, display film | 0.50 level | 1 |
| gradient, linear_light film | 0.49 level | 1 |
| gradient against a Skia gradient in the same linear_light film (test) | within 1 level | 1 |
| ping_pong index sequence over 4 colours | `0 1 2 3 2 1 0 1` exact | exact |
| keyed offset 0 to 4 over 1 s | indices exact at t = 0, 0.5, 1 | exact |
| opacity ramp 0.8 per copy | 0.89 level (display), 0.48 level (linear) | 1 |
| taper, straight, 3 eases, 8 distances each | 0.090, 0.087, 0.158 px | 0.25 |
| taper, curve, fractions, 9 distances | 0.134 px | 0.25 |
| taper, closed circle, 7 distances | 0.099 px | 0.25 |
| taper, lengths of 5000 px held to half the 400 px path | 0.000 px | 0.25 |

Which space the gradient interpolates in, and why: the film's working colour space (`parity/specs/colour_pipeline.md`), because that is where Skia blends and interpolates gradient stops after converting them (`CompPlan.color`). In a `display` film that is the encoded sRGB numbers; in a `linear_light` film it is light, and the CPU colour of a copy goes through Skia's own sRGB curve (`Profile::mix`) so it agrees with a drawn gradient. The gamut does not matter in light (a matrix commutes with a blend of light; tested).

Byte identity of existing films (baseline built from `git archive HEAD` extracted into an empty `/tmp/pt-base`, built read-only, deleted afterwards; method of `wp/WP-10/identity/{snap.sh,video.sh}`; tables in `wp/WP-30/identity/`):

| set | before | after | result |
| --- | --- | --- | --- |
| stills at 0.5 s and 1.2 s of 43 films (33 example/probe comps and the 10 built `film.ts`) | 86 hashes | 86 hashes | `diff` empty |
| the 10 built `film.ts` comp JSONs themselves (so the SDK output is identical too) | 10 hashes | 10 hashes | `diff` empty |
| 10 video-stream hashes (5 examples x software and alpha codec) | 10 | 10 | `diff` empty |

Deviations from this plan:
- Fractional copies and the start/end opacity ramp (the brief says the cycle "must work with fractional copies and the opacity ramp") do not exist in either repeater: `count` and `copies` are integers and the only fade is a geometric per-copy `opacity` factor. I did not build them, as they are outside these two slices. The palette depends only on the integer copy index and the (fractional, animatable) cycle `offset`; it composes with the existing opacity factor (probe `palette-opacity`, 0.89 level). `parity/MATRIX.tsv` has a `none` row for them (WP-30).
- `taper` is a key of the shape item (next to `stroke`), not inside `stroke`, so it can apply to any stroked item without changing the `stroke` type; it needs the item to have a stroke and refuses a dash (a dashed taper is a later step of the same mechanism).
- A layer-repeater palette is carried by a new typed layer key `swatch`, stamped on each copy by the repeat pre-pass, because the pre-pass runs on JSON before time exists and the offset can be keyed. A layer that has its own `swatch` and sits under a repeat palette is refused. This adds `swatch` to the common layer keys and so to the SDK's `LayerOpts`.
- The tests in `shape/stroke/tests.rs` use the engine's ease functions for the expected widths; `render/taper_tests.rs` and `make.py` use independent formulas, so the ease table itself is checked there.
- The Skia curve differs from the IEC formula by up to about 0.02 level, so the mix test allows a quarter of a level.
- `motions/lib/CAPABILITIES.md`, `layer_keys.json` and `ae_terms.json` were already modified before this work; `CAPABILITIES.md` was regenerated on top, `layer_keys.json` got one summary line.

Open questions for the owner:
- Should fractional copies and the start/end opacity ramp be built as their own slice before Motion Tile (they change the repeat model, and the palette already takes `copy` as the argument)?
- Should the taper later move inside `stroke` once dashes and gradient strokes land (so `taper`, `dash` and `gradient` share one block)?

Estimated cost to finish (if partial): 0 for these two slices; fractional copies and the opacity ramp about 4 hours.
