WP: TASK-0 Make `cargo test --release --workspace` green (BUILD_PLAN section 0), plus verification of WP-52 (fonts) and WP-00 (parity ledger) against the current plan text
Status: done
Changed files:
- Engine, examples fixed at their source: `crates/comp/src/catalogue.rs` (`extrude` example, `tokens` example), `crates/comp/src/catalogue/property_examples.rs` (glass panel example)
- Engine, harness (written earlier in this session; the plan said it was at HEAD, it is untracked): `crates/comp/tests/examples_check.rs`, `crates/comp/tests/examples/{mod.rs,fixtures.rs}`
- Generated: `docs/reference/layers.md` (the `extrude` example), regenerated with `docs --write`; `docs/reference/{README,parity}.md` unchanged by this task
- Reverted (the earlier `decor` attempt, restored to HEAD with `git checkout --`): `crates/comp/src/schema/mod.rs`, `crates/comp/src/schema/extrude.rs`, `crates/comp/src/check/lint.rs`, `crates/script/sdk/types.ts`. Before reverting, `git diff` of each held only the `decor` hunks. `rg decor crates` now finds no layer key.
- Kept, outside plan paths: `crates/script/tests/cli_summary_and_layers.rs` (per-test scratch folders; the shared `film.ts` race made it flaky about 1 run in 7)
- Python / tmp hygiene: `scripts/font_info.py` (docstring: how to run it with the venv), `motions/lib/CAPABILITIES.md` (regenerated, 670 entries, same as before)
- `tmp/` (only paths we created): `tmp/venv-fonts` replaced by `tmp/fonts-incoming/venv` (recreated with `python3 -m venv`, `pip install fonttools brotli`: fonttools 4.60.2, brotli 1.2.1, the same versions; shebangs of a moved venv would have broken), `tmp/parity-seed.py` moved to `tmp/fonts-incoming/parity-seed.py`; references updated in `tmp/fonts-incoming/_ingest.py`, `wp/WP-52/REPORT.md`, `wp/WP-00/REPORT.md`, `scripts/font_info.py`. The old `tmp/venv-fonts` folder was removed after the new venv worked (it was ours; this completes the move). `tmp/clones/*` untouched.
- `wp/TASK-0/REPORT.md` (this file)

