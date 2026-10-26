WP: WP-52 Fonts
Status: done
Changed files:
- NEW `scripts/font_info.py` (fontTools; prints family, style, weights, axes, glyph count, U+2588 and U+00A0, sha256)
- NEW `motions/lib/fonts/<Family>/` for 42 families (45 listed in `fonts.json` with Instrument Serif, JetBrains Mono, Manrope), each with its licence file
- NEW `motions/lib/fonts/fonts.json`, `specimen.json`, `SPECIMEN.png`
- `scripts/capability_index.py` (adds `font` entries with the `comp({fonts})` snippet), `scripts/test_capability_index.py` (class `Fonts`)
- `motions/lib/CAPABILITIES.md` regenerated (452 entries; 45 `font | ...` lines)
- `turbomotion/crates/script/tests/cli_summary_and_layers.rs` (deviation 1)
Tests added: `Fonts.test_each_listed_file_exists_and_matches_its_hash`, `Fonts.test_the_plan_families_are_all_present`,
`Fonts.test_the_index_lists_each_font_with_its_snippet`, `Fonts.test_the_specimen_film_passes_check_with_every_family_named_without_a_path`
Gate output:
- `cargo test --release --workspace --no-fail-fast`: 406 passed, 0 failed (exit 0)
- `python3 -m unittest scripts/test_capability_index.py`: Ran 15 tests, OK
- `turbomotion docs --write`: wrote 6 pages to docs/reference; `git diff --stat docs/` shows only the owner's staged ARCHITECTURE.md edit
- `python3 scripts/capability_index.py`: 452 entries; `find anton` and `find pixel font` return the font lines
- `parity check`: not applicable (WP-00 not built yet)
- `comp check motions/lib/fonts/specimen.json`: ok, 46 layers, 0 errors, 0 warnings
- Determinism: two `comp still` renders: e61b10bf... = e61b10bf...; two `comp sheet` renders: 84957d43... = 84957d43...
- Probe: `wp/WP-52/probes/specimen.png`: 45 lines, each family name drawn in its own face; every one is visibly a different design (DSEG7 shows seven-segment digits).
Deviations from this plan:
1. Fixed an existing flaky test (not in this WP's paths): `cli_summary_and_layers.rs` had two tests writing the same `film.ts` in
   one scratch folder, failing ~1 run in 7 with "the script has no default export". Each test now has its own folder. Verified 12/12 passes
   after the change. Reason: the gate requires a zero-failure suite and every later WP inherits this flake.
2. Python run as `python3 -I` cannot see user-site `fontTools`; `font_info.py` runs as `tmp/fonts-incoming/venv/bin/python -I scripts/font_info.py`
   (venv has `fonttools`, `brotli`). Ingest was a one-off script in `tmp/fonts-incoming/_ingest.py` (not committed).
3. "check must find each family by name without a path" is met through the comp's `fonts` roles (name -> file) used as `"font": "<Family>"`.
   The engine has no font search path for `motions/lib/fonts`; adding one means changing `text/layout.rs`, which this WP does not name.
4. Fonts are fetched from google/fonts at commit e7db6ca603fd (42 families; 4 are Apache-2.0 under `apache/`), DSEG7 from keshikan/DSEG
   v0.46 (zip sha256 a6c2f435...). Only the five `DSEG7-Classic` TTFs were installed (Modern, 14-segment, MINI, Weather skipped).
Open questions for the owner:
- Variable fonts render at their default instance until WP-32 adds `variations` (Archivo width axis, Fredoka, League Gothic).
- JetBrains Mono's bundled `.woff` has no U+2588 (recorded `has_block_elements: false`); of the new fonts only Bungee and DotGothic16 have it.
- `CARGO_TARGET_DIR` in this environment points into a temp cache, so `turbomotion/target/release/turbomotion` (the default for
  `capability_index.py`) is the binary from before this session; set `TURBOMOTION_BIN` or unset `CARGO_TARGET_DIR` for fresh builds.
Estimated cost to finish (if partial): n/a
