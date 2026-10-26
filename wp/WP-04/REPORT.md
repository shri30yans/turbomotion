WP: WP-04 Docs and probes as tests
Status: done
Changed files:
Engine (`turbomotion/`):
- NEW `crates/comp/tests/examples/plugins.rs`; `crates/comp/tests/examples/mod.rs`; `crates/comp/tests/examples_check.rs` (step 1: every plugin in `motions/lib/effects/` is tested through its `@example`, or its defaults when it has none)
- NEW `crates/comp/src/schema/layer_keys.rs`; `crates/comp/src/schema/mod.rs`; `crates/comp/src/api.rs` (`layer_key_problems`): the engine's own list of keys a layer takes, reachable from the SDK
- NEW `crates/script/src/vocabulary.rs`; `crates/script/src/run.rs`; `crates/script/src/lib.rs`: host function `__tm_layer_problems`
- `crates/script/sdk/{arg-check,layers,comp,timeline,three,assets,text-fx,types,index}.ts`; NEW `crates/script/sdk/{raw-types,matte-check}.ts`; `crates/script/sdk/README.md` (step 3: unknown-field throw, `layer.effects =` refusal, `texture()`, typed `raw`, text `italic`/`outline_only`, hidden matte source on an adjustment layer; documented `TURBOMOTION_BIN`, `fx`, `solid(color, opts)`, `texture(shader, opts)`)
- `crates/script/src/{scaffold,cli}.rs`; NEW `crates/script/templates/probe/{film.ts,effect.sksl}` (step 2: `sdk new probe NAME`)
- `crates/cli/src/docs.rs`; NEW `crates/cli/src/docs/sdk.rs` (the sdk page now carries ```` ```ts example ```` blocks built from `@example` JSDoc tags)
- NEW `crates/cli/tests/sdk_examples.rs`; NEW `crates/script/tests/sdk_guards.rs`; `crates/script/tests/{builder_errors,loader_scaffold}.rs`
- Generated: `docs/reference/sdk.md` (regenerated), `docs/reference/parity.md` (regenerated after the ledger change)
- `parity/MATRIX.tsv`: row "Docs and probes run as tests" todo -> done (test `every_example_on_the_sdk_page_builds_and_passes_check`, doc `docs/reference/sdk.md`)
- `wp/WP-04/REPORT.md`, `wp/WP-04/probes/{wobble/,diff_faithful.jpg,diff_wrong.jpg}`
Repo root:
- `scripts/capability_index.py`, `scripts/capability_terms.py` (`ranked`), `scripts/test_capability_index.py`; `motions/lib/ae_terms.json` (+ `group`, `texture`); `motions/lib/CAPABILITIES.md` regenerated (670 entries)
- `motions/tools/refstudy/refstudy/{strip,cli,plates(NEW),shots,schema,validate,stages,diff,diff_rows,diff_config,diff_layout(NEW),diff_ssim,sheets}.py`, `refstudy/data/diff_tolerances.json`, `README.md`
- `-nostdin` added to every ffmpeg command in `refstudy/{analysis,audio,audio_hq,frames,loudness,native,raw,scenes,video}.py`, `refstudy/spec/hits.py`, and the test helpers `tests/{synth,test_addendum,test_spatial,test_clone}.py`
- NEW tests `motions/tools/refstudy/tests/{test_plates,test_strip_sheet,test_diff_gate,synth_layout}.py`
Tests added:
- Rust, step 1: `every_plugin_example_in_the_shared_effects_folder_passes_check` (32 plugins), `a_plugin_whose_example_is_wrong_fails_and_one_without_a_header_example_is_tested_on_its_defaults` (`examples_check.rs`); `every_example_on_the_sdk_page_builds_and_passes_check`, `every_example_tag_in_the_sdk_sources_reaches_the_page`, `a_wrong_snippet_fails_with_the_message_a_reader_needs`, `the_block_finder_reads_only_runnable_blocks` (`crates/cli/tests/sdk_examples.rs`); 4 unit tests in `docs/sdk.rs`
- Rust, step 2: 2 probe tests in `loader_scaffold.rs` (the probe builds, checks clean, renders a sheet twice identically and the effect changes the sheet; name validation, never overwrites)
- Rust, step 3: 6 tests in `sdk_guards.rs` (unknown layer fields and `set()` keys throw; `solid({color})` says "solid(color, opts)" (already in `builder_errors.rs`); `layer.effects =` refused with a pointer to `fx`; `texture`; `italic`/`outline_only`; a hidden matte source on an adjustment layer errors), 3 unit tests in `layer_keys.rs`
- Python, step 4 (9, all in `scripts/test_capability_index.py`): any-word ranked by hits, a named entry beats a mention, `--limit`, option splitting, `fields` rows, generator tag, `find matte|group|texture`, `--fields text|group`, `--fields texxt` fails with the engine's did-you-mean
- Python, step 5: `tests/test_strip_sheet.py` (7: one sheet by default, cells never below 96 px, paging past the floor, `--cols`, widest cell that fits, every ffmpeg command in tool and tests starts with `-nostdin`, strip finishes with `q` keys on stdin), `tests/test_plates.py` (7: a mirrored background the histogram detector cannot see is found, a centre element is not, hard cuts excluded, noise and a moving square are not, `shots.json` carries `plate_changes`, schema)
- Python, step 6: `tests/test_diff_gate.py` (10: same film passes; faithful copy passes; the same elements in wrong places FAILS while every old statistic is inside its tolerance; one wrong row of five fails although the warn share is 20% < 25%; all old JSON keys remain; config defaults/validation; box, layout and cell SSIM unit tests)
Gate output:
- `cargo test --release --workspace --no-fail-fast`: exit 0; 38 test binaries, 448 passed, 0 failed, 2 ignored (both pre-existing: `pose_matrices_match_the_after_effects_dump` needs an After Effects dump, `actual_typescript_compiler_checks_types_without_executing_the_film` needs `TURBOMOTION_TEST_TSC`). The first run failed only `the_generated_reference_is_current` because the ledger edit makes `docs/reference/parity.md` stale; fixed by `docs --write`.
- `turbomotion docs --write`: `wrote 7 pages to docs/reference`; `docs --check`: `docs are current (7 pages in docs/reference)`, exit 0
- `python3 scripts/capability_index.py`: `motions/lib/CAPABILITIES.md: 670 entries`; `find matte|group|texture|generator|noise cells` and `find --fields solid|text|group` print the expected lines (checked by tests)
- `python3 -m unittest discover -s scripts -p 'test_*.py'`: Ran 44 tests, OK (35 baseline + 9)
- refstudy: `PYTHONPATH=motions/tools/refstudy python3 -m unittest discover -s motions/tools/refstudy/tests -t motions/tools/refstudy`: Ran 156 tests, OK (132 after `-nostdin`, 146 after the strip and plate tests, 156 with the diff gate tests)
- `turbomotion parity check`: `parity check: ok (62 core rows, 264 effect rows, 97 catalogue effects)`, exit 0; `parity report`: T0+T1 effects done 0 of 172 (unchanged)
- `comp check wp/WP-04/probes/wobble/comp.json`: `ok: 1920x1080 at 30 fps, 2.00s (60 frames), 2 layers, 0 nested comps; 0 assertions; 0 errors, 0 warnings`
- Determinism: `sdk render wobble/film.ts --sheet ... --times 0.5,1.5` twice: `992f58d5...` = `992f58d5...`
- Probes (`wp/WP-04/probes/`):
  - `wobble/sheet.png`: the film `sdk new probe wobble` writes, at 0.5 s and 1.5 s: white "wobble" text turned to dark grey by the new `wobble` effect at `amount 0.8`, with an orange rule under it, on the dark comp background.
  - `diff_faithful.jpg`: reference | faithful copy (1 px off, +2% brightness, noise) at 0, 1 and 2 s; green borders, PASS.
  - `diff_wrong.jpg`: reference | the same title, caption and button in other places; red FAIL borders on all three rows, though colour, edge, motion and palette all pass the old limits.
Thresholds (refstudy diff; also in the `refstudy/diff_config.py` docstring and `refstudy/data/diff_tolerances.json`), measured on the synthetic posters:

| metric | faithful | wrong layout | WARN limit | floor (row fails the whole run) |
| --- | --- | --- | --- | --- |
| `region_ssim_mean` (8x8 grid) | 0.935 | 0.611 | - | `region_ssim_mean_floor` 0.70 |
| `regions_below` (cells with SSIM < `region_ssim_cell_min` 0.5) | 0 of 64 | 24 of 64 | more than `regions_below_max_fraction` 0.15 | - |
| `layout_score` (box F1, IoU >= 0.3) | 0.923 | 0.462 | `layout_score_min` 0.70 | `layout_score_floor` 0.55 |
| colour, edge, motion, palette (unchanged) | pass | pass | the original four | - |

  `pass` = warn share <= `max_warn_fraction` AND no row has `floor_failures`; `budget_pass` is the old test alone. New row keys: `region_ssim_mean`, `region_ssim_worst`, `regions_below`, `worst_region`, `layout_score`, `layout_boxes_ref`, `layout_boxes_ours`, `floor_failures`; new doc keys: `budget_pass`, `floor_failed`, `floor_failed_times`. Old keys and `status` values (PASS/WARN) are unchanged. For scale: the figma-anthem v1 rebuild, which the old limits already warned on 12 of 15 rows, scores `region_ssim_mean` 0.15 to 0.68 and `layout_score` 0 to 0.83 per second, so every row of it is under a floor.
Deviations from this plan:
1. Step 1 plugin test reads `../../../motions/lib/effects` from a crate test (`TURBOMOTION_EXAMPLE_FX_DIR` overrides it). It fails loudly if the folder is missing or has fewer than 30 plugins instead of skipping. None of the 32 plugins declares an explicit `@example`, so the tested example is the descriptor's default-generated one; all 32 pass with no plugin edit.
2. Step 1 sdk snippets: `docs/reference/sdk.md` had no runnable TypeScript, so the mechanism is a new JSDoc tag `@example` on SDK functions, rendered as ```` ```ts example ```` blocks by the docs generator (`crates/cli/src/docs/sdk.rs`, split out of `docs.rs` to stay under 300 lines). 19 examples; the test is `crates/cli/tests/sdk_examples.rs` (the cli crate owns the generator and depends on `script`). `light` had no doc comment; I wrote one.
3. Step 3 unknown fields: instead of a second key list in TypeScript, `Layer` asks the engine (`__tm_layer_problems`, backed by `schema::layer_keys`, which also lists the pre-expansion keys `repeat`, `extrude`, `on`, precomp `block`, shape `surface`). Behaviour change: an unknown key now throws at construction, not at build; `builder_errors.rs::many bad layers` was changed to use value errors because key errors now surface first. `layer.effects =` is refused through a Proxy (every non-state field assignment is refused, with a special message for `effects`/`effect` pointing at `fx`).
4. Step 3 `texture`: plan section 3.4 said there is no texture helper; the plan text of step 2 and 3 names `texture(shader, opts)`, so `texture()` is added (`layers.ts`, types in `raw-types.ts`). `solid({color})` already threw "solid(color, opts)" at HEAD; only a test on the exact message was needed and it exists.
5. Step 3 matte: the SDK errors when a `matte` source on an adjustment layer is hidden. The engine IGNORES a `matte` on an adjustment layer whether the source is visible or hidden (adjustment layers are limited only by `masks`). That is an engine gap, not an SDK one: see open question 1.
6. Step 4 `matte`, `group`, `texture`: the index already had entries (layer-key `matte`, layer `null`, sdk `group`, sdk `texture`); what was missing was finding them first. I added AE terms `group` and `texture` to `ae_terms.json` and ranked exact names first. `find` now cuts at `--limit 30` (default; `--limit 0` shows all) and says how many more there are, because any-word matching on two words returns hundreds of lines.
7. Step 5: much of it was already at HEAD (the `END_EPS` constant, `--cols`, `--crop`, clamping, a histogram `soft_cuts` detector with tests). Done now: one contact sheet by default (cells 320 px shrinking to 96 px, paging only beyond that or with `--scale`), `-nostdin` on all 26 ffmpeg command lists in the tool and its tests, and the plate-change detector (`plates.py`, `plate_changes` in `shots.json`, schema, review table). "Plate change" had no definition in the plan: I defined it as the median colour of the top, bottom, left and right border bands changing at once, which sees a mirrored or swapped background that keeps the histogram and ignores a centre element.
8. Step 6: all new thresholds are calibrated on synthetic posters (two rows of numbers above) and sanity-checked on one real pair. I did not tune on the 4.7 GB `rebuild_v2/.../proof.mp4` (decoding it did not finish in 5 minutes); I used the 4 MB v1 proof instead.
9. Not verified: the new TypeScript was never run through `tsc` (none is installed; the typecheck test is ignored without `TURBOMOTION_TEST_TSC`). Everything runs on QuickJS and is covered by the tests above, but a typing mistake would not show.
10. `-nostdin` could not be proven necessary by a failing test (ffmpeg on a short clip finishes before it reads a key); the test is a static guard on every command plus a smoke run with `q` bytes on stdin.
Open questions for the owner:
1. Should the engine apply `matte` on an adjustment layer (what After Effects does), or refuse it at `comp check` for every case? Today it is silently ignored. I suggest WP-14 (adjustment layers) decides; the SDK's hidden-source error is correct either way.
2. `layout_score` and region SSIM are computed on the 160x90 analysis frames, so text under about 12 px tall at 1080p is not a block. Do you want a second, native-resolution pass for small captions (slower; per-row cost about 20x)?
3. The new floors make `refstudy diff` fail films that the old gate passed. Existing studies under `motions/` that cite a `pass: true` are not rerun; any `--config` you already use keeps working (the structural keys fall back to the packaged values).
Estimated cost to finish (if partial): n/a
