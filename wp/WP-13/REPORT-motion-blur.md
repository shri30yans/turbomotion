WP: WP-13 step 3, motion blur (finish-stack item 1). Only this slice; posterize_time, time_displacement, frame_blend, pixel motion blur / flow, timewarp and reverse are not started.

Status: done for this slice. Nothing committed or pushed.

# Audit of the motion blur that was there (HEAD 00525b2)

What it was: the composition key `"mblur": {"samples": 8, "shutter": 180}` and the layer switch `"mblur": true`. The root composition was evaluated at `samples` moments `t + ((i + 0.5)/n - 0.5) * shutter/360 * (1/fps)`, a layer with the switch was drawn from each moment (effects and masks included) and the pictures were summed with Skia `Plus` at `opacity / n` (`Stage::ShutterAverage`).

What was right (and is kept, byte for byte): one evaluator for everything that moves (transforms, paths, keys with their handles, camera, 3D, text animators, particles); premultiplied pictures summed and divided by the total `n`, so alpha coverage is exact; the surfaces are colour managed (WP-03), so with `color.linear_light` it averages light.

What it did not do against After Effects:
- no shutter phase: always centred on the frame (AE default -90 for a 180 degree shutter matches only in that one case);
- no validation: `samples < 2` or `shutter <= 0` silently drew no blur, nothing capped `samples` (1000 was accepted), negative or absurd angles passed;
- a layer not showing at the frame time took no part (a layer entering during the shutter did not blur in; a layer present only in the last samples was dropped);
- no still detection: a layer that does not move was drawn `n` times and averaged (pictures equal to the last bit are not always the average);
- no lint (a shutter with no switched layer, a shut shutter, a switch in a nested comp), no cost warning, no docs or catalogue entry, not typed in the SDK for the composition (only on timelines);
- nothing said what a switch inside a precomp does.

Decision: the existing maths is right for a centred shutter, so it stays as the legacy key, unchanged; the correct AE behaviour is the new opt-in comp block `motion_blur`. The two cannot be used together (an error). Why a new key rather than changing the old: its defaults (8 samples), its visible-at-`t` rule and its lack of still detection are all visible in existing renders (`figma-anthem`, `flow-state`, `micro-courage-reel`, `media-effects-reel` use it), and the owner's rule is that no existing render changes silently. Both keys resolve to one `Shutter { shutter_angle, shutter_phase, samples }` and one sampling function, so there is one mechanism: legacy is the phase `-angle/2` case with its old quirks (visible at `t` only, no still shortcut, no range check).

# What was built

- `schema/mblur.rs`: `Shutter` (strict, did-you-mean from the existing machinery, ranges: angle 0 to 720, phase -360 to 360, samples 2 to 16; defaults 180, -90, 16), `Blur::{Legacy, AfterEffects}`, `blur_of`. Moment `i` of `n`: `t + ((i + 0.5)/n + phase/angle) * angle/360 * (1/fps)`. For the legacy phase `-angle/2` this is exactly the old expression, which is why the old films are identical.
- `render/graph/shutter.rs`: `frames` (the moments, evaluated by `Evaluator::frame`) and `Builder::composite_blurred` (the average of a switched layer; replaces the old inline branch). New `check/motion_blur.rs` (4 rules), `catalogue/motion_blur.rs`, SDK `MotionBlurOpts`, `parity/specs/motion_blur.md`.
- Still detection (`motion_blur` only): `eval/same.rs` compares the evaluated state of a layer at the frame time and at every moment, with the camera and lights, apart from its own clock (`local_t`); the comparison is an exhaustive destructure, so adding a field to `LayerState` without deciding about it does not compile. A layer with effects, a matte, a clip, a blend mode that reads the frame, texture or pack content, or a precomp that contains any of these, is never still (platform effects read the clock when they draw). A still layer is drawn once, as without blur.
- Cost: a comp with the block and no switched layer evaluates no extra moments (a test counts them); a frame costs about `samples` times for a switched layer that moves.
- Adaptive sampling is not implemented: the number of samples is fixed and capped at 16 (`comp check` and the schema refuse more, the older key is warned about at more than 16). The midpoint rule gives the exact mean, but the outer extent of a smear is one sample step short and a smear much longer than the thing that moves shows a ripple of one sample (see the numbers).
- Nested rule (a deviation from AE, documented in the spec, the catalogue and by a lint): only layers of the root composition have a switch. A precomp layer with the switch blurs everything it shows; a switch inside a nested comp does nothing.
- Colour space: the average is made in the film's working space; premultiplied pictures are summed, so alpha is exact.

