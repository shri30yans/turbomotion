WP: WP-00 Parity ledger
Status: done
Changed files:
- NEW `parity/MATRIX.tsv` (62 core rows: the 36 of section 7a, 17 for the other WPs, 9 `not_supported` decisions), `parity/effects.tsv`
  (264 rows: 210 AE effects from section 10 plus 54 engine-only catalogue effects), `parity/{specs,probes,golden}/.gitkeep`
- NEW `crates/cli/src/parity/{mod,ledger,check,report,init_effect}.rs`; `crates/cli/tests/parity.rs`
- `crates/cli/src/main.rs` (registers `parity`), `crates/cli/src/docs.rs` (page `parity.md` from the ledger)
- `docs/reference/parity.md` (generated), `docs/reference/README.md`
- `scripts/capability_index.py` (`not-built` / `not-supported` entries from `parity gaps --json`); `motions/lib/CAPABILITIES.md` regenerated (670 entries)
- `docs/BUILD_PLAN.md`: ADR-6 note about the two extra columns; Roughen Edges corrected (deviation 3)
Commands: `parity check | report | gaps [--json] | init-effect <name> [--ae "AE Name"] [--category C]`
Tests added:
- `parity::check::tests`: a_sound_ledger_has_no_problems, a_done_core_row_without_a_test_or_a_doc_fails (the plan's fixture test),
  a_done_row_must_name_a_test_that_exists, an_effect_marked_done_that_is_not_in_the_catalogue_fails, a_catalogue_effect_with_no_row_fails,
  bad_vocabulary_and_duplicates_are_named_with_their_line, a_done_effect_needs_its_spec_and_probe_files, slugs_are_file_names
- `parity::report::tests::the_report_counts_each_category_and_the_tier_yardstick`, `parity::tests::the_page_lists_gaps_by_category_and_separates_decisions`
- `parity::init_effect::tests`: it_scaffolds_every_file_and_takes_over_the_todo_row, it_appends_a_row_for_an_unlisted_effect_and_refuses_to_overwrite,
  it_rejects_bad_names_and_unknown_or_started_rows
- `tests/parity.rs`: the_real_ledger_is_sound, the_gap_list_names_what_is_not_built_and_what_is_decided_against, the_report_prints_the_tier_yardstick
- Manual: `init-effect zz_scaffold_probe` was run on the real tree: the shader compiled and joined the catalogue, the stub tests passed (2 ok, 1 ignored by design),
  `comp check` of the probe passed, and `parity check` accepted the new row. Then every file it made was removed and both edited files restored.
Gate output:
- `cargo test --release --workspace --no-fail-fast`: 422 passed, 0 failed (exit 0)
- `turbomotion docs --write` then `--check`: 7 pages, current; `git diff --stat docs/`: ARCHITECTURE.md (owner's edit) and reference/README.md (+1 line, the new page)
- `python3 scripts/capability_index.py`: 670 entries; `find "box blur"` returns `not-built | Box Blur | not built yet (WP-02)`; `find "slider"` returns `not-built | Slider Control (WP-11)`
- `python3 -m unittest scripts/test_capability_index.py`: OK (15 tests)
- `parity check`: ok (62 core rows, 264 effect rows, 97 catalogue effects)
- `parity report`: T0+T1 effects done: 0 of 172 (0.0%); core model 1.9% done (WP-52 and WP-00 rows)
- comp check on probes / determinism / probe images: not applicable; WP-00 renders nothing (the init-effect probe was checked as above)
Deviations from this plan:
1. `effects.tsv` has two extra columns, `engine` and `test` (ADR-6 lists eight). An AE name and an engine name differ ("Gaussian Blur" is `blur`),
   and "done" needs a named test. Recorded in ADR-6.
2. Step 4 says the catalogue prints a `not_supported` list. `comp catalogue` lives in `turbomotion-comp`, which cannot see the ledger beside the CLI,
   so the list is a generated docs page (`docs/reference/parity.md`), `parity gaps [--json]` and `CAPABILITIES.md` entries. `comp catalogue` JSON is unchanged.
3. The plan claimed "Roughen Edges [have]"; the catalogue has no such effect (`highlighter` is a different one). The row is `todo` (A, T1) and section 10 now says so.
   Similar name drift: the plan's `god_rays` is `shafts`; `film_grain` and `roughen_edges` in WP-01's list do not exist in the catalogue.
4. `done` rows require test + doc (core) or test + spec + probe (effects); `golden/` is not required yet because nothing writes goldens before WP-02.
5. Seeding used a one-off script (`tmp/fonts-incoming/parity-seed.py`, not in the engine repo). Ambiguous groupings in section 10 were split into one row each
   (Auto Color / Contrast / Levels; the CC simulation effects). Tiers for the untagged ones (CC Ball Action, CC Drizzle, CC Hair, CC Mr. Mercury) are my choice: T2.
6. Effect-row WPs shown in `gaps` come from the category (Blur -> WP-02, Keying -> WP-35, warps -> WP-34, Time -> WP-13, ...), the rest WP-33.
Open questions for the owner:
- Is `chroma_key` correctly mapped to AE's Color Key (marked `part`)? It also removes spill, which is closer to Keylight.
- `have`/`part` rows are mapped by summary text, not by probe, so they are unverified claims in the plan's own sense. WP-33 re-checks each against AE parameters.
Estimated cost to finish (if partial): n/a