Examples changed (where declared -> what was wrong -> what changed):
| Example | Declared | Wrong | Change |
| --- | --- | --- | --- |
| `extrude` (also `layers.md` block) | `crates/comp/src/catalogue.rs` entry `extrude` | 24 depth slices are text layers; default slice colour `#7a1d14` fading to 55% of it has contrast 1.1 to 1.6 against the `#1a1d29` ground (floor 3:1); the face text was tiny (default size) and left-aligned; no camera | face `#ffd166`, `size 220`, `weight 800`, `align center`; slices `color #e8590c` to `far #c2410c` (explicit `far`, contrast 3.2:1 or better at the far end; the 55% default only suits a light near colour); the camera comes from the harness (below). Checked: `comp check` 0 errors 0 warnings; sheet shows a legible amber 3D number with orange sides |
| `tokens` | `catalogue.rs` `"tokens"."example"` | two text layers at the default `pos` (0,0), left-aligned: text reaches x < 0, `outside-safe-area` x2. (The earlier failure "colour accent not in palette" was a harness bug: the example's `type` key is the text scale object, and the classifier took any object with a `type` key for a layer; it now requires `type` to be a string) | `pos [960,440]` / `[960,640]`, `align center` |
| glass panel (property example) | `crates/comp/src/catalogue/property_examples.rs` | text size 90 spans 23..617 px of a 640 px film: `outside-safe-area`, and wider than the 400 px glass panel it is meant to sit behind | `size 56` (about 370 px, inside the panel and the safe area) |
| `image` | `catalogue.rs` layers entry `image` | fixture was 1280x720; the example zooms 1.0 to 1.3 on a 1080p film (cover scale 1.5 x 1.3 = 1.95x): `upscaled-media`. The example is right (a Ken Burns needs a large source; its own text says to size the source for the zoom) | fixture only: `photo.jpg`/`png`/`webp` are 3840x2160 (ratio 0.65x at full zoom). Example unchanged |
| `mesh` (`layers.md` block) | `catalogue.rs` layers entry `mesh` | `three_d` without a camera: `3d-without-camera` | example unchanged; harness puts the catalogue's own `camera` example beside any layer fragment with `three_d: true` |
| `light` (`layers.md` block) | `catalogue.rs` layers entry `light` | a light with no 3D layer: `light-without-3d` | example unchanged; harness puts the catalogue's own `mesh` example (and so a camera) beside a `light` fragment |

Why the harness, not the examples, for mesh, light and extrude's camera: a single layer cannot carry its own camera or its own lit solid. The catalogue documents both preconditions itself ("Layers with three_d are projected through it" under `camera`; "3D layers are shaded by every active light" under `light`). `Declared::in_context` (tests/examples/mod.rs) adds exactly two things: the catalogue's `camera` example when a layer has `three_d` and the film has no camera, and the catalogue's `mesh` example when a light has no 3D layer. Both are looked up from the catalogue, none invented. No rule allow-list, no lint change, no `#[ignore]`.

The third test (`an_effect_example_that_forgets_its_declared_layer_input_fails_instead_of_being_helped`): a TEST bug, not an engine bug. `err.to_string()` on an `anyhow` error is only the outermost context ("the root composition"); the cause chain holds the message. The command line prints the whole chain; I confirmed with `turbomotion comp check` on the same film:
```
Error: the root composition
Caused by:
    layer 0 (solid): fx 0: effect "turbulent_displace" needs "noise": the id of the layer whose picture it reads ("noise": "some_layer") -- a noise picture, for example a `noise` texture layer (hidden); its brightness is the field
```
That message names the input, the layer, the fix and an example, so the engine is fine. The test now asserts on `format!("{err:#}")` (the chain). It stays meaningful: it still requires validation to fail and the message to name `noise`, and its second half still proves no layer is invented for a value that is not a declared input.

Tests added:
- `examples_check::a_fragment_is_placed_beside_what_the_catalogue_says_it_needs_and_nothing_else` (a 3D layer gets `["text","camera"]`, a light gets `["mesh","light","camera"]`, a flat solid gets only itself, a composition whose `type` is an object is not taken for a layer)
- Changed: `an_effect_example_that_forgets_...` (asserts on the cause chain)
- The existing harness is the test for the example fixes: `every_catalogue_example_passes_check` (188 examples) and `every_json_block_in_the_reference_pages_passes_check` (115 blocks), both zero findings including warnings.
- Removed with the revert (they tested the reverted `decor` key): `the_slices_are_decoration_and_the_face_is_content`, `decoration_is_not_judged_for_legibility_but_content_is`.

Gate output:
1. `cargo test --release --workspace --no-fail-fast`: 427 passed, 0 failed, 2 ignored (the 2 pre-existing ignores: `ae_rotation`, `typecheck`). `examples_check`: `test result: ok. 5 passed; 0 failed`. `turbomotion-comp` lib 248 passed.
2. `cargo run --release -p turbomotion -- docs --write` -> `wrote 7 pages to docs/reference`; `docs --check` -> `docs are current (7 pages in docs/reference)`; `git diff --stat docs/`: `ARCHITECTURE.md` (owner's), `reference/README.md` (+1, the parity page link), `reference/layers.md` (the `extrude` example); `parity.md` is new (untracked).
3. `python3 scripts/capability_index.py` -> `motions/lib/CAPABILITIES.md: 670 entries`; `find "box blur"` -> `not-built | Box Blur | not built yet (WP-02)`; `find "Bebas"` -> the `font` entry with its `comp({ fonts: ... })` snippet. `python3 -m unittest scripts/test_capability_index.py` -> `Ran 15 tests ... OK`; `python3 -m unittest discover -s scripts -p 'test_*.py'` -> `Ran 35 tests ... OK` (none failing, so nothing in `scripts/capability_sdk.py` to report).
4. `turbomotion parity check` -> `parity check: ok (62 core rows, 264 effect rows, 97 catalogue effects)`, exit 0. `parity report`: `T0+T1 effects done: 0 of 172 (0.0%)`; core model 3.8% done (2 of 62 rows, WP-52 and WP-00). The ledger needed no edit.
5. `comp check` on probes: no probe JSON exists for this task; the `extrude` example as a whole film (`/tmp/t0/ex.json`, with the catalogue camera) -> `0 errors, 0 warnings`; `comp check motions/lib/fonts/specimen.json` -> `0 errors, 0 warnings, 46 layers`.
6. Determinism / 7. Probe images: not applicable to a test-and-examples task; one sheet of the `extrude` example was opened (a legible amber "88888" with orange sides, centred).

WP-52 verification against the current plan text (no deltas needing code):
- 45 families in `fonts.json` (`fonts` list, `rejected` empty): the 42 of the plan list plus Manrope, Instrument Serif, JetBrains Mono; all files exist and every sha256 matches the file; each record has `family, files, weights, axes, tags, has_block_elements, licence, source_url, sha256`; licences OFL-1.1 and Apache-2.0 only; a licence file beside each. U+2588 is present in Bungee and DotGothic16 only; JetBrains Mono (bundled) was checked and does not have it (recorded as false in fonts.json).
- `scripts/font_info.py` prints family, weights, axes, glyph count, U+2588, U+00A0 (and sha256) and runs as `-I`; `SPECIMEN.png`, `specimen.json` and the capability entries exist; `comp check` finds each family by name through `comp({fonts})` roles (WP-52 report deviation 3 stands: no engine font search path without touching `text/layout.rs`).
WP-00 verification against the current plan text (ADR-6 and its 2026-10-26 note, section 7 WP-00, 7a, 10): all five steps are present; `check` fails on a done row without test/doc, on a done effect missing from the catalogue and on a catalogue effect without a row; `report` prints coverage per category; `init-effect` scaffolds the header, probe, test stub and row. Two points where the text asks for more than the tree gives, both already recorded in `wp/WP-00/REPORT.md` and left as is: (a) step 4 says the catalogue itself prints the not-supported list; it is in `docs/reference/parity.md`, `parity gaps` and `CAPABILITIES.md` instead, because `comp` cannot see a ledger beside the CLI; (b) ADR-6 mentions golden files for `done` rows but its own note requires only test + doc (core) and spec + probe (effects), which is what `check` enforces; `parity/golden/` is empty.
`wp/WP-52/REPORT.md` and `wp/WP-00/REPORT.md` were edited only for the moved venv and seed-script paths; their older test counts (422 passed) are history, not corrected.

Deviations from this plan:
1. The plan calls the harness "already existing" and says 3 tests fail on 30 + 28 examples; the 30 and 28 are findings, not examples (6 distinct examples and one test bug). The harness is not at HEAD: it is untracked work from the earlier agent in this session.
2. Fragment examples (mesh, light, the `extrude` camera) are fixed in the harness, not in the catalogue text, as the task allows for inherent fragments; the docs still show them as layers.
3. `tmp/venv-fonts` was recreated, not moved (a moved venv keeps absolute shebangs); the old copy was deleted after the new one ran `font_info.py`.
4. Test count 427 differs from the plan's 423 + 3 = 426 because the new harness test and the 5 parity/ledger tests were added since, and the 2 reverted `decor` tests were removed.

Open questions for the owner:
- The `extrude` default `far` is 55% of `color`. A light near colour is needed to stay readable on a dark film; should the default be lighter (say 75%)? Not changed: it is behaviour, not an example.
- Section 3.4 of the plan still says `parity/` does not exist and fonts has 3 families; it describes the state before any WP and is now stale. Update or leave as history?
- The mesh and light fragments rely on the harness; if you prefer the docs to show a whole film for 3D, the catalogue needs an optional "also needed" field, which is a docs-format change.

Estimated cost to finish (if partial): n/a

State for the next session: task 0, WP-52 and WP-00 are done and green. Next per section 0 is item 3, WP-04 (docs and probes as tests: step 1 extends the now-green harness to every plugin `@example` in `motions/lib/effects/` and the TypeScript snippets in `docs/reference/sdk.md`; step 6 strengthens `refstudy diff`). Then WP-01 (opt-in `effect_space`).
