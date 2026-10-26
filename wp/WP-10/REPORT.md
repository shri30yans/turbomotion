WP: WP-10 Property model v2

Status: done (with the deviations and open questions below)

Changed files:
- Engine (`turbomotion/`):
  - `crates/comp/src/anim/prop.rs` deleted (531 lines); replaced by `crates/comp/src/anim/prop/{mod,value,key,shape,spatial,track,looping,parse,parse_keys,parse_side,vector,vector_parse,colour}.rs` (all under 210 lines) and the test modules `testkit,tests,tests_forms,tests_spatial,tests_temporal`
  - `crates/comp/src/anim/bezier.rs` (new: the one cubic-Bezier solver, Newton inside a bisection bracket), `anim/mod.rs`, `anim/ease.rs` (`Ease::Cubic` now calls it; the private copy was removed)
  - `crates/comp/src/eval/travel.rs` (new: `face`, `heading_of`, shared by `along.orient` and `auto_orient`), `eval/mod.rs` (the five per-axis loops became `VecProp::at_with`; 749 -> 737 lines)
  - `crates/comp/src/plan/facing.rs` (new: the one conflict check for `aim`, `along.orient`, `auto_orient`), `plan/layer.rs`, `plan/mod.rs` (+1 line: `mod facing;`)
  - `crates/comp/src/schema/wrap.rs` (position, scale and triples read through `VecProp`)
  - `crates/comp/src/catalogue.rs`, `catalogue/motion_keys.rs`, `catalogue/property_forms.rs` (new): the `keyframe_motion` and `property_forms` entries
  - `crates/cli/src/docs.rs`, `crates/cli/src/docs/keyframe_motion.rs` (new), `crates/cli/tests/parity.rs` (the gap-list test named "Spatial bezier paths" as a gap; it now names "Expressions: full object model", which is still `none`)
  - SDK: `crates/script/sdk/{motion-types,motion-path}.ts` (new), `timeline.ts`, `tween-types.ts`, `compile.ts`, `types.ts` (+2 lines), `index.ts`; `crates/script/tests/sdk_motion_path.rs` (new)
  - `docs/reference/keyframe-motion.md` (new, generated), `docs/reference/README.md`, `parity.md`, `sdk.md` (generated)
  - `parity/MATRIX.tsv`, `parity/specs/property_model.md` (new)
  - `wp/WP-10/{REPORT.md, probes/*, identity/*}` (new)