# Changed files

Engine (turbomotion/crates):
- new: `comp/src/schema/material.rs` (moved out of `schema/mod.rs`), `comp/src/render/graph/shutter.rs`, `comp/src/eval/same.rs`, `comp/src/eval/shared.rs`, `comp/src/eval/shading.rs` (moved out of `eval/mod.rs`), `comp/src/check/motion_blur.rs`, `comp/src/check/effect_inputs.rs` (moved out of `check/lint.rs`), `comp/src/catalogue/motion_blur.rs`, `comp/src/render/shutter_tests.rs`, `comp/src/render/shutter_world_tests.rs`, `comp/tests/motion_blur_schema.rs`.
- modified: `comp/src/schema/{mblur.rs,mod.rs}`, `comp/src/plan/mod.rs`, `comp/src/render/mod.rs`, `comp/src/render/graph/{mod.rs,build.rs,describe.rs}`, `comp/src/eval/{mod.rs,fx.rs,media.rs,mesh.rs,palette.rs,particles.rs,shape.rs,shape/repeat.rs,text.rs}` (`PartialEq` on the evaluated state; `Rc` fields behind `Shared`), `comp/src/shape/stroke/taper.rs` (`PartialEq`), `comp/src/check/{mod.rs,lint.rs}`, `comp/src/catalogue.rs`, `comp/README.md`, `cli/src/docs.rs`.
- SDK: `script/sdk/{film-types.ts,types.ts,README.md}`, `script/tests/{sdk.rs,sdk_option_types.rs}`.
- Docs and ledger: `docs/reference/{layers.md,sdk.md,parity.md}` (generated), `parity/MATRIX.tsv`, `parity/specs/motion_blur.md`.
- Evidence: `wp/WP-13/identity/*`, `wp/WP-13/probes/*`, this report.

Outside `turbomotion/` (repo root `YoutubeVideoMaker`): `motions/lib/ae_terms.json` (the "motion blur" term now names `motion_blur` and `mblur`, and has the AE spellings as `aka`), `motions/lib/layer_keys.json` (`mblur`), `motions/lib/CAPABILITIES.md` (generated; its diff is larger than my change because the file was already stale against the engine), `scripts/capability_index.py` (rows for `motion_blur` and its three options), `scripts/test_capability_index.py` (one test).

File sizes: every new file is under 300 lines (largest 199). The oversized files did not grow: `eval/mod.rs` 735 to 696 (`culled`, `lit_state` moved to `eval/shading.rs`), `schema/mod.rs` 493 to 482 (`Material` moved to `schema/material.rs`), `check/lint.rs` 358 to 336 (the effect-input rule moved to `check/effect_inputs.rs`), `plan/mod.rs` 433 to 433, `render/mod.rs` 262 to 252, `render/graph/build.rs` 282 to 266. `fxlib/mod.rs` untouched. No `unwrap()` or `expect()` outside tests in the new code.

# Tests added (38 Rust, 1 Python)

