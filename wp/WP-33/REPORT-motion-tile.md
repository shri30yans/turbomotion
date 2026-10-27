WP: WP-33 (one effect only): Motion Tile. No other effect or WP was touched.

Status: done, with one disclosed limit. After Effects cannot be measured here, so the tiling model, and above all the phase rule, are **definitions** taken from the plan, the AE help text as remembered and the EffectCraft reference (read only, nothing copied); `parity/specs/motion_tile.md` marks each one. The ledger row is `done` because test, spec and probe all exist, not because the rules were measured against After Effects.

Changed files (under `turbomotion/` unless stated; nothing committed):

New
- `crates/comp/src/fxlib/builtin/motion_tile.sksl` (72 lines): the effect.
- `crates/comp/src/fxlib/bounds_tests.rs`: the `@bounds` tests, moved out of `bounds.rs` plus two new ones.
- `crates/comp/src/eval/buffer_motion_tile_tests.rs`: planning tests (8).
- `crates/comp/src/render/fx_motion_tile_tests.rs`, `crates/comp/src/render/motion_tile_seam_tests.rs`: rendered-pixel tests (11 and 9).
- `parity/specs/motion_tile.md`, `parity/probes/motion_tile.json`.
- `wp/WP-33/REPORT-motion-tile.md`, `wp/WP-33/motion-tile/{make.py, README.md, data.tsv, panels.png, layer-vs-screen.png, scale2-quarter-turn.png, scale2-rotation30.png, seam-subpixel.png, identity/{before,after,video-before,video-after}.tsv}`.

