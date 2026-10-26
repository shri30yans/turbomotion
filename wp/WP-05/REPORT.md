WP: WP-05 Blend modes

Status: done

Changed files:
- Engine (`turbomotion/`):
  - `crates/core/src/effects/blend.rs` deleted; replaced by `crates/core/src/effects/blend/{mod,table,programs,closed_form,tests,tests_modes,tests_dissolve}.rs`
  - `crates/core/src/shaders/blend.sksl`, `crates/core/src/shaders/dissolve.sksl` (new)
  - `crates/core/src/effects/mod.rs` (`pub mod blend`, re-exports), `crates/core/src/shaders.rs` (compiles the two programs), `crates/core/Cargo.toml` (feature `reference`)
  - `crates/comp/src/render/blend.rs`, `crates/comp/src/render/blend_tests.rs` (new), `render/mod.rs`, `render/composite.rs`, `render/graph/{run,build,describe}.rs`, `eval/mod.rs`, `crates/comp/Cargo.toml` (dev-dependency on the `reference` feature)
  - `crates/comp/src/catalogue/blend_modes.rs` (new), `catalogue.rs`
  - `crates/cli/src/docs/blend.rs` (new), `crates/cli/src/docs.rs`
  - `crates/script/sdk/blend-types.ts` (new), `sdk/types.ts`, `sdk/index.ts`, `crates/script/tests/sdk_option_types.rs`, `crates/script/Cargo.toml`, `Cargo.lock`
  - `docs/reference/blend-modes.md` (new, generated), `docs/reference/README.md`, `docs/reference/sdk.md` (generated)
  - `parity/MATRIX.tsv`, `parity/specs/blend_modes.md` (new)
  - `wp/WP-05/{REPORT.md, probes/*, identity/*}` (new)
- Outside `turbomotion/`: `scripts/capability_index.py`, `scripts/test_capability_index.py`, `motions/lib/layer_keys.json`, `motions/lib/ae_terms.json`, `motions/lib/CAPABILITIES.md` (regenerated). `scripts/capability_sdk.py` untouched.

Tests added (37 Rust, 1 Python):
- core, `effects/blend/tests.rs`: `the_table_has_one_row_per_variant_in_the_order_of_the_enum`, `names_are_unique_lowercase_words_and_every_group_has_a_mode`, `the_original_seventeen_modes_are_drawn_by_the_skia_modes_they_always_were`, `the_shared_blender_numbers_exactly_the_program_rows`, `a_name_or_an_alias_parses_and_a_misspelling_names_the_nearest_mode_and_lists_them_all`, `only_the_dissolves_and_the_two_stencils_that_reach_past_the_layer_need_the_frame`
- core, `tests_modes.rs`: `every_mode_but_the_dissolves_equals_its_closed_form_on_every_pair` (567 pairs per mode), `values_worked_out_by_hand`, `the_classic_modes_differ_from_the_new_ones_at_the_edges_and_under_opacity`, `darker_and_lighter_colour_compare_the_sum_of_the_channels_not_the_luma`, `alpha_add_adds_alphas_and_closes_a_seam_that_over_leaves`, `luminescent_premul_keeps_light_above_alpha_that_normal_would_clip_to_it`, `stencils_and_silhouettes_act_on_what_is_below_and_a_transparent_layer_is_the_edge_case`, `the_non_separable_modes_keep_the_luminosity_and_hue_they_are_defined_by`, `a_transparent_layer_changes_nothing_except_in_the_stencils_and_a_transparent_base_shows_the_layer`, `opacity_scales_the_layer_before_the_mode_sees_it`
- core, `tests_dissolve.rs`: `the_share_of_pixels_kept_is_the_opacity`, `the_share_is_the_same_in_every_part_of_the_area`, `a_pixel_is_the_layer_whole_or_what_is_below_whole`, `the_pattern_depends_on_the_pixel_and_the_seed_alone`, `the_pattern_is_noise_neighbours_and_frames_are_unrelated`, `the_pattern_is_flat_across_its_whole_range`
- comp, `render/blend_tests.rs` (GPU, whole renderer): `every_mode_over_an_opaque_backdrop_shows_the_closed_form_level_within_one`, `the_modes_that_reach_past_their_layer_show_the_closed_form_when_the_layer_covers_the_frame`, `multiply_screen_and_overlay_in_linear_light_differ_from_display_numbers_by_what_the_curve_says`, `every_mode_in_linear_light_shows_the_closed_form_of_the_light_within_one_level`, `a_dissolve_keeps_the_share_of_a_large_flat_area_that_its_opacity_says`, `a_dissolve_repeats_exactly_and_only_the_dancing_one_changes_from_frame_to_frame`, `the_alpha_stencil_keeps_the_backdrop_by_the_layers_alpha_and_clears_it_where_the_layer_is_not`, `the_alpha_silhouette_cuts_the_backdrop_by_the_layers_alpha_and_leaves_it_where_the_layer_is_not`, `the_luma_stencil_and_silhouette_weigh_the_backdrop_by_the_brightness_of_the_layer`, `a_layer_with_effects_in_layer_space_is_placed_before_it_stencils_so_the_stencil_reaches_the_frame`, `an_effects_composite_blend_takes_every_mode_as_a_layer_does`, `a_film_with_every_mode_renders_the_same_bytes_twice`, `a_misspelt_mode_on_a_layer_is_refused_with_the_nearest_mode_and_the_list`
- comp, `catalogue/blend_modes.rs`: `the_entry_lists_every_mode_of_the_table_in_order_each_with_a_composition_that_loads_in_that_mode`. The 37 catalogue examples and 37 docs blocks also run in the existing `examples_check` harness (`every_catalogue_example_passes_check` and the docs one), unchanged and not loosened.
- script, `sdk_option_types.rs`: `the_blend_option_is_typed_with_exactly_the_engines_names_in_its_order`
- python, `scripts/test_capability_index.py`: `test_every_blend_mode_is_found_by_name_and_by_what_it_does`