- `render/shutter_tests.rs` (8): `a_100_px_per_frame_square_with_a_180_degree_shutter_smears_over_70_px`, `a_360_degree_shutter_smears_over_120_px_and_a_shut_shutter_or_a_switch_that_is_off_leaves_the_sharp_square`, `the_smear_has_a_flat_top_and_straight_ramps_when_the_samples_are_dense_enough`, `a_one_pixel_per_frame_mover_shows_no_stepping_with_16_samples`, `the_phase_slides_the_window_by_its_own_degrees`, `the_first_frame_blurs_over_moments_before_time_zero_and_a_layer_is_only_there_from_its_start`, `a_layer_that_enters_during_the_shutter_is_drawn_for_the_moments_it_is_there`, `a_block_with_no_switched_layer_evaluates_no_extra_frames`.
- `render/shutter_world_tests.rs` (12): ease (true sub-frame positions), speed/influence handles, curved path, rotating bar arc, linear light 188 against 128, alpha, switch off and the nested rule, still layer and graph pass counts, effects per sample, older key equals new key where they agree, determinism, the oracle agrees with a sharp render.
- `tests/motion_blur_schema.rs` (6): defaults, legacy conversion, did-you-mean, ranges, both keys refused, a timeline that switches its shots takes the block's shutter (and the object form of `timeline.mblur` is the older key).
- `schema/mblur.rs` (5), `check/motion_blur.rs` (3), `eval/shared.rs` (1), `catalogue/motion_blur.rs` (1: the example loads and the keys named are the ones parsed; the examples harness also runs it).
- SDK: `comp_motion_blur_reaches_the_json_and_is_left_out_by_default` (`tests/sdk.rs`), `the_motion_blur_option_is_typed_with_exactly_the_engines_keys` (`tests/sdk_option_types.rs`).
- Python: `test_the_motion_blur_key_of_the_composition_its_options_and_the_layer_switch_are_found_by_name_and_by_what_they_do`.

# Acceptance numbers (`wp/WP-13/probes/data.tsv`, all 31 rows `ok`; the Rust tests assert the same)

| quantity | expected | measured | limit |
| --- | --- | --- | --- |
| 100 px/frame, 20 px square, shutter 180: smear length (variance) | 70 | 69.92 | 1.5 |
| same, outer extent (one 16-sample step, 3.1 px, short) | 66.9 | 68.0 | 1.5 |
| shutter 360: smear length (variance) | 120 | 119.82 | 1.5 |
| shutter 360: outer extent | 113.8 | 114.0 | 1.5 |
| shutter 0, or switch off: extent | 20 | 20, identical to unswitched | 0 |
| 180: flat top level (255 * 20 / 50) | 102.0 | 102.4 | 2 |
| 360: flat top level | 51.0 | 51.3 | 2 |
| flat top ripple (one of 16 samples) | 15.9 | 16.0 | 1 |
| centre, 180 / 360 at phase -90 | 600 / 625 | 600.0 / 625.0 | 0.3 |
| phase -180 / -90 / 0 / 90: centroid | 575 / 600 / 625 / 650 | same | 0.3 |
| `power2.in` ease: centroid | 610.475 | 610.482 | 0.3 |
| speed/influence 60 % handles: centroid | 208.256 | 208.259 | 0.3 |
| rotating bar: worst tip radius error over the arc | 0 | 0.29 | 1.5 |
| rotating bar: swept angle, degrees | 45 | 45.6 | 3 |
| linear light: edge pixel | 187.5 | 188 | 1 |
| display numbers: edge pixel | 127.5 | 128 | 1 |
| alpha: red / blue / alpha of the edge pixel over blue | 127.5 / 127.5 / 255 | 128 / 127 / 255 | 1 / 1 / 0 |
| static scene against no block: largest pixel difference | 0 | 0 | 0 |

The 1 px/frame mover within 2 levels of the closed form is asserted in `a_one_pixel_per_frame_mover_shows_no_stepping_with_16_samples`; the threshold for "smear 70 px" is 1.5 px on the variance-based length (`sqrt(12 var - size^2) + size`), which is the length of the ideal box smear; the raw outer extent of 16 samples is one step shorter by construction and is stated against 70 minus a step. Schema: `motion_blur.samples: 32` is refused with the range; the older key at 32 samples warns `motion-blur-cost`; a mistyped key gets "did you mean". Determinism: `smear-180` rendered twice with the software codec gives `4a313c0a0a4db6e6` both times.