- Outside `turbomotion/`: `scripts/capability_index.py` (new `motion_keys` rows), `motions/lib/ae_terms.json` (8 terms added), `motions/lib/CAPABILITIES.md` (regenerated; it also carries the other agents' in-flight `ae_terms.json`/`layer_keys.json` edits, which I did not touch). `scripts/capability_sdk.py` untouched.

Tests added (44 Rust; all pass):
- `anim/prop/tests_temporal.rs` (14): `a_third_of_the_way_at_the_segments_own_speed_is_a_straight_line`, `a_linear_side_is_the_segments_speed_with_a_third_influence`, `easy_ease_is_the_smoothstep_of_its_two_handles`, `full_and_zero_influence_ease_matches_its_closed_form`, `both_influences_at_a_hundred_stall_at_the_middle_and_still_solve`, `the_time_mapping_is_monotone_and_exact_for_every_pair_of_influences`, `a_fast_out_speed_with_a_slow_in_speed_overshoots_and_comes_back`, `the_speed_at_a_handle_is_the_slope_of_the_curve_there`, `a_hold_key_steps_exactly_at_the_next_key`, `an_auto_bezier_key_takes_the_slope_between_its_neighbours_with_a_sixth_influence`, `a_continuous_key_shares_one_speed_between_its_sides`, `a_color_or_vector_with_a_hold_key_holds_every_component`, `the_cases_of_the_after_effects_fidelity_run_match_the_reference_solver`, `influence_33_with_speed_0_is_the_css_cubic_bezier_of_the_same_numbers`
- `anim/prop/tests_spatial.rs` (12): `the_curve_passes_through_its_keys_exactly`, `auto_bezier_handles_are_a_sixth_of_the_way_between_the_neighbours`, `the_direction_is_continuous_through_an_auto_key`, `a_linear_ease_travels_the_curve_at_a_constant_speed`, `roving_keys_keep_one_speed_across_the_whole_path`, `a_roving_point_is_passed_exactly_when_the_arc_length_reaches_it`, `a_path_runs_on_along_its_last_tangent_when_the_timing_overshoots`, `a_path_hold_stays_at_its_key_until_the_next`, `explicit_handles_make_the_curve_the_bezier_they_describe`, `halfway_round_a_quarter_circle_the_path_points_at_45_degrees`, `a_position_in_3d_follows_a_curve_through_depth`, `the_heading_of_a_straight_run_is_its_direction`
- `anim/prop/tests_forms.rs` (8): `every_old_key_list_gives_the_same_bits_as_before` (8 eases x 4 loops, `to_bits` equality), `old_vector_forms_still_parse_to_one_property_per_axis`, `a_vector_that_is_not_a_position_interpolates_each_component`, `a_colour_with_handles_interpolates_each_channel_as_written`, `a_path_survives_the_composition_schema`, `bad_handles_are_refused_with_what_to_change`, `handles_that_belong_to_a_path_are_refused_where_there_is_none`, `a_switch_and_text_change_only_at_a_key`
- `eval/travel.rs` (3, whole evaluator): `a_flat_layer_turns_to_the_direction_of_its_path_about_z`, `a_3d_layer_points_its_depth_axis_along_the_path`, `one_thing_decides_where_a_layer_faces`
- `catalogue/motion_keys.rs` (2): `every_form_loads_and_evaluates`, `the_key_fields_are_the_ones_a_key_object_accepts`. The 9 catalogue examples and the new docs page also run in the existing `examples_check` harness, unchanged and not loosened.
- `script/tests/sdk_motion_path.rs` (5): `a_motion_path_round_trips_from_the_sdk_through_json_into_the_engine`, `a_path_tween_passes_its_points_at_an_even_speed_and_ends_on_time`, `a_path_tween_with_auto_orient_banks_the_layer_along_its_curve`, `handles_on_a_tween_are_speed_and_influence_with_easy_ease_as_the_smoothstep`, `what_the_engine_would_refuse_is_refused_where_it_was_written`
- The 7 older tests of `prop.rs` moved to `prop/tests.rs` unchanged.

What was built:
- One evaluator: `Track::sample` serves a scalar `Prop`, `VecProp::Joined` and `ColorProp::Joined`. Legacy key lists keep the legacy formula bit for bit (`a + (b-a)*ease.at(x) + extra`).
- Temporal: AE's cubic of value against time (P0 = (0, a), P1 = (i_out, a + s_out*i_out*dur), P2 = (1 - i_in, b - s_in*i_in*dur), P3 = (1, b)); `linear`, `hold`, `auto`, `easy`, `{speed, influence}` and `continuous`. Spatial: a cubic per segment with an arc-length table, `auto`/`linear`/`bezier`/`continuous` keys, roving keys, `auto_orient: "path"`, `separate_dimensions`. Typed values: scalar, vec2, vec3, colour, bool, text. Strict (`deny_unknown_fields`, did-you-mean) with errors that say what to change.
- Definitions, limits and numbers: `parity/specs/property_model.md`. Reference page: `docs/reference/keyframe-motion.md`. JSON and SDK (`motionPath`, `tl.to(..., {handles})`, `tl.to(..., {path})`) author the same thing.

Acceptance numbers (every one asserted by a test above):
- Influence 33.333 % on both sides is linear: within 1e-9. Easy ease (speed 0, 33.33 %) equals the smoothstep: within 1e-9. Influence 33 / speed 0 equals `cubic(0.33,0,0.67,1)`: within 1e-12 on the numbers, 0.0000 px over 41 samples in the probe.
- The time solve returns x(u) = x within 1e-12 for all 11 x 11 influence pairs (0 to 100 %); the time mapping is monotone; 100 % / 100 % stalls at the middle and still solves. The 8 cases of the EffectCraft fidelity run match a bisection reference within 1e-7 (1e-3 for the triple root at 100 % / 100 %). Overshoot: a 900 deg/s leaving speed passes the end value and comes back.
- Spatial: passes through its keys exactly; speed steady along an S curve within 1e-3, and across the whole run with roving keys within 1e-3; a roving point is passed when the arc length reaches it; direction continuous through an Auto key (dot > 1 - 1e-9); Auto Bezier handle = a sixth of the way between neighbours. Quarter circle: arc length within 3e-4 of pi/2 * r, equals 200k chords within 1e-6; at the arc-length midpoint it is at 45 degrees within 0.5 px (probe: 45.000 degrees, 816.78, 536.78) and the heading is 135 +- 0.5 degrees.
- Hold steps exactly at the key time (probe: x 140.0 at 0.9 s, 440.0 at 1.0 s). Vectors and colours interpolate per component. The path round-trips through SDK -> JSON -> engine and the composition schema.
- Done-when: a circular path with 4 keys (+ the closing key) renders a smooth circle with a banked object: radius error 0.067 px at most over 41 samples, speed 392.3 to 392.4 px/s, arrow tangent to the circle (`probes/circle.png`, `probes/arrow-1.5s.png`).

Byte identity of everything that existed (baseline binary built from `git archive HEAD` in `/tmp/wp10-base`, since deleted; table = first 16 hex digits of the sha256 of the PNG at t = 0.5 and 1.2 s, rows in `identity/before.tsv` and `identity/after.tsv`; `diff` of the two files is empty, 96 rows each, 0 render failures):

| group | rows (films x 2 times) | before = after |
| --- | --- | --- |
| `examples/comp/*` (17 comps incl. showcase) | 34 | identical |
| `examples/sdk/launch` and 11 `components/*` | 24 | identical |
| WP-05 probes (`old-modes`, `old-modes-light`, `display`, `light`) | 8 | identical |
| 10 built films, SDK-built JSON file hash | 10 | identical |
| the same 10 built films, stills | 20 | identical |

Per-film hashes (stills at 0.5 and 1.2 s share the row prefix; the first is the film at t = 0.5): `01-shapes e6d87e9ca3f4b437`, `02-type 878232c58ce16bf8`, `03-media d5c9a9dc476ea6a4`, `04-plugins 5f8fefd0cf5a9cf5`, `06-3d c079fdbc5e42de1d`, `showcase c39b44b7a0c92789`, `sdk launch 0b8abe740eb05068`, built `launch-film c490159d6d813f00` (JSON `ba57aa8de070016a`), built `figma-anthem 10279e2746f10688` (JSON `b3682baf6fe37b92`), built `flow-state c5fa1465d524d88e` (JSON `89438f6aa91f3b2e`), built `001-esports-momentum df0f21a386c7a434` (JSON `7b1b0585fa768eba`); the rest are in the two tsv files. Video streams (frames 0..12, one worker, md5 of the video stream, `identity/video-before.tsv` = `video-after.tsv`): `01-shapes` software `4a3b0253712f3ea9` / alpha `20c5f315ddfb91f1`; `02-type` `c861873f3d6aea2c` / `3d9c588da461cacc`; `03-media` `335e344cb50bee45` / `431cbcd47342649e`; `04-plugins` `1c2f5826ae888ecd` / `196ff4e24fe8bf50`; `09-look` `5c4d1aeedf048445` / `e845c309b93e4c57`: all identical.

Gate output:
1. `cargo test --release --workspace --no-fail-fast` -> exit 0, 41 test result lines, `passed 649 failed 0 ignored 2` (WP-05 recorded 590 passed; this WP adds 44; I did not re-measure the count at the start of this WP). Ignored, both pre-existing: `pose_matrices_match_the_after_effects_dump` (needs an AE dump) and `actual_typescript_compiler_checks_types_without_executing_the_film` (needs `TURBOMOTION_TEST_TSC`). comp lib tail: `test result: ok. 390 passed; 0 failed; 0 ignored`. A first full run failed one test, `the_gap_list_names_what_is_not_built_and_what_is_decided_against`, which asserted that "Spatial bezier paths" is a gap; it is done now, so the test names another open row (see Changed files).
2. `docs --write` -> `wrote 9 pages to docs/reference`; `git status` shows `docs/reference/{README,parity,sdk}.md` modified and `keyframe-motion.md` new; `docs --check` -> `docs are current (9 pages in docs/reference)`.
3. `python3 scripts/capability_index.py` -> `motions/lib/CAPABILITIES.md: 732 entries`; `find "roving keyframes"` -> `ae term 'roving keyframes': a key {roving: true, v} with no time ...` then `motion-form | roving | Two roving points between fixed keys ...`. `python3 -m unittest scripts/test_capability_index.py` -> `Ran 27 tests ... OK`.
4. `turbomotion parity check` -> `parity check: ok (83 core rows, 264 effect rows, 97 catalogue effects)`, exit 0. Rows: Spatial bezier paths, Roving keyframes, Auto-orient along path, Separate dimensions, Temporal and spatial ease split, Speed and influence handles, Hold keyframes, Auto Bezier spatial tangents: `done` (each with a test and `parity/specs/property_model.md`). `Temporal Auto Bezier and Continuous Bezier` and `Typed property values` are `part`; `Colour keys interpolated in linear light` is `none`.
5. `comp check` on both probe films: `ok: 1280x720 at 30 fps, 4.00s (120 frames), 3 layers ...; 0 errors, 0 warnings` (`circle`) and `... 4 layers ...; 0 errors, 0 warnings` (`motion`).
6. Determinism: two `comp render --codec software` runs of each probe: `circle` `4357f9429d38fd05` both times, `motion` `1c4daa4c323ed725` both times (sha256 of the mp4, first 16 digits).
7. Probe images in `wp/WP-10/probes/`: `circle.png` shows eight moments of an arrow riding a 4-key circle tangent to the grey ideal circle while a red dot on Auto Bezier keys drifts inside it (a rounded square); `arrow-1.5s.png` is a 3x crop showing the arrow's tip pointing along the tangent; `motion.png` shows the green S-path dot at one pace through its roving keys, the yellow handle dot and the blue cubic dot together all the way, and the red square stepping at 1, 2 and 3 s. Data: `data.tsv` (positions every 0.1 s), `speeds.tsv`, `numbers.txt`; one-line descriptions with numbers: `probes/README.md`.
8. Housekeeping: `/tmp/wp10-*` removed; no worktree or branch made; nothing committed; the files under `archive/`, `tmp/`, `motions/leeson-barings/`, `motions/lib/brush-stroke/`, `motions/lib/wet-reflection/` and `.rtk/` untouched. New files all under 300 lines (largest `tests_temporal.rs` 208). `eval/mod.rs` shrank (749 -> 737 lines); `anim/prop.rs` (531) is split. `plan/mod.rs` (+1 line) and `sdk/types.ts` (+2 lines) are over 300 lines already and grew by those lines (a `mod` line and one option's type).

Deviations from this plan:
- No `Path` value type: a path is the position property plus per-key tangents; `Interp` is realised as `Side` and `Shape`. The file is `looping.rs` (not `loop.rs`, a keyword-adjacent name).
- Colour interpolates the literal channels as written (0 to 255 and alpha), not in linear light: the colour pipeline has no CPU sRGB curve, so the "film's working space" rule of `parity/specs/colour_pipeline.md` is not met for colour keys (row `Colour keys interpolated in linear light` is `none`).
- The Auto Bezier temporal rule for the first and last keys (speed 0, influence 1/6) is the plan's, not measured in AE. EffectCraft's `keyframe-ease.json` publishes error summaries only, so the 8 fidelity cases are checked against a bisection reference, not against AE numbers.
- Rotation and Orientation stay per-axis (no joined track), so an AE-style joined 3D rotation path is not built. Switch and text exist in the track API only, with no JSON form yet (row is `part`).
- Joined-vs-legacy rule: a vector with key objects (or `{"keys": ..}`, `separate_dimensions`) is one joined property; an array whose items are plain key lists stays one property per axis. An array of key objects needs the `{"keys": [...]}` wrapper.
- Equal key times are allowed (a step); times may not go back.
- `tl.to` takes `handles` and `path` (roving intermediate points); one path tween per layer.
- Expressions are not available on a joined whole-vector track (use `separate_dimensions`).
- `plan/mod.rs` and `sdk/types.ts` grew by 1 and 2 lines (above); the first `parity.rs` assertion was changed (above).
- `scripts/capability_index.py` gained `motion_keys` rows (kinds `motion-form`, `key-field`, `property-field`) so the `ae_terms.json` names resolve; the ae terms carry no `feature` because the triage vocabulary has none for motion paths.

Open questions for the owner:
1. Colour space of colour keys: keep the literal channels (matches today's colours), or interpolate in linear light when the film is in linear light (needs a CPU sRGB curve in `core`, a follow-up)?
2. Is AE's Auto Bezier end-key rule (speed 0, influence 1/6) what you measure in AE, or is it 33.33 % like Easy Ease? One AE dump would settle it.
3. Do you want a JSON form for switch and text keys (the track takes them; no layer property reads one yet) in a later WP?
4. Should an array of key objects without the `{"keys"}` wrapper be read as a joined vector? It is ambiguous with per-axis lists, so it is refused for now.

Estimated cost to finish (if partial): none; the items above are follow-ups, about half a day each for 1 and 3.