What was built:
- One mechanism (`effects/blend`): `Blend` enum + one table `MODES` (name, AE menu group, summary, how drawn, whole-frame flag). Parser, did-you-mean, schema names, catalogue, docs page, SDK type test and the choice of draw all read the table. `Blend::stage` is the only way a mode is drawn; the layer compositor and an effect's `composite.blend` both call it through `render/blend.rs`.
- 17 original modes: the same Skia modes as before. `stencil_alpha`, `silhouette_alpha`, `luminescent_premul`: Skia `DstIn`, `DstOut`, `SrcOver`. 15 modes: one shared SkSL runtime blender, mode a uniform numbered by table order. Dissolves: a shader pre-pass with an exact whole-number Feistel pattern (SkSL runtime effects have no unsigned ints or bit operators). The stencils and dissolves need a frame-sized picture, so a layer-space layer is placed first (`Mode::whole_frame`).
- Definitions, tolerances and known limits: `parity/specs/blend_modes.md`. Examples of every mode: `docs/reference/blend-modes.md`. Probes: `wp/WP-05/probes/`. Identity tables: `wp/WP-05/identity/`.

Gate output:
1. `cargo test --release --workspace --no-fail-fast`: 40 test binaries, `passed 590 failed 0 ignored 2` (before WP-05: 553 passed, 2 ignored; +37). Tail: `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`. No existing golden changed.
2. `cargo run --release -p turbomotion -- docs --write` -> `wrote 8 pages to docs/reference`; `git diff --stat docs/` -> `docs/reference/README.md | 1 +`, `docs/reference/sdk.md | 9 +++++++++` (plus the new untracked `blend-modes.md`); `docs --check` -> `docs are current (8 pages in docs/reference)`.
3. `python3 scripts/capability_index.py` -> `motions/lib/CAPABILITIES.md: 712 entries`; `find dissolve` -> `blend-mode | dissolve | Each pixel is either the layer or what is below, chosen by a fixed random pattern ... | blend=dissolve group=normal`; `find stencil` -> `blend-mode | stencil_alpha | Everything below shows only where the layer is opaque ... | blend=stencil_alpha group=matte`. `python3 -m unittest discover -s scripts -p 'test_*.py'` -> `Ran 47 tests ... OK`.
4. `turbomotion parity check` -> `parity check: ok (77 core rows, 264 effect rows, 97 catalogue effects)`, exit 0.
5. `turbomotion comp check` on the five probe films (`modes`, `display`, `light`, `old-modes`, `old-modes-light`): each `ok: ... 0 assertions; 0 errors, 0 warnings`.
6. Determinism: two stills runs of `modes.comp.json` at 0.5 s and 0.8 s: `2a0e28915cc7d4f8 2dfbea26309b8ead` both times; two `--codec software --workers 1` renders of frames 0..12: `MD5=f372191bdfc2a793b634554539f68e6e` both times (the video stream); the in-test render-twice (`a_film_with_every_mode_renders_the_same_bytes_twice`, `a_dissolve_repeats_exactly_...`) pass.
7. Probe images in `wp/WP-05/probes/`: `modes.png` shows all 37 modes at opacity 0.85 on one backdrop (dissolves are visibly speckled, stencils keep the ground only inside the layer's shape, silhouettes cut it out); `display.png`/`light.png`/`compare.png` show multiply, screen and overlay on a flat 64,128,192 backdrop: the disc centre is 56,24,96 / 232,152,224 / 112,49,192 on display numbers and 55,20,95 / 226,135,208 / 78,31,139 in linear light. Numbers per mode: `probes/README.md`.

Acceptance numbers:
- Closed form, 567 pairs per mode (9 colours x 9 colours x 7 alpha pairs incl. partial alpha and a transparent side), 32-bit float raster: worst error 3.8e-6 (Skia's colour dodge and burn), bound 1e-5 (a bit under a 400th of one 8-bit level, 2.6x the measured worst). Other modes: <= 2.3e-6 (hue, saturation), 9.5e-7 (colour, luminosity), ~6e-8 (the shader's own modes).
- Whole renderer on the GPU (half-float pictures, 8-bit output), every mode, two colour pairs, two opacities: worst 0.88 level of 255, bound 1 level. In linear light, every mode at opacity 0.5 against the closed form of the light, shown over black and encoded: within 1 level, and at least 20 modes show a different picture from the display-number film.
- Linear vs display (backdrop 64,128,192, layer 224,48,128): multiply differs by up to 5.4 levels, screen 17.7, overlay 53.2; each matches the sRGB-curve prediction within 1 level.
- Non-separable, four pairs whose results need no clipping, within 1e-5: the luminosity result has the layer's luma and moves the base along the grey axis; colour and hue keep the base's luma (hue also its saturation); saturation takes the layer's saturation. The clipped cases are covered by the closed-form table.
- Dissolve, 512 x 512 flat area (262144 pixels, standard deviation of a share <= 0.001): kept share within 0.0045 of opacity at 0.05, 0.1, 0.3, 0.5, 0.75, 0.9, 0.95, and at 15 evenly spaced thresholds; every 128 x 128 block within 0.016 of 0.3; pixels 1, 2, 3, 5, 8, 64 apart and frames 100/101/102/1000/70000 agree within 0.0045 of chance; two renders identical bytes; plain dissolve identical across frames, dancing one agrees with itself between frames 100 and 101 only by chance (0.5 +- 0.0045). In the film (GPU) the same shares and byte identity hold.
- Stencils/silhouettes (premultiplied colour over black): alpha stencil at 0.5 keeps 0.5 of the backdrop inside the layer and 0 outside (also for a rotated layer-space layer); alpha silhouette 0.5 inside, 1.0 outside, opaque layer cuts a hole; luma at #808080: 128/255 and 1 - 128/255 of the backdrop inside, 0 and 1 outside; all within 1 level.
- 17 original modes: `the_original_seventeen_modes_are_drawn_by_the_skia_modes_they_always_were` plus the byte identity below.

Byte identity of the default (baseline = `git archive HEAD` built in an empty directory, deleted afterwards):

| set | rows | before = after |
| --- | --- | --- |
| stills at 0.5 s and 1.2 s of `examples/comp/*` | 34 | yes |
| stills of `examples/sdk` (launch film, components) | 24 | yes |
| stills of ten built SDK films | 20 | yes |
| built film JSON (`sdk build`) of those ten | 10 | yes |
| stills of the WP-05 probes made only of the original 17 modes (partial alpha, default and linear light, multiply/screen/overlay) | 8 | yes |
| mp4/mov streams of five examples x software, alpha codecs | 10 | yes |

`diff before.tsv after.tsv` and `diff video-before.tsv video-after.tsv`: empty; `shasum` of both still tables `2964de02dcaaf0e6`. Tables: `wp/WP-05/identity/`.

Deviations from this plan:
1. The plan puts the shader blend "in `Stage::Composite`". It is in `Blend::stage`, which both the layer compositor and an effect's `composite.blend` call: one mechanism, no per-site special cases, and no new graph stage (the existing `Place` stage supplies the frame-sized picture a stencil or dissolve needs).
2. The plan says the formulas come from EffectCraft `color/src/blend.rs`. Used, with these differences (reasons in `parity/specs/blend_modes.md`): darker/lighter colour compare the sum of channels (Adobe's description), not luma; `alpha_add` rescales the colour so abutting edges of one colour close seamlessly; `divide` and the classic burn clamp to 0..1; inputs of the new modes are clamped to 0..1 (8/16 bpc modes, not 32 bpc).
3. Tests: 567 pairs per mode (9 x 9 colours x 7 alpha pairs), more than "3x3 pixel pairs", in the working-space numbers (core) and on the GPU in both display and linear light (comp), instead of a separate linear run of the core table, because the mode sees working-space numbers only (Skia does the conversion).
4. A new `reference` feature on `turbomotion-core` exposes `closed_form` so the comp tests use the same closed forms instead of a second copy.
5. `comp inspect` now names modes by their parse names (`soft_light`, `color_dodge`, `dancing_dissolve`) where it printed the lowercased variant (`softlight`, `colordodge`) before. Renders are unaffected; any script reading the old inspect spelling would see the change.
6. `docs/reference/blend-modes.md` is 2688 lines because each of the 37 modes carries its own self-contained composition, printed by the existing JSON block writer.
7. `docs/BUILD_PLAN.md` still says `core/src/effects/blend.rs`; it is now a directory. Not edited (plan owner's file).

Open questions for the owner:
1. Luma stencils and silhouettes weigh the working-space numbers, so in a linear-light film a mid grey stencil keeps less than half; luma mattes read display numbers. Which should the stencils follow?
2. A stencil clears the whole frame outside the layer, including the composition `bg` (my reading of After Effects; not measured). Right?
3. `luminescent_premul` equals `normal` in a half-float engine (EffectCraft too). Listed so films that name it load; the ledger row is `part`.
4. Darker/lighter colour by channel sum (Adobe's wording) vs luma (EffectCraft). Which is the reference?
5. `alpha_add` seam: rescaled colour (mine) vs EffectCraft's premultiplied sum (seam darkens to 0.75). Which does After Effects show?
6. `divide` by zero (white where the base is lit, black where it is black) and the classic modes' 8/16 bpc clamping are from EffectCraft, not measured on After Effects. The classic fade rule is EffectCraft's issue #494, measured by its reporter at 32 bpc.
7. Pre-existing, found while writing the linear-light tests (WP-03): a solid or shape smaller than the frame in a `linear_light` film is quantised to 8 bits of light in the darks before anything blends it (levels 8 and 16 both show 13, 24 shows 22, 32 shows 34, 48 shows 50; full-frame solids are exact; above level 64 all exact). The linear-light tests therefore use full-frame solids. It belongs to the colour pipeline, not blending; should a WP fix it?
8. The default mp4 encoder (hardware) is not byte-deterministic from run to run on this machine (`01-shapes`: two renders gave different stream hashes); `--codec software` is. Pre-existing; the determinism gate used the software codec and stills.
9. The dancing dissolve's seed is `round(t * fps)` of the composition being drawn; inside a nested composition with its own clock that is the nested frame. Acceptable?

Estimated cost to finish (if partial): none, status done.