# Byte identity of existing films (full tables in `wp/WP-13/identity/TABLE.md`)

Baseline: a binary built from `git archive HEAD` (00525b2) extracted into an empty `/tmp/mb-base` (no worktree, read-only use; deleted afterwards), against this tree. Scripts: `wp/WP-13/identity/{snap.sh,video.sh}` (copied from WP-10) and `blurfilms.sh`.

| set | rows | before = after |
| --- | --- | --- |
| stills at 0.5 s and 1.2 s of all 43 example and built-film comps, plus the sha256 of the 10 built comp JSONs | 96 | 96 of 96 |
| video-stream hashes, 5 examples x software and alpha codec, frames 0 to 12 | 10 | 10 of 10 |
| four films that use the older `mblur` key, frames 0 to 40 | 4 | 4 of 4 |

The ten video streams (`01-shapes` software 4a3b0253712f3ea9, alpha 20c5f315ddfb91f1; `02-type` c861873f3d6aea2c, 3d9c588da461cacc; `03-media` 335e344cb50bee45, 431cbcd47342649e; `04-plugins` 1c2f5826ae888ecd, 196ff4e24fe8bf50; `09-look` 5c4d1aeedf048445, e845c309b93e4c57) are identical before and after, and the blurred films are too (`figma-anthem` 007420a5cf416ce4, `flow-state` 6bdcaf0a10d8f50b, `micro-courage-reel` 90f6fec276544078, `media-effects-reel` f09dbda7d326359b). The tables were produced after the last refactor.

# Render time

- Existing films (older key): `figma-anthem` 0.42 s before, 0.54 s after (noise: the same run gave 0.41 s earlier); `flow-state` 1.32 and 1.24 s; `micro-courage-reel` 1.24 and 1.24 s; `media-effects-reel` 1.07 and 1.00 s (41 frames each, one worker). No change within noise: the legacy path is the old code with the moments moved.
- A film with no `motion_blur` block or no switched layer: same video stream and the same time (`block-no-switch` rows in the probes README: 0.56, 0.52 and 1.60 s against 0.60, 0.53 and 1.52 s).
- With the switch on every layer at 16 samples (60 frames, one worker): `01-shapes` (1280x720, 9 layers) 0.60 s to 1.40 s; `04-plugins` 0.53 s to 1.02 s; `09-look` (1920x1080, 16 layers) 1.52 s to 8.61 s. Layers that do not move cost one draw.
- Memory: one picture per sample is live per blurred layer while it is averaged (up to 16; `comp graph` prints the peak).

# Gate output

```
$ cargo test --release --workspace --no-fail-fast        (exit 0)
test result: ok. 522 passed; 0 failed; 0 ignored   (turbomotion-comp, lib; the largest suite)
... all suites: 836 passed, 0 failed, 2 ignored
```
The 2 ignored tests were already there and are not mine: `pose_matrices_match_the_after_effects_dump` (needs a golden dumped from After Effects) and `actual_typescript_compiler_checks_types_without_executing_the_film` (needs `TURBOMOTION_TEST_TSC`; no `tsc` on this machine, so the new SDK types were checked by the SDK tests, not by a real compiler). The examples harness passed (the catalogue example is a composition in `catalogue::all()`).

