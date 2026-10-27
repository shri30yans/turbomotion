WP: WP-33 finish stack: Add Grain, film halation and output dithering (three pieces, nothing else touched).

Status: done, with two disclosed limits. (1) After Effects cannot be measured here, so Add Grain and halation are held to their own written formulas (`parity/specs/add_grain.md`, `film_halation.md`), measured against the pixels; the ledger rows are `done` because test, spec and probe exist, not because they were compared with After Effects. (2) The engine's Skia blur is exact only up to sigma 4; the halation is exact (4.000 luma for a 4.000 source) at sigma 4 and about 4% low at its peak for a wide halo. Fixing the blur would change glow, bloom and shadow, so it was left alone (open question 1). Nothing committed or pushed.

## Audit (step 0), what existed

| thing | where | finding |
|---|---|---|
| `grain` | `core/effects` | white per-pixel, luma-blind, mono, strength in 0..1 of 255; same noise at every size; kept (existing films depend on its bytes) |
| `film_grain` | `core/effects` | one fixed 2-tap-wide clump, colour fixed, no size/softness/aspect, frame key from time; kept (byte identity) |
| `halation` (legacy) | `core/effects` | a tinted, thresholded blur on display numbers with no energy rule; kept; the new effect is `film_halation` because a plugin cannot reuse a platform effect's name |
| `bloom`, `glow` | `core/effects` | both use the Skia blur; they inherit its limit above sigma 4 (measured, below) |
| `look` | `comp` | per-film colour grade; no grain/halation/dither in it |
| gradient dither | `core/shaders/gradient` | a private 1-level noise to hide banding inside one fill; kept (byte identity); not the output dither |
| `dither` plugin | `fxlib/builtin` | a per-layer effect on the float picture, so it is rounded again afterwards; kept |
| final 8-bit step | `render/mod.rs` `draw_state` | one `draw_image` of the F16 composite onto the BGRA8888 surface: this is the only place the picture is quantised, and the only place the new dither is applied |

Decision on legacy grains: not reimplemented. Add Grain is a new, separate plugin (`add_grain`) with a different noise model; the two legacy effects are measured in `wp/WP-33/finish/legacy-grain-table.md` (`grain` lag-1 autocorrelation -0.003, mono; `film_grain` 0.47, R-G 0.79) and kept because rewriting them changes the bytes of existing films. There is no duplicate noise implementation inside the new code: `add_grain` is the only place the lattice noise lives.

## Design note, where each stage sits

```
layer pixels -> effects (add_grain: @space display, @bounds none; film_halation: @space linear, grows 3 sigma)
             -> composite in F16 (premultiplied, working space)
             -> output-profile transfer (Skia colour management, only when the film is managed)
             -> [dither: rank-pattern threshold on the float picture, ONCE]
             -> 8-bit round when drawn to the BGRA8888 frame -> encoder (libx264 / videotoolbox / prores)
```

