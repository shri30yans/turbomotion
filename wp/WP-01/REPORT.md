WP: WP-01 Layer-space effect pipeline and bounds (opt-in `"effect_space": "layer" | "screen"`, default `"screen"`)

Status: done

Changed files:
- Engine, planning: `crates/comp/src/schema/{effect_space.rs (new), mod.rs}`, `plan/{layer_space.rs (new), layer.rs, mod.rs}`, `eval/{buffer.rs (new), mod.rs}`
- Engine, render: `render/{local.rs, passgeo.rs (new), fx.rs, content.rs, post.rs, composite.rs, history.rs, mod.rs}`, `render/graph/{local.rs (new), mod.rs, build.rs, run.rs, describe.rs}`
- Core: `crates/core/src/layer.rs` (`place_about`), `crates/core/src/effects/{bounds.rs, tests.rs (new; the registry tests moved out of mod.rs), pass.rs, mod.rs, place.rs, blur.rs, distort.rs, texture.rs}`, `crates/core/src/shaders/effects/vignette.sksl`
- Plugins: `crates/comp/src/fxlib/{bounds.rs (new), header.rs, compile.rs, mod.rs, prelude.sksl}` and all 70 `fxlib/builtin/*.sksl` (`@bounds`, `in_buffer` clamps instead of resolution clamps)
- Check/lint: `check/{fx_bounds.rs (new), lint.rs, mod.rs}`, `tools/reframe.rs`
- SDK: `crates/script/sdk/film-types.ts` (`CompOpts.effect_space`)
- Docs/ledger: `catalogue.rs`, `catalogue/effect_space.rs (new)`, `crates/cli/src/{docs.rs, parity/check.rs, parity/mod.rs}`, `docs/reference/{effects.md, parity.md}` (generated), `parity/{MATRIX.tsv, effects.tsv}`, `parity/specs/effect_space.md (new)`
- Outside `turbomotion/`: `scripts/capability_index.py`, `scripts/test_capability_index.py` (a `comp-key` entry), `motions/lib/CAPABILITIES.md` (regenerated). `scripts/capability_sdk.py` untouched.
- New under `wp/WP-01/`: `REPORT.md`, `probes/{comp.json, layer_space_probe.png, README.md}`, `identity/{snap.sh, before.tsv, after.tsv, README.md}`
- File length: new files are all under 300 lines. `eval/mod.rs` went from 783 to 749 lines (the new planning code is `eval/buffer.rs`; its `light_tests` moved to `eval/light_tests.rs`). Five files that were already over 300 grew by declarations only: `fxlib/mod.rs` 716->730, `plan/mod.rs` 453->460, `render/mod.rs` 332->339, `schema/mod.rs` 495->501, `check/lint.rs` 357->358 (see Deviations).