```
$ cargo run --release -p turbomotion -- docs --write
wrote 9 pages to docs/reference
$ cargo run --release -p turbomotion -- docs --check
docs are current (9 pages in docs/reference)
$ git diff --stat docs/
 docs/reference/layers.md | 81 +++++++
 docs/reference/parity.md |  2 +-
 docs/reference/sdk.md    |  5 +
$ python3 scripts/capability_index.py
motions/lib/CAPABILITIES.md: 757 entries
$ python3 scripts/capability_index.py find motion_blur     (comp-key motion_blur, motion_blur.samples, .shutter_angle, .shutter_phase, layer-key mblur, ae term 'motion blur')
$ python3 -m unittest discover -s scripts -p 'test_*.py'
Ran 49 tests in 2.325s   OK
$ turbomotion parity check
parity check: ok (92 core rows, 264 effect rows, 98 catalogue effects)       exit 0
$ python3 wp/WP-13/probes/make.py
31 rows, all ok; every probe film: 0 errors, 0 warnings (smear-0 warns motion-blur-shut on purpose; samples-32 is refused; older-key-32 warns motion-blur-cost)
```

Ledger: the old row "Motion blur per layer" (part) is split into three: "Motion blur per layer: switch, shutter angle, shutter phase and samples (opt-in `motion_blur` block)" is `done` (test `a_100_px_per_frame_square_with_a_180_degree_shutter_smears_over_70_px`, doc `parity/specs/motion_blur.md`); "Adaptive Sample Limit, and a switch inside a precomp that blurs its own layers" is `none` (WP-13); the older key is `have`. The two other WP-13 time rows (frame blending / pixel motion blur / timewarp; posterize / time displacement / difference) are unchanged: `none`.

# Deviations from this plan

- Opt-in key `motion_blur` (not a `mode` switch on `mblur`), the older key kept as legacy: reason in the audit.
- Default `samples` is 16 (the cap), not 8 as the older key has, so that AE's default look is reached without a setting; the cost is stated by the catalogue, the lint and the schema message.
- Adaptive sampling is not implemented.
- Nested switches are not honoured (see above); a lint says so.
- The outer extent of a 16-sample smear is one sample step short of the ideal length; the acceptance length is measured by variance (`69.9` for `70`) and the extent is reported against `70 - step`.
- `phase` is in degrees of the frame interval as in AE (-360 to 360). A 360 degree shutter at the default phase is centred a quarter frame late; this is AE's behaviour and is documented.
- Edits to `PartialEq` derives across `eval/` and `Shared<T>` (pointer equality for the layout and mesh `Rc`s) were needed for still detection; no behaviour change.
- Three small modules were split out of oversized files so none of them grew (`shading.rs`, `material.rs`, `effect_inputs.rs`); pure moves.
- Outside the engine: `motions/lib/ae_terms.json`, `layer_keys.json`, `CAPABILITIES.md`, `scripts/capability_index.py` and its test were edited (not `capability_sdk.py`).

# Open questions for the owner

1. Honour the inner switches of a precomp (AE blurs a precomp's layers only when both switches are on)? That needs one shutter per composition, and a per-level average, which multiplies the cost (n x n); I left it out.
2. Adaptive sampling: worth a fixed-budget version (more samples for faster layers, up to a total)? It would change the one-mechanism design slightly (per-layer moments).
3. Should `samples` above 16 be allowed behind an explicit flag for stills of very fast motion? Today it is refused.
4. Timeline films: `timeline.mblur: true` switches every shot and combines with the comp's `motion_blur` (checked by hand on the built `flow-state` comp: `comp check` 0 errors, 0 warnings, a still renders); only the object form (`timeline.mblur: {samples, shutter}`) is the older key, and with it `motion_blur` is refused as both keys. A test holds both cases (`a_timeline_that_switches_its_shots_takes_the_shutter_of_the_block_...`).
5. `CAPABILITIES.md` had changes that did not come from me (it was stale); I regenerated it. The repo root also has staged deletions (`motions/lib/MIGRATION_NOTES.md`, a migration image) and edits to `scripts/capability_terms.py` that are not mine and were left alone.

# Estimated cost to finish

Nothing is left in this slice. Optional follow-ups: inner precomp switches 1 to 2 days; adaptive sampling about 1 day.