* **Add Grain** (`crates/comp/src/fxlib/builtin/add_grain.sksl`). Lattice Gaussian-kernel noise (kernel sigma 0.75 cell, 4 x 4 taps, uniform +-sqrt(3) lattice values), cell length = `size` px stretched by `aspect`; autocorrelation `exp(-lag^2 / (L^2 + 4 sigma_soft^2))`; amplitude std = `intensity * 0.04 * 255 * tone` (plus 1/12 for the rounding); colour: shared grain and per-channel grain mixed by `sqrt(1-s)` and `sqrt(s)` (`saturation` s), so every channel keeps its strength; tone: three bands (`shadows`, `midtones`, `highlights`) on display luma; seed from (`seed`, frame key `floor(time * rate + 0.5)`, pixel in layer space or screen space); zero mean, never fogs transparent pixels. Presets `fine_35mm`, `coarse_16mm`, `vintage_8mm` are our own names for sets of numbers (not AE's). **Linear-light rule:** the strength is in display numbers, so a linear-light film gets the same grain as a display film (tested). Omitted from AE: Viewing Mode, blend modes, Animate Smoothly, Tint (open question 3).
* **Halation** (`crates/core/src/effects/halation.rs`, 38 lines). `e = max(0, luma(premul) - threshold * a)` (Rec.709) in linear light, blurred by the existing `blurred(ctx.like, radius)` (no second blur), added as `strength * tint / luma(tint) * blurred`, alpha `a + (1 - a) * min(1, max light)`. `@space linear`; `grows()` gives 3 x radius on all sides (`GROWING` 14 names). **Energy rule, the number:** luma added = strength x the sum of the excess, test `the_luma_it_adds_is_strength_times_the_light_above_the_threshold_summed_over_the_picture` gives 4.000 for a 4.000 source at sigma 4.
* **Dither** (`crates/core/src/dither/`, `shaders/dither.sksl`, `color.dither` in the film root). `out = floor(v * 255 + (rank + 0.5) / 256) / 255` on the float picture after the output-profile transfer, with a tiled rank pattern: `ordered` is the 16 x 16 Bayer matrix, `blue_noise` a 64 x 64 void-and-cluster tile (sigma 1.5, torus, fixed seed, checksum pinned `1_680_497_426_194_267_903`). The ranks are an A8 image used as a Repeat/nearest child shader; alpha forced to 1. Mean threshold is exactly 0.5 (every rank appears equally); an error never exceeds +-1 level. Default `none` keeps the exact old `draw_image` path, so a film without the key is byte-identical. The pattern is static (the same on every frame), so it is not "temporal noise"; this is also why it cannot be hurt by frame order.

## Changed files (under `turbomotion/`; nothing committed)

New
- `crates/comp/src/fxlib/builtin/add_grain.sksl` (97 lines).
- `crates/core/src/effects/halation.rs` (38), `halation_tests.rs` (183), `shaders/effects/{halation_source,halation_add}.sksl`.
- `crates/core/src/dither/{mod.rs (73), patterns.rs (36), blue_noise.rs (129), tests.rs (172)}`, `shaders/dither.sksl`.
- `crates/comp/src/render/{grain_tests.rs (220), halation_tests.rs (82), dither_tests.rs (72), stats.rs (48)}`.
- `parity/specs/{add_grain,film_halation,dither}.md`, `parity/probes/{add_grain,film_halation}.json`.
- `wp/WP-33/REPORT-finish-stack.md`; `wp/WP-33/finish/` (README.md, five images, six tables, the four scripts, `films/` and `identity/` with `snap.sh`, `video.sh`, before/after TSVs and `identity-table.md`).

Modified
- `crates/core/src/effects/{mod.rs (265 lines; registry entry `film_halation`), programs.rs, bounds.rs}`; `crates/core/src/lib.rs` (`pub mod dither;`); `crates/core/src/color/{pipeline.rs, tests.rs}` (a `dither` field, `is_managed` compares only working and output spaces).
- `crates/comp/src/render/{mod.rs (268 lines), testkit.rs}`: a `dither: Option<Dithering>` field built in `Renderer::new`, one `match` in `draw_state`, test-module lines, and `render_frames` in the testkit. No other behaviour.
- `crates/comp/src/schema/color.rs` (`dither` in the colour block, refused outside the film root, one test), `crates/comp/src/catalogue/color.rs` (key and wording).
- `crates/script/sdk/{film-types.ts, README.md}`, `crates/script/tests/sdk_option_types.rs` (SDK typing, docs and a type test for `color.dither`).
- `parity/{effects.tsv, MATRIX.tsv}`, `docs/reference/{effects.md, parity.md}` (regenerated).
- Outer repo: `motions/lib/CAPABILITIES.md` regenerated (761 entries; the diff includes drift from other work, not only these two effects).
- No file in a forbidden path was touched; every Rust file is under 300 lines; `eval/mod.rs`, `plan/mod.rs`, `schema/mod.rs`, `fxlib/mod.rs` and `check/lint.rs` are not changed.

## Tests added

All pass; 33 new tests plus the type test and the schema test.

- `render::grain_tests` (12): the_standard_deviation_follows_intensity_as_the_spec_says, the_mean_stays_within_half_a_level_of_the_grey_it_is_added_to, size_sets_the_length_over_which_the_grain_agrees_with_itself, aspect_stretches_the_grain_sideways_and_softness_widens_it_without_weakening_it, saturation_mixes_a_shared_grain_with_one_per_channel_and_every_channel_keeps_its_strength, the_tone_response_scales_the_grain_by_the_band_the_brightness_is_in, a_preset_is_the_same_picture_as_its_values_written_out, the_same_film_renders_the_same_bytes_and_a_frame_or_a_seed_changes_the_grain, the_grain_is_anchored_to_the_layer_in_layer_space_and_to_the_frame_in_screen_space, both_effect_spaces_make_grain_of_the_same_statistics, a_linear_light_film_gets_the_same_grain_because_the_strength_is_in_display_numbers, grain_never_fogs_what_is_transparent_and_zero_intensity_changes_nothing.
- `effects::halation_tests` (6, GPU): the_luma_it_adds_is_strength_times_the_light_above_the_threshold_summed_over_the_picture, the_falloff_of_a_point_is_the_gaussian_the_radius_names, a_wide_halo_follows_the_closed_form_to_four_percent_of_its_peak, only_light_above_the_threshold_halates_and_a_covered_pixel_counts_by_its_coverage, the_halo_has_the_tints_hue_and_lights_a_transparent_picture_to_hold_it, no_strength_changes_nothing_and_no_radius_adds_the_excess_where_it_is.
- `render::halation_tests` (5): both_effect_spaces_draw_the_same_halo, the_halo_grows_with_the_layer_in_layer_space_and_keeps_its_size_in_screen_space, the_threshold_is_light_in_a_linear_light_film_and_encoded_numbers_otherwise, the_tail_has_the_tints_hue, the_same_film_renders_the_same_bytes.
- `dither::tests` (5): every_rank_appears_equally_so_the_mean_threshold_is_exactly_one_half, blue_noise_keeps_the_low_frequencies_out_where_white_noise_would_not, the_blue_noise_tile_is_the_same_on_every_build, no_dither_bands_a_dark_ramp_and_each_dither_removes_the_bands, a_flat_colour_keeps_its_level_except_for_the_half_float_it_is_held_in.
- `render::dither_tests` (4): leaving_dither_out_or_saying_none_is_the_old_rounding_byte_for_byte, a_dither_moves_no_pixel_more_than_a_level_and_breaks_the_bands_up, the_same_film_dithers_to_the_same_bytes_in_every_frame_and_every_render, a_colour_managed_film_dithers_after_the_output_conversion.
- `schema::color`: dither_is_the_roots_and_changes_no_colour_management; `effects::bounds`: a line for `film_halation` (3 x radius); `sdk_option_types`: `color.dither` typing; the catalogue `@example`s of `add_grain` and `film_halation` run through `examples_check` (7 tests) with no lint rule loosened, no `#[ignore]`, no allow-list.

## Acceptance numbers

Add Grain (65 536 px, `render/grain_tests.rs`; the bands on the probe image are looser, see `finish/add_grain-table.md`): std at mid-grey 6.2, 11.4, 18.1 levels for the three presets, within 1.0%, 1.2% and 1.5% of the spec; mean error under half a level; red-green correlation 0.85 / 0.70 / 0.55 as `1 - saturation` says.

Halation: energy 4.000 luma for a 4.000 source (sigma 4); point falloff against the closed form of the truncated, renormalised kernel; wide halo within 4% of its peak (Skia reduced-resolution blur); halo adds R : G : B = 14 : 5 : 2 (tint 255 : 90 : 40 is 1 : 0.353 : 0.157, measured 1 : 0.357 : 0.143); the grey disc under the threshold is unchanged (5, 6, 10).

Dither (a 0 to 16 level ramp, `finish/dither-table.md`):

| rounding | distinct levels | distinct 16-column means | max step between 16-column means | max slab-mean error vs ideal ramp (levels) |
|---|---|---|---|---|
| none | 17 | 33 | 0.500 | 0.300 |
| ordered | 17 | 80 | 0.227 | 0.017 |
| blue_noise | 17 | 80 | 0.208 | 0.007 |

Every pixel is within one level of the ideal ramp; the mean threshold is exactly 0.5 (zero-mean error).

Codec limits (`finish/codec-table.md`, the same ramp encoded): in software H.264 at crf 0 (lossless) the dither survives (15 luma values, 72 distinct 16-column means against 29 for none, max step 0.41 against 1.0); at crf 18 and with hardware `h264_videotoolbox` at 20 Mbit/s the compression removes it (29 to 30 means, step 1.0; blue noise is worse in hardware, 16 means and step 2.0). So dither helps ProRes and lossless output, not lossy H.264; 10-bit export is the right fix (listed under next steps).

## Byte identity, before and after (`finish/identity/identity-table.md`)

Baseline: `git archive HEAD` (2ab067e) built in `/tmp/fs-base`, rendered with the same scripts as the new binary, and deleted afterwards. 96 rows: the built comp JSON of 10 films, and stills at 0.5 s and 1.2 s of every example and built film; 10 video-stream md5s (frames 0 to 11 of five examples through the software and alpha codecs). Result: 96 of 96 stills/JSON identical, 10 of 10 video streams identical, 0 render failures. Re-run with the final binary after the last code change.

| set | rows | identical |
|---|---|---|
| built film JSON | 10 | 10 |
| stills (2 times each) | 86 | 86 |
| video-stream hashes | 10 | 10 |

Determinism: each of `add_grain` (5f6cbe1d3546adf6), `film_halation` (bca9a86992866b40), `dark-ramp-ordered` (9e5d65d3a294b779) and `dark-ramp-blue_noise` (a0c93efee0f98713) is byte-identical across two runs (sha256 of the 0.5 s still, first 16 hex).

## Gate output

```
cargo test --release --workspace --no-fail-fast   exit 0
    45 suites; 869 passed; 0 failed; 2 ignored (the doc-test ignores that were there before)
    turbomotion-comp lib: 544 passed;  turbomotion-core lib: 80 passed;  script: 8 lib + integration suites, all ok
turbomotion docs --check            docs are current (9 pages in docs/reference)
turbomotion parity check            parity check: ok (95 core rows, 266 effect rows, 100 catalogue effects)   exit 0
turbomotion parity report           ALL 266 rows: 3 done, 68 partial, 23 ..., T0+T1 effects done: 2 of 172 (1.2%); the programme target is 95%
turbomotion comp check <probe>      ok, 0 errors, 0 warnings for parity/probes/add_grain.json, film_halation.json and the four films in finish/films
scripts/capability_index.py         motions/lib/CAPABILITIES.md: 761 entries; find add_grain and find film_halation both return the effect with its parameters
python3 -m unittest scripts/test_capability_index.py   Ran 29 tests ... OK
```

## Deviations

1. `film_halation`, not `halation`: the platform already has a `halation`, and a plugin cannot reuse a platform effect's name. The legacy one stays for byte identity.
2. The dither key is `color.dither`, not `output.dither`, because the colour block is where the output profile lives and it keeps the dither after the transfer in one place.
3. The dither pattern is static; there is no temporal variation (a moving pattern would change every frame's bytes and cost the encoder).
4. Add Grain leaves out Viewing Mode, blend modes, Animate Smoothly and Tint. Its preset names are ours.
5. In an unmanaged display film the `film_halation` threshold is in the film's own numbers, not light; it is true light only in `linear_light` films (the same as every Linear effect).
6. `CAPABILITIES.md` regenerated with unrelated drift from other work. The repository is not rustfmt-clean, so I did not reformat.
7. A flat colour can move by one level for a small share of pixels (at most k/2048 + 1/256) because the picture is a half float; the test states it.

## Open questions

1. The Skia blur is not energy-exact above sigma 4 (it over- or under-counts small sources; the wide halo peaks about 3% to 4% low). Fixing it means a cascade or a versioned fix, which changes glow, bloom and shadow. Do you want that as its own work package?
2. Add the dither to the hardware and lossy path by moving to 10-bit export (ProRes and `yuv420p10le`), or keep it as a ProRes/lossless feature?
3. Should Add Grain get the AE Viewing Mode, blend modes and Tint, and should the presets be renamed to match AE's?
4. Should the legacy `grain`, `film_grain` and `halation` be put behind a versioned alias so a future film can drop them?

## Estimated cost

One long agent session (not timed). The full workspace test run takes about 4.5 minutes and an identity run about 20 seconds; the largest costs were the halation energy investigation (the blur limit) and the dither codec table.