Tests added (all pass; none ignored):
- Acceptance, `render/layer_space_tests.rs`: `a_blur_tail_doubles_with_the_layer_scale_in_layer_space`, `a_blur_tail_is_the_same_at_every_scale_in_screen_space_as_before`, `a_blur_uses_content_outside_the_frame_in_layer_space`, `a_glow_and_a_blur_of_a_square_wholly_outside_the_frame_bleed_into_it_in_layer_space`, `a_layer_scaled_into_the_frame_is_blurred_as_a_whole`, `a_directional_blur_turns_with_the_layer_in_layer_space`, `the_default_is_the_screen_pipeline_and_a_render_repeats_exactly`, `a_mask_is_drawn_on_the_layers_pixels_so_it_scales_with_the_layer`
- Buffer sizing through a real evaluated layer, `eval/buffer_tests.rs`: `the_picture_is_the_layer_grown_by_its_blur_and_not_the_frame`, `the_layer_scale_does_not_change_the_picture_it_is_drawn_into`, `effects_add_up_along_the_chain_and_each_grows_the_box_the_one_before_left`, `a_directional_blur_grows_the_picture_along_its_angle_only`, `content_larger_than_the_frame_keeps_its_whole_picture`, `a_layer_without_effects_or_masks_and_every_screen_space_layer_has_no_picture`, `inspect_shows_the_three_boxes_of_a_layer_space_layer_and_nothing_for_a_screen_layer`; and the pure planner tests in `eval/buffer.rs` (6)
- Graph stages, `render/graph/layer_space_tests.rs`: `a_layer_with_effects_is_painted_in_its_own_pixels_and_placed_by_the_composite`, `the_default_pipeline_still_paints_a_frame_sized_picture`, `masks_come_after_the_effects_and_before_the_transform`, `a_layer_that_is_matted_is_placed_before_the_matte_which_works_in_the_frame`, `the_blur_of_a_transition_runs_in_the_frame_after_the_layer_is_placed`, `a_layer_without_effects_or_masks_is_painted_in_the_frame_even_in_layer_space`, `twelve_layer_space_layers_never_hold_more_than_three_pictures_at_once`, `the_description_names_the_largest_picture_a_layer_is_painted_into`
- Step 1 reproduction and what plugins are told, `render/layer_space_plugin_tests.rs`: `in_screen_space_a_plugin_sees_the_whole_frame_as_its_resolution_and_its_picture`, `in_layer_space_a_plugin_sees_the_layer_as_its_resolution_and_a_picture_grown_by_its_bounds`, `a_plugin_that_moves_pixels_outside_the_layer_is_clipped_in_layer_space_unless_it_declares_bounds`; `render/passgeo.rs` (4)
- Schema, planning, lint: `tests/effect_space_schema.rs` (`both_values_load_and_the_default_is_screen`, `a_mistyped_key_names_the_nearest_valid_one`, `a_mistyped_value_lists_the_valid_ones`), `plan/layer_space.rs` (7 incl. every refusal), `check/fx_bounds.rs` (2), `schema/effect_space.rs` (1)
- Bounds: `core/src/effects/bounds.rs` (9: each growing effect's reach, `GROWING` equals the effects that grow), `fxlib/bounds.rs` (4: `@bounds` grammar and errors)
- Catalogue, SDK, ledger: `catalogue/effect_space.rs::the_entry_names_the_values_the_parser_reads_and_its_example_asks_for_layer_space` (the example also runs in `examples_check`), `script/tests/sdk.rs::comp_effect_space_reaches_the_json_and_is_left_out_by_default`, `script/tests/sdk_option_types.rs::the_effect_space_option_is_typed_with_exactly_the_engines_names`, `cli parity::check::tests::grows_bounds_must_say_what_the_engine_says`, `scripts/test_capability_index.py::test_the_effect_space_key_of_the_composition_is_found_by_name_and_by_what_it_does`

Acceptance numbers (measured on rendered frames, red >= 3/255 counts as tail):

| measurement | screen (before and now) | layer |
| --- | --- | --- |
| `blur` radius 10, 4x4 square: lit pixels each side of the centre, scale 1 / scale 2 | 15 / 22 | 15 / 30 (exactly 2x) |
| `blur` radius 10, 60 px square: tail past the edge, scale 1 / scale 2 | 22 / 22 (equal, pinned) | 22 / 45 (2x within the stated 2 px tolerance) |
| off-frame square (right edge 10 px left of the frame), `blur` r10 or `glow` r10: red at x = 2 | 0 | >= 8 and columns 0..30 equal to the same square drawn in frame within 2 levels |
| half-off-frame 100 px square, scale 1 and 2: columns inside the frame vs the whole square | differ by more than 20 levels | within 2 levels |
| `directional_blur` 40 px, angle 0 on a 20 px square rotated 90 degrees: lit across / tall | >= 55 / <= 25 (along the frame) | <= 25 / >= 55 (along the layer) |
| buffer of a 40x40 layer with blur r10, in a 1920x1080 frame | frame-sized (16.6 MB) | 100x100 (80 KB) at any scale |
| 100 layers of 40 px with blur r6: `comp graph` "largest layer picture", peak images | 1920x1080 px, 2 | 76x76 px, 2 |

Gate output (all run in `turbomotion/` unless noted, `unset CARGO_TARGET_DIR`, foreground):

1. `cargo test --release --workspace --no-fail-fast`: exit 0, 39 test binaries, **514 passed, 0 failed, 2 ignored** (the 2 ignored predate this WP: `pose_matrices_match_the_after_effects_dump` needs the AE dump file, `actual_typescript_compiler_checks_types_without_executing_the_film` needs `TURBOMOTION_TEST_TSC`). Largest: `test result: ok. 302 passed; 0 failed; 0 ignored` (comp lib).
2. `cargo run --release -p turbomotion -- docs --write` -> `wrote 7 pages to docs/reference`; `docs --check` -> `docs are current (7 pages in docs/reference)`; `git diff --stat docs/`: `docs/reference/effects.md | 138 +++`, `docs/reference/parity.md | 2 +-` (only those two).
3. `python3 scripts/capability_index.py` -> `motions/lib/CAPABILITIES.md: 670 entries`; `... find effect_space` -> `comp-key | effect_space | Where a layer's effects and masks run. "screen" (the d...`. `python3 -m unittest test_capability_index test_capability_sdk` in `scripts/`: OK.
4. `target/release/turbomotion parity check` -> `parity check: ok (63 core rows, 264 effect rows, 97 catalogue effects)`, exit 0. `grows_bounds` is now set on all 91 ledger rows that name engine effects (30 `yes`, 61 `no`, 0 `?`), and `parity check` fails if a row disagrees with the catalogue's `bounds`.
5. `target/release/turbomotion comp check wp/WP-01/probes/comp.json` -> `ok: 1300x720 at 30 fps, 2.00s (60 frames), 20 layers, 10 nested comps; 0 assertions; 0 errors, 0 warnings`.
6. Determinism: two renders of the probe, `shasum -a 256` of the PNG: `3fe990e13edf2a6a...` both (and equal to the saved `layer_space_probe.png`). Layer-space renders are also compared byte for byte in `the_default_is_the_screen_pipeline_and_a_render_repeats_exactly`.
7. Probe images: `wp/WP-01/probes/layer_space_probe.png` (top row `screen` with an orange rule, bottom row `layer` with a cyan rule; one-line descriptions with numbers are in `probes/README.md`): scale (blur halo 18 px a side vs 46 px, 2.55x), rotate (smear along the frame vs along the bar), offscreen (glow bleeds 29 columns into the frame vs 0), glitch (triage 002: slices are 249 px wide, unclipped at the text box, vs 179), echo (triage 005: trail follows the text's own baseline).

Default-mode byte identity (plan step 10, `wp/WP-01/identity/snap.sh`): baseline binary built from commit 5dff2ab before any change (`before.tsv`) against this tree (`after.tsv`); `diff before.tsv after.tsv` is empty, **49 of 49 inputs identical** (10 `film.ts` of `motions/` built with `sdk build`, sha256 of the JSON; stills at 0.5 s and 1.2 s of every `examples/comp/*/comp.json`, the SDK examples and the 10 built films, sha256 of each PNG; the first 16 hex digits are in the files, the first 8 below as before / after):

| input | stills / json | result |
| --- | --- | --- |
| `m/launch-film_film.ts` | built-json 5688b9a6 / 5688b9a6 | identical |
| `m/figma-anthem_film.ts` | built-json fca73e59 / fca73e59 | identical |
| `m/flow-state_film.ts` | built-json 96b75c7a / 96b75c7a | identical |
| `m/reel-clones_04-newspaper-match-cut_film.ts` | built-json b2578c77 / b2578c77 | identical |
| `m/examples_micro-courage-reel_film.ts` | built-json 84f007f4 / 84f007f4 | identical |
| `m/examples_snapgram-rebuild_snapgram_film.ts` | built-json a5e616cc / a5e616cc | identical |
| `m/examples_media-effects-reel_film.ts` | built-json 5e159efd / 5e159efd | identical |
| `m/examples_weave-looks_butterfly_film.ts` | built-json e887c591 / e887c591 | identical |
| `m/examples_weave-looks_bloom_film.ts` | built-json 9c15d959 / 9c15d959 | identical |
| `m/inspiration_artlist-ae-50_rebuilds_001-esports-momentum_film.ts` | built-json c1270fe3 / c1270fe3 | identical |
| `ex/comp_01-shapes_comp.json` | 0000.50 e6d87e9c / e6d87e9c; 0001.20 7731a1b2 / 7731a1b2 | identical |
| `ex/comp_02-type_comp.json` | 0000.50 878232c5 / 878232c5; 0001.20 5bc2a250 / 5bc2a250 | identical |
| `ex/comp_03-media_comp.json` | 0000.50 d5c9a9dc / d5c9a9dc; 0001.20 8b019beb / 8b019beb | identical |
| `ex/comp_04-plugins_comp.json` | 0000.50 5f8fefd0 / 5f8fefd0; 0001.20 f831fe58 / f831fe58 | identical |
| `ex/comp_05-particles_comp.json` | 0000.50 52c61f7c / 52c61f7c; 0001.20 86b449ad / 86b449ad | identical |
| `ex/comp_06-3d_comp.json` | 0000.50 c079fdbc / c079fdbc; 0001.20 01d1d1e3 / 01d1d1e3 | identical |
| `ex/comp_07-blocks_comp.json` | 0000.50 daf59a98 / daf59a98; 0001.20 513e5929 / 513e5929 | identical |
| `ex/comp_09-look_comp.json` | 0000.50 aba95218 / aba95218; 0001.20 eee96495 / eee96495 | identical |
| `ex/comp_11-adopt_comp.json` | 0000.50 fbbea38b / fbbea38b; 0001.20 51f84231 / 51f84231 | identical |
| `ex/comp_12-depth_comp.json` | 0000.50 439c6078 / 439c6078; 0001.20 d6b7cf51 / d6b7cf51 | identical |
| `ex/comp_14-hits_comp.json` | 0000.50 66817d34 / 66817d34; 0001.20 adca9008 / adca9008 | identical |
| `ex/comp_15-looks_comp.json` | 0000.50 d525e188 / d525e188; 0001.20 255ac27e / 255ac27e | identical |
| `ex/comp_16-timeline_comp.json` | 0000.50 3bf3b4a1 / 3bf3b4a1; 0001.20 a0839aa9 / a0839aa9 | identical |
| `ex/comp_17-text3d_comp.json` | 0000.50 2db086fd / 2db086fd; 0001.20 7d7cd958 / 7d7cd958 | identical |
| `ex/comp_18-transitions_comp.json` | 0000.50 e0dc18ac / e0dc18ac; 0001.20 c0ee7372 / c0ee7372 | identical |
| `ex/comp_19-solids_comp.json` | 0000.50 4a937454 / 4a937454; 0001.20 e4dc0883 / e4dc0883 | identical |
| `ex/comp_showcase_comp.json` | 0000.50 c39b44b7 / c39b44b7; 0001.20 df9a3d37 / df9a3d37 | identical |
| `ex/sdk_launch_film.comp.json` | 0000.50 0b8abe74 / 0b8abe74; 0001.20 fe86749d / fe86749d | identical |
| `ex/sdk_components_appear.comp.json` | 0000.50 d6c616e2 / d6c616e2; 0001.20 2e81ded0 / 2e81ded0 | identical |
| `ex/sdk_components_artwork.comp.json` | 0000.50 5b2e1e6a / 5b2e1e6a; 0001.20 607cc72a / 607cc72a | identical |
| `ex/sdk_components_blob.comp.json` | 0000.50 d344650a / d344650a; 0001.20 fb7066a9 / fb7066a9 | identical |
| `ex/sdk_components_cubes.comp.json` | 0000.50 bf154dd3 / bf154dd3; 0001.20 da0f08c3 / da0f08c3 | identical |
| `ex/sdk_components_dolly-zoom.comp.json` | 0000.50 25cb0a14 / 25cb0a14; 0001.20 f69c8516 / f69c8516 | identical |
| `ex/sdk_components_flip-tile.comp.json` | 0000.50 60081bba / 60081bba; 0001.20 549163ea / 549163ea | identical |
| `ex/sdk_components_glass-panel.comp.json` | 0000.50 c6ae9fb5 / c6ae9fb5; 0001.20 eb4e60b3 / eb4e60b3 | identical |
| `ex/sdk_components_hands.comp.json` | 0000.50 e8edd9e7 / e8edd9e7; 0001.20 092c6ecc / 092c6ecc | identical |
| `ex/sdk_components_lens.comp.json` | 0000.50 8072d1dc / 8072d1dc; 0001.20 716b1ee3 / 716b1ee3 | identical |
| `ex/sdk_components_montage.comp.json` | 0000.50 a2b0f296 / a2b0f296; 0001.20 60e50804 / 60e50804 | identical |
| `ex/sdk_components_swarm.comp.json` | 0000.50 58f0fb26 / 58f0fb26; 0001.20 5999b279 / 5999b279 | identical |
| `built_m/launch-film_film.ts.comp.json` | 0000.50 c490159d / c490159d; 0001.20 41146510 / 41146510 | identical |
| `built_m/figma-anthem_film.ts.comp.json` | 0000.50 10279e27 / 10279e27; 0001.20 c0ab5d6d / c0ab5d6d | identical |
| `built_m/flow-state_film.ts.comp.json` | 0000.50 c5fa1465 / c5fa1465; 0001.20 072c82a1 / 072c82a1 | identical |
| `built_m/reel-clones_04-newspaper-match-cut_film.ts.comp.json` | 0000.50 46ecceb9 / 46ecceb9; 0001.20 b4e26bb8 / b4e26bb8 | identical |
| `built_m/examples_micro-courage-reel_film.ts.comp.json` | 0000.50 f3b0503e / f3b0503e; 0001.20 404b9d0c / 404b9d0c | identical |
| `built_m/examples_snapgram-rebuild_snapgram_film.ts.comp.json` | 0000.50 b529f861 / b529f861; 0001.20 a97251bd / a97251bd | identical |
| `built_m/examples_media-effects-reel_film.ts.comp.json` | 0000.50 c52f2801 / c52f2801; 0001.20 60cba16a / 60cba16a | identical |
| `built_m/examples_weave-looks_butterfly_film.ts.comp.json` | 0000.50 a2c899aa / a2c899aa; 0001.20 828883af / 828883af | identical |
| `built_m/examples_weave-looks_bloom_film.ts.comp.json` | 0000.50 7d673c23 / 7d673c23; 0001.20 68d0dbad / 68d0dbad | identical |
| `built_m/inspiration_artlist-ae-50_rebuilds_001-esports-momentum_film.ts.comp.json` | 0000.50 df0f21a3 / df0f21a3; 0001.20 8bf365c2 / 8bf365c2 | identical |

Deviations from this plan:
- **Plugin uniforms are `layer_origin` and `buffer_size`, not `origin`, and `resolution` stays the content size.** The plan says `resolution` is the buffer size and `origin` the buffer offset. Every shipped plugin assumes `xy / resolution` is 0..1 over the layer and clamps to `[0, resolution]`; with a grown buffer that would shift every pattern and clip every spread. So `xy` counts from the content box's top-left, `resolution` is the content box, `layer_origin` (the buffer's top-left relative to it, negative when it grew) and `buffer_size` are new, and the prelude has `in_buffer`/`in_buffer_x`. In screen space `layer_origin` = 0 and `buffer_size` = `resolution`, so plugins are unchanged byte for byte.
- **Layer-space buffers only for layers that have effects or masks.** A layer without either (and adjustment layers, `look`) is painted in the frame as before: nothing runs before the transform, and a buffer would only add a resample. The composite fuses `Place` (`Composite {placed}`) so a plain layer-space layer never makes a frame-sized picture.
- **Refused in layer comps, with a message that names the effect and `"effect_space": "screen"`:** `backdrop`, a mesh with effects or masks, a composite mask taken from another layer, plugin inputs that read another layer or the layer's own history. There is no per-layer override yet (an `effect_space` on a layer would remove this limit; I did not add a key the plan does not ask for).
- **Lighting and the blur or smear of a transition run after placement** (`Stage::Finish`), `clip_to`, matte, reflect and wipe stay in frame space after `Place`, as the plan says for mattes; lighting is per-frame-pixel by nature.
- **Order stays effects, then masks** (the plan's own text and WP-14 "masks before effects" row keep that open); masks are drawn on the layer's pixels and scale with it (`a_mask_is_drawn_on_the_layers_pixels_so_it_scales_with_the_layer`).
- **`@bounds source=<input>` parses but errors when growth is asked for** (it needs the source layer's own bounds at plan time; no shipped plugin uses it).
- **Growth is conservative** (Gaussian 3 sigma, disc reach to the farthest corner), so a buffer can be a little larger than strictly needed; never smaller.
- **Particles, textures and packs use the layer's `size`** as their content box (the frame unless the layer sets `size`); particle trails outside it are clipped in layer space.
- **`inspect`** shows the three boxes under `layers[].local` (`bounds_local`, `bounds_post_fx`, `bounds_screen`, plus `origin` and `size`) and only in layer space, so inspect output of a default film is unchanged; the plan said the names as top-level fields.
- **The `fx-needs-bounds` lint is a warning that only fires for plugins in layer-space layers** (a plugin with no `@bounds` is cut at the content box there; the lint names the plugin and the fix). The 70 shipped plugins all declare `@bounds`.
- **`comp graph` header gained "largest layer picture WxH px"** (the plan asked to record peak images; the count was the same in both modes, the size is what drops).
- **Ledger:** `parity check` now compares every row's `grows_bounds` with the catalogue (`yes` if any engine effect grows, `no` if all declare none; `?` stays allowed only for `todo`/`in_progress` rows), not just the vocabulary. New MATRIX row "Effects and masks run in layer space..." added; "Effect bounds (effects grow the layer)" is `done` with test `the_picture_is_the_layer_grown_by_its_blur_and_not_the_frame` and doc `parity/specs/effect_space.md`.
- **Step 1 finding differs from the plan's wording:** screen space does not clip glyphs to the layer bounds (the picture is frame-sized). What it does is lose off-frame content, run `glitch_slices` bands over the frame's height, and keep pixel parameters in screen pixels; in layer space the same clip appears for a plugin that spreads without `@bounds`. Numbers and mechanism are in `parity/specs/effect_space.md`.
- **Five files already over 300 lines grew by 1-14 lines** (module declarations, new fields and doc lines; list under Changed files). I extracted what I could (`eval/mod.rs` shrank 783 -> 749, the registry tests moved out of `core/src/effects/mod.rs`) but did not split `fxlib/mod.rs`, `plan/mod.rs`, `schema/mod.rs`, `render/mod.rs` or `check/lint.rs`, which would be a refactor unrelated to this WP.
- **Platform effects whose parameters are fractions of the layer** (twirl/bulge/radial blur centres, vignette, transform anchor) refer to the content box in layer space, not to the grown buffer, so `0.5` stays the layer's middle.

Design note:
- **Layer buffers.** `eval::buffer::LocalBuffer` plans a layer's picture at evaluation time: the content box (`local_bounds`; solids their size, text the measured box, shape path bounds, images/precomps/packs/particles/textures their size) grown by `chain_growth` and rounded out, with limits of 16384 px a side and 64 Mi pixels (past either, `check` fails naming the layer). `Stage::PaintLocal` draws content with the identity transform into it; its origin is recorded so `Space {origin, content}` lets builtin shaders see layer pixels. A layer costs its own size plus its effects' reach, not the frame.
- **Bounds growth.** Platform effects implement `Effect::grows(t, content)` (blur/glow 3 sigma, shadow offset+3 sigma, directional half its smear along the angle, radial in proportion to distance, twirl/bulge disc reach, transform bounding box, outline width, ...); plugins declare `@bounds none | expand=<expr> | expand_x=/expand_y= | source=`. Growth is summed along the chain over the running box, so a blur after a glow grows the box the glow left.
- **Transform resampling.** `Stage::Place` (or the composite directly) draws the buffer through the layer's matrix: Catmull-Rom when the layer is not shrunk (scale >= 0.999), bilinear with mips when it is, via `concat_44` so 3D layers keep perspective. Effects therefore run at layer resolution and enlargement is cubic (a blur at scale 2 is a 2x-enlarged blur, as After Effects does at 100% quality).
- **Matte/mask interplay.** Masks (`MaskCoverageLocal`) multiply the effect output in layer pixels, so they scale and turn with the layer. Anything defined on the frame (`clip_to`, track matte, reflect, wipe, lighting, transition blur) forces `Place` first and works on the placed picture; the graph test pins the order. Adjustment layers and `look` still see the composite below.
- **Memory.** Images are dropped after their last reader as before; the fused composite never makes a frame-sized transient for a plain layer-space layer.

Effects whose bounds growth is not yet declared (the 32 shader plugins in `motions/lib/effects/`, reported by `comp fx list --dir motions/lib` as `undeclared`; the `fx-needs-bounds` lint warns when one is used in a layer-space layer; I did not guess their reach): `anaglyph_stereo, ascii_filter, cell_glyphs, cloud_field, crt, dither, dither_sprites, dot_matrix, dust_scratches, film_grain, geographic_globe, god_rays, gradient_map, halftone_cmyk, ink_bleed, kaleido, light_streak, linocut, mandala, metaballs, mosaic_reveal, night_vision, paper_texture, pixel_avg, pixel_mosaic, pixel_smear, region_matte, rgb_glitch, riso_shift, roughen_edges, thermal_scan, torn_edge`, and `examples/comp/04-plugins/effects/edge_wipe.sksl`. Probable growers among them (the plan's step-5 list): `god_rays, light_streak, pixel_smear, roughen_edges, ink_bleed, rgb_glitch, riso_shift, torn_edge, pixel_avg`. All 70 plugins in `crates/comp/src/fxlib/builtin/` declare `@bounds` (53 `none`, 17 expansions) and the 27 platform effects state theirs in code (13 grow, 14 do not), 97 catalogue effects in all.

Open questions for the owner:
1. Per-layer override (`"effect_space"` on a layer) so a film can use layer space for most layers and keep `backdrop`/history plugins on screen ones? Today the comp key is all or nothing (a precomp can differ, since a nested comp takes its own value).
2. Should the default flip to `"layer"` after you review `motions/`? Nothing in this WP changes the default. Before flipping: the 32 undeclared lib plugins, and the five refused cases above, need a decision.
3. `echo`'s and every directional plugin's `angle` is in layer degrees in layer space (it turns with a rotated layer); is that the convention to document, or should plugins offer a screen-angle option?
4. Plugin `resolution` is the content size in layer space. A future plugin that wants the grown buffer's size must read `buffer_size`; confirm that naming (`layer_origin`/`buffer_size`) rather than the plan's `origin`.

Estimated cost to finish (if partial): not partial. Follow-ups outside this WP: declare `@bounds` on the 32 lib plugins (about 2 hours with a probe each), per-layer override (about 4 hours).

Next task per the plan: WP-02 Blur kernels and the blur family (depends on WP-01: correct Gaussian sigma, disc/box/lens kernels, no banding on large blurs, bounds per WP-01).