Modified
- `crates/comp/src/fxlib/bounds.rs` (251 lines, was 289 before its tests moved out): the `@bounds` grammar gains `box_w` and `box_h` (the layer's own content size), so growth can be a share of the layer; a float parameter named like either is refused with a "rename the parameter" message; `growth()` takes the content box.
- `crates/comp/src/eval/buffer.rs` (148 lines): passes the content box to `growth()`; one `#[cfg(test)]` module line.
- `crates/comp/src/fxlib/mod.rs`: one doc-comment line (the `@bounds` summary now names `box_w`, `box_h`). No growth beyond that; `render/mod.rs` gains 4 lines of test-module declarations.
- `parity/effects.tsv`: Motion Tile `todo` -> `done` (grows_bounds `yes`, linear `yes`, engine `motion_tile`, test `an_output_of_300_percent_repeats_the_card_with_exact_pixels_at_every_tile_origin`).
- `docs/reference/effects.md` (+31, the new entry), `docs/reference/parity.md` (1 line: Motion Tile leaves the WP-33 list). Regenerated.
- Outer repo: `motions/lib/ae_terms.json` (a `motion tile` term, aka tile, tiling, repeat tile), `motions/lib/CAPABILITIES.md` (regenerated, 751 entries). The outer repo was already dirty before this work; none of my edits are in forbidden paths.

Tests added (all passing; 28 for Motion Tile + 2 for the `@bounds` grammar):
- `fxlib::bounds::tests`: the_layers_own_width_and_height_can_be_part_of_an_expression, a_parameter_may_not_hide_the_layers_size (the old tests now pass a content box).
- `eval::buffer::motion_tile_tests` (planning): the_picture_grows_by_half_the_extra_output_on_each_side, an_output_that_is_smaller_than_the_layer_does_not_shrink_the_picture, the_default_output_adds_nothing, a_fraction_of_a_pixel_is_rounded_outwards, the_growth_follows_an_animated_output_size, an_output_past_its_range_is_held_at_one_thousand_percent, it_adds_to_the_effects_around_it, an_output_that_would_make_the_picture_too_big_is_an_error_that_names_the_layer.
- `render::fx_motion_tile_tests`: motion_tile_with_neutral_parameters_returns_the_input_byte_for_byte (both effect spaces), an_output_of_300_percent_repeats_the_card_with_exact_pixels_at_every_tile_origin, mirror_edges_flips_every_other_tile_and_leaves_the_middle_one_alone, a_phase_of_a_quarter_turn_shifts_each_row_a_quarter_of_a_tile_further_than_the_one_before, without_the_horizontal_switch_the_phase_shifts_each_column_up_and_down, a_phase_of_a_whole_turn_is_a_whole_tile_and_changes_nothing, moving_the_tile_centre_moves_the_tiles_with_it, tiles_of_half_the_size_hold_the_whole_card_reduced_two_to_one, an_output_of_50_percent_keeps_the_middle_half_of_the_layer_and_clears_the_rest, a_gutter_leaves_a_gap_of_its_colour_between_tiles_and_makes_the_pitch_tile_plus_gutter, motion_tile_renders_the_same_twice.
- `render::motion_tile_seam_tests`: a_centre_off_by_a_third_of_a_pixel_blends_two_texels_and_keeps_the_seam_columns_exact, moving_the_centre_by_whole_pixels_scrolls_the_card_around_with_no_gap, the_centre_and_the_phase_animate_with_keys, layer_space_and_screen_space_draw_the_same_tiles_over_the_frame, a_layer_scaled_by_two_and_turned_a_quarter_keeps_its_pattern_in_layer_space, a_half_pixel_slide_is_blended_where_the_film_is_in_light_and_where_it_is_not, a_reduced_tile_is_the_average_of_the_pixels_it_covers_in_the_encoding_the_film_is_in, a_mistyped_parameter_or_option_names_the_valid_ones, a_tile_width_below_its_range_is_held_at_the_smallest_tile.
- The catalogue `@example` runs through `examples_check` (7 tests) with no lint rule loosened, no `#[ignore]`, no allow-list.

Gate output:

1. `cargo test --release --workspace --no-fail-fast`: exit 0, **798 passed, 0 failed, 2 ignored** (both pre-existing: `ae_rotation` needs an After Effects dump, `typecheck` needs `TURBOMOTION_TEST_TSC`). Goldens unchanged.
   ```
   test result: ok. 492 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out   (turbomotion-comp lib)
   test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out     (examples_check)
   test result: ok. 69 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out    (turbomotion-core lib)
   test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out     (turbomotion-script lib)
   test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out    (turbomotion cli, unit)
   ```
2. `docs --write`, `docs --check`:
   ```
   wrote 9 pages to docs/reference
   docs are current (9 pages in docs/reference)
   ```
   `git diff --stat docs/`: `effects.md` +31, `parity.md` 1 line.
3. `python3 scripts/capability_index.py` -> `motions/lib/CAPABILITIES.md: 751 entries`; `find motion tile` returns `ae term 'motion tile'` and `effect | motion_tile | Repeats the layer as tiles ...`; `python3 -m unittest` in `scripts/`: `Ran 48 tests ... OK`.
4. `turbomotion parity check` -> `parity check: ok (90 core rows, 264 effect rows, 98 catalogue effects)`, exit 0. `parity report`: Stylize 24 rows, 1 done; T0+T1 effects done 1 of 172.
5. `turbomotion comp check parity/probes/motion_tile.json` -> `ok: 1280x720 at 30 fps, 1.00s (30 frames), 12 layers, 1 nested comps; 0 assertions; 0 errors, 0 warnings`.
6. Determinism (software codec, frames 0 to 5, one worker, rendered twice): file `shasum` `26c5cbe05af1cd80699fa3fea12a14ecd1d83c88` both times; video-stream md5 `48b8d204cb5246574f8fdbbc7ffaa77d` both times. The planning and pixel tests also render twice (`motion_tile_renders_the_same_twice`).
7. Probe images in `wp/WP-33/motion-tile/`, one line each in its `README.md`.

Byte identity of the existing films (baseline = `git archive HEAD` extracted into a fresh `/tmp/mt-base`, made read-only, built with the target dir outside the tree; both deleted afterwards; reusing `wp/WP-10/identity/{snap.sh,video.sh}`):

| set | what | rows | before | after | diff |
| --- | --- | --- | --- | --- | --- |
| stills | 53 films (examples/comp, SDK launch and components, 4 WP-05 probes, 10 built `film.ts`), stills at 0.5 s and 1.2 s, plus the built-json hashes | 96 | `c42a90f9f6a1...` (shasum of the table) | `c42a90f9f6a1...` | none |
| video | 5 examples x {software, alpha}, frames 0 to 12, video-stream md5 | 10 | `1addd908718c...` | `1addd908718c...` | none |

Zero `build-failed` or `render-failed` rows in either table. Full tables: `wp/WP-33/motion-tile/identity/`. (The baseline binary rejects the new probe with "no effect or shader called motion_tile", as it should.)

Acceptance numbers (worst 8-bit level difference; rendered pixels against values worked out on integers or, for `make.py`, in numpy from the spec formulas):

| criterion | numbers |
| --- | --- |
| 300 % output repeats the tiles, exact at closed-form origins | 0 levels at every tile origin of the 3 x 3 grid (`an_output_of_300_percent_...`) |
| Mirror edges flips alternate tiles | 0 levels; middle tile unflipped, odd tiles flipped (mirror panel 0.00 over 93 600 px) |
| Phase of 0.25 (90 degrees) | row `j` shifted `j * 8` px on a 32 px card (closed form `x + 8*row`), 0 levels; columns `y + 4*col` with the switch off; 360 degrees changes nothing; brick panel (180) 0.50 levels over 93 600 px |
| Centre offsets | grid moves by exactly the offset: a +/- whole-pixel scroll test with no gap, 0 levels; probe panel (+20, -10 px) 0.50 |
| Output 50 % crops | middle half of the layer kept, the rest transparent (0 levels); probe 60 % crop 0.00 over 5 568 px |
| Layer-space buffer growth | a 32 x 16 layer at 300 % plans origin (-32, -16), size 96 x 48; width only: 96 x 16; 150 % gives -8 / 48; 117 % height gives -2 / 20; animated 100 -> 300 gives widths 32 / 64 / 96 |
| Layer space equals screen space over the frame | 4 parameter sets <= 1 level; the probe's frame-sized card 0.00 |
| Layer scale 2 and rotation | rotation 90 equals the flat picture turned with `np.rot90`: 0.00 levels; rotation 30 drawn in `scale2-rotation30.png` |
| Sub-pixel seams, no bleed | centre 0.3 px off: the seam columns (x = -32, 0, 32 and the columns before them) are exactly the card's first and last columns, alpha 255; the 0.3 / 0.7 blend appears only at the card's own interior edge |
| Reduction filtering | 2:1 reduction equals the exact 2 x 2 mean; stripes of 224 and 32 average to 127-128 encoded in a display film and 187-189 in a `linear_light` film |
| Animation | centre and phase keyed with linear keys give the closed-form position at the start and halfway |
| Validation | a mistyped name gets "did you mean"; a mistyped option lists `off, on`; a width below its range is held at 1 % |

Deviations from this plan:
- **`tile_center` is a share of the layer, not pixels** (AE counts pixels). A plugin default cannot depend on the layer's size; shares stay central when a layer is resized (as `barrel`, `twirl`, `vanish` do). Recorded in the spec.
- **`output_width` / `output_height` range 1 to 1000 %**, AE's is up to 32 000 %: a layer's picture is capped at 16384 px a side and 64 Mpx (`eval/buffer.rs`). Past the cap is an error naming the layer.
- **The two switches are `choice off/on`** because the engine has no checkbox parameter type (same as `displacement_map`).
- **Gutter colour and size are not in AE**: BUILD_PLAN WP-34 lists them for Motion Tile; the default (0) makes a use without them AE's.
- **`@cost` is not declared**: the plan's authoring rules name it, but the engine has no `@cost` header tag (it is not in `fxlib/header.rs`), and adding one is outside this task. The plugin does 1 to 25 bilinear taps per pixel and one image.
- **Grammar extension of `@bounds`** (`box_w`, `box_h`): needed because the growth is a share of the layer; it is the only engine change outside the plugin, with its own tests.
- **Reduction is a bounded average** (at most 5 x 5 taps), not an exact box filter, so reductions past 5:1 alias a little. The plan caps loops at 32 taps.
- The probe card is 160 x 96, not 160 x 90: at 90, a 50 % tile is 45 px high and a tile edge lands on a pixel centre, where `floor(x + 0.5)` is a tie that float noise decides. That is a property of the definition, not a bug; the probe avoids it, and the spec's rule is "a pixel is in when its centre is".

Open questions for the owner:
1. **Phase rule.** I implemented cumulative per-row (column) shift (each row `s` pitches further than the one before, EffectCraft's rule). The alternative, "every other row is shifted by the phase", agrees at 0, 180 and 360 degrees and differs at 90. One After Effects dump of a 90-degree phase would settle it; `make.py` and the unit tests encode the current rule.
2. **Mirror Edges, 1-pixel overlap.** In AE, mirrored neighbours may repeat the shared edge pixel or not; mine does not repeat it (the tile is the whole layer, the flip is exact). Needs one dump to confirm.
3. **Tile Center in pixels.** Should the plugin grow a pixel-unit vec2 flavour (a `@param ... px` type) so that AE projects import without conversion? Not needed for authoring.
4. **Tie rule.** A pixel centre exactly on a tile edge goes to the higher-numbered tile (`floor(x + 0.5)`). Fine for the film; say if AE rounds the other way.

Estimated cost to finish: none for the engine; the open questions need a few AE reference stills (about 1 hour once available).
