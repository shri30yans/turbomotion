# Architecture and code-quality audit: turbomotion

Date: 2026-10-27. Reviewer: architecture audit pass (read-only).
Source of truth for reading: `git archive HEAD` → `/tmp/review-src` (deleted after). HEAD `3b191de`, branch `workspace-split`, working tree clean at start.
Prior research: `wp/REVIEW/research.md` (read fully; this audit engages its recommendations, does not overwrite it).
Scope: no edits under `crates/`, `parity/`, `docs/`, `motions/`. Only this file written under `wp/REVIEW/`.

How to read tags: **[F]** fact with file:line or measured number; **[I]** inference; cost **S/M/L/XL** as in research.

---

## Verdict (one paragraph)

The staged pipeline (schema → plan → eval → render), pure-function frame, frame graph, layer-space bounds growth, single blend table, Skia/SkSL plugins, and QuickJS authoring SDK are the right skeleton for After Effects Classic parity — **agree with research: do not rebuild**. What is wrong is strategy and honesty, not topology: the system is **incorrect-by-default** (screen-space effects, energy-losing Skia blur) so that every correctness fix is an opt-in that old films never take; the parity ledger marks features `done` from engine self-tests with **zero AE measurements** (the only AE dump test stays `#[ignore]`); schema truth is split across Rust types and a ~7.5k-line hand-written TS SDK; and the WP gate spends agent days proving byte-identity to the *old wrong* look. Before more effect breadth or 3D shading, flip the measurement and default story: AE corpus + ledger gate, make AE-correct paths the default (with an explicit legacy escape), generate schema artefacts, add a content-hash pass cache. Keep Skia, SkSL, QuickJS; do not chase OpenFX, Roto Brush, Advanced 3D, or a renderer rewrite.

---

## Snapshot numbers (measured on archive of HEAD)

| Quantity | Value | Evidence |
| --- | --- | --- |
| Rust LOC (`crates/`) | ~45k lines, 377 `.rs` files | `wc` on archive |
| By crate | core 6.3k, comp 34.3k, script 2.9k, cli 1.5k | same |
| SkSL | 102 files, ~2.3k LOC; 72 builtin plugins | `find` / `wc` |
| Platform registry effects | 32 named (`blur` … `transform`) | `effects/mod.rs` `registry!` |
| Blend modes | 37 in one table | `blend/table.rs:79` `MODES: [Mode; 37]` |
| `#[test]` attrs | 928 | `rg` |
| Workspace tests (WP-02 gate) | **923 passed, 0 failed, 2 ignored** | `wp/WP-02/REPORT.md:30` |
| Growth | WP-01: 514 → WP-13: 836 → WP-02: 923 | reports |
| Files >300 lines | **16** (worst: `eval/mod.rs` 696, `schema/tokens.rs` 644, `schema/text.rs` 607) | `wc`; violates BUILD_PLAN rule 4 |
| `.unwrap()` | **801** total; **~315** outside `*test*` paths | `rg` |
| TS SDK | ~7.6k lines hand-written; `types.ts` 426 | `wc crates/script/sdk` |
| schemars / ts-rs / specta | **none** | `Cargo.toml` |
| Parity effects.tsv | 266 rows: **done 6, part 24, have 68, todo 168** | DictReader |
| MATRIX.tsv | 95 rows: done 36, none 29, part 12, not_supported 9, have 7, unverified 2 | same |
| Specs / probes | 17 specs, 8 probe JSONs | `ls parity/` |
| AE match names (`ADBE…`) | **zero** in crates/parity | `rg` |
| Identity artefacts | 10 WP identity dirs, 64 files | `find wp -path '*/identity/*'` |
| WP REPORT prose | **1236 lines** across 14 reports | `wc wp/*/REPORT*` |
| History cache | 512 MiB LRU, history-only | `render/history.rs:31` |
| Content-hash / ROI | **absent** | `rg` empty for real ROI/hash cache |
| skia-safe | `0.87` + metal + textlayout | root `Cargo.toml:16` |
| macOS/Metal | hard dependency (`metal`, skia metal feature) | `core/Cargo.toml` |

*(Stale note: archive cohabited with an old `target/release/turbomotion` that failed `parity check` on missing catalogue names; source at HEAD *does* register `box_blur` / `lens_blur` / `unsharp_mask` at `effects/mod.rs:107–124`. Numbers above are from source and WP reports, not that binary.)*

---

## 1. Architecture fitness for full AE parity

### What is already right

| Concern | State | Evidence |
| --- | --- | --- |
| Frame = f(t) | **Has it** | `ARCHITECTURE.md` principle 1; history re-derives past via same graph (`history.rs:1–13`) |
| Stages | schema → plan → eval → render | `ARCHITECTURE.md:70–85` |
| Frame graph | Named images, topo emit, drop after last reader, matte/clip once | `render/graph/build.rs:27–160` |
| Layer-space + bounds growth | **Implemented, opt-in** | `effect_space` default **screen** (`schema/effect_space.rs` / catalogue default `"screen"`); layer path in `render/graph/local.rs`, `eval/buffer.rs` |
| Precomps | Nested comps, time remap, cycles refused | `plan/mod.rs` Precomp cycle checks ~183–422 |
| Mattes | Alpha/luma; place-before-matte in layer space | `graph/build.rs:128–131`; `layer_space_tests.rs:67–74` |
| Plugins | Header DSL `@effect/@param/@input/@pass/@bounds/@space/@example` | `fxlib/header.rs:89` lists tags; **no** `@roi`, `@identity`, `@frames_needed`, `@ae` |
| Colour | Opt-in linear, profiles, dither; AE-default display blend | WP-03 / catalogue `color.rs`; agrees with research §3 |
| Blends | One table, closed forms, 37 modes | WP-05; `blend/table.rs` |
| Property model v2 | Temporal/spatial handles, roving, hold | WP-10; **constants from EffectCraft AE 26.3, not owner AE** (`property_model.md:29`) |
| Motion blur | Sub-frame average, opt-in | WP-13; root-only nesting deviation documented |

**Agree with research:** keep Skia/SkSL, QuickJS for authoring, pure frame, layer-space design, Classic 3D plan (CPU raster + G-buffer), no OpenFX / OCIO / Roto / Advanced 3D.

### Gaps ranked by cost-to-fix-later

| Rank | Gap | Why cost escalates | Cost now vs later |
| --- | --- | --- | --- |
| 1 | **No AE ground truth; `done` = self-test** | Every effect/template built on wrong sigma / blend / blur nesting must be reworked after first corpus | **S–M now** vs **L–XL** after breadth |
| 2 | **Incorrect defaults** (`effect_space: screen`, `quality: legacy`) | Films and SDK habits lock the wrong pipeline; flipping later is a migration project | **M now** (escape hatch + new default) vs **XL** after library grows |
| 3 | **Schema dual source** (Rust + hand TS) | Agent edits one side; drift tests only compare *key names*, not types/ranges/docs (`sdk_option_types.rs:1–33`) | **M now** vs permanent tax |
| 4 | **Classic 3D / mesh** still painter's algorithm, no depth buffer | Templates needing intersecting planes wrong; materials cannot be parameters | Already P1/P2 in BUILD_PLAN; **L–XL**; do **after** camera corpus (research §6) |
| 5 | **Expressions** = tiny total language (`anim/expr.rs:1–4`), not AE JS | WP-12 is XL; every film expression written in the mini language is throwaway or dual-path forever | Design host-lib + dependency log **before** shipping WP-12 surface (research §4) — **L** |
| 6 | **No content-hash pass cache** | Only history LRU (`history.rs:31–70`); seeks and re-renders recompute; no determinism key | **M**; cheap while eval is pure |
| 7 | **No ROI / identity / frames_needed in plugin contract** | Large comps and temporal effects stay full-buffer; hard to add without touching every plugin | Header grammar **M** with match-name column (research §1) |
| 8 | **Motion blur nesting ≠ AE** | Root-only shutter (`motion_blur.md:23–25`); fixing after many precomp films is painful | Record + measure first (**S**); adaptive samples **M** |
| 9 | **Mask-before-effects order** still `none` (WP-14) | Layer space fixed space but not AE order | WP-14 **M** |
| 10 | ROI tiling for 8K | Not a parity blocker | **Defer L** (research) |

Precomps/mattes/graph are structurally fine for AE Classic compositing. The dangerous absences are **measurement**, **defaults**, and **3D depth**, not the graph algebra.

---

## 2. Correctness by construction

### Schema vs hand-written TS SDK

- Engine: serde types, strict unknown fields, did-you-mean — good.
- SDK: hand-authored interfaces (`film-types.ts`, `types.ts` 426 lines, etc.).
- Guard: `sdk_option_types.rs` parses Rust `const` lists / `struct` fields / TS `interface` keys and asserts **name sets** match. It does **not** check types, optionality, ranges, defaults, or docs.
- No `schemars` / `ts-rs` / `specta` / `typify` in the workspace.

**Verdict:** not correct-by-construction; correct-by-convention with a weak tripwire. **Agree with research §7:** generate JSON Schema + TS from Rust; keep drift tests as second guard. Cost **M**.

### Plugin header DSL

Strong for what it covers: params, inputs (layer/history), passes, kernel passes, bounds, colour space (`header.rs:70–89`). Unknown tags hard-fail with an allow-list — good.

Missing for AE-host parity vocabulary (research §1–2): `@ae` match name, `@identity`, `@frames_needed`, `@roi`, typed param kinds beyond float/vec2/color/choice. Extending the grammar is **M** and should happen before the next wave of plugins so headers stay the single descriptor.

### Parity ledger honesty

| Claim | Reality |
| --- | --- |
| BUILD_PLAN goal | Explicitly "**not** pixel-matching AE" (`BUILD_PLAN.md:70`) — unfalsifiable "parity" |
| `effects.tsv` `done` (6) | Engine closed-form / self-pixel tests only. **Gaussian Blur is `done`** while `parity/specs/gaussian_blur.md:56–61` marks AE radius→sigma **`UNSURE`** (`sigma = blurriness/2`) and says nothing was measured against AE |
| `have` (68) | No test column filled (68 empty) — catalogue existence ≠ behaviour |
| MATRIX `done` (36) | Same pattern: strong internal tests (blends within 1 level of closed form — WP-05) but **no AE row** |
| AE dump | `crates/comp/tests/ae_rotation.rs` `#[ignore]` — needs golden from `tools/ae-export/…` |
| Match names | No `ADBE …` column anywhere |
| Property handles | Spec cites EffectCraft’s AE 26.3 run as authority (`property_model.md:29`), not owner AE |

`parity check` enforces structural links (engine name exists, test fn greppable) — it does **not** require `measured` / tolerance / `ae_version`. **Agree with research:** add ledger columns and refuse `done` without measurement or waiver. Cost **S**.

---

## 3. Hacks and smells

### Legacy vs exact parallel paths (incorrect default)

Documented pattern, repeated:

| Path | Default | Exact / AE-shaped | Evidence |
| --- | --- | --- | --- |
| Effect space | `screen` | `layer` | catalogue default `"screen"`; WP-01 identity proves default = old pipeline |
| Blur family | `quality: legacy` (Skia; light wrong above σ≈4) | `quality: exact` (separable kernel) | `gaussian_blur.md:44–45`; WP-02 light table: legacy impulse energy 0.00–1.27 depending on size; exact ≈1 |
| Halation / glow / shadow / backdrop / tilt_shift / radial | same `legacy` default | `exact` | `halation.rs:5–6`; `tilt_shift.sksl:8`; quality wired in blur/halation/backdrop/smear/… |
| Motion blur | absent = sharp; old `"mblur": {samples, shutter}` kept alongside `"motion_blur"` | new shutter block | `motion_blur.md:27–29` — dual keys forever |
| Colour / dither | off / none | opt-in | intentional and AE-aligned for colour |

WP-02 reports exact Gaussian σ=16 at 1080p ≈ **17 ms** blur cost vs legacy cheaper — performance used to justify keeping wrong default. That is a product choice, not correctness-by-construction.

**Smell:** two renderers for one concept, with the broken one shipping. Each new blur-adjacent effect inherits the fork (`quality.rs` and callers).

### File size debt

BUILD_PLAN rule 4: split past 300 lines. **16 files violate**; several are the hottest modules (`eval/mod.rs` 696, `fxlib/mod.rs` 534, `anim/expr.rs` 490, `plan/mod.rs` 433). WP reports repeatedly note “already over 300, grew by declarations only” — the rule is aspirational.

### Unwraps

~315 outside tests. Many are in schema helpers / kits where panic-on-bug is arguable; still contradicts “no unwrap on user input.” Not the top risk vs defaults/measurement, but a hygiene debt.

### Duplication / dual models

- Mini expression language **and** planned QuickJS AE expressions (BUILD_PLAN WP-12 dual-parse strategy: old parser first).
- Two motion-blur schemas.
- Platform effects in Rust registry **and** SkSL builtins **and** external `fx_path` plugins — three discovery surfaces (manageable if descriptors unify).
- Catalogue summaries that restate specs (long strings in `catalogue/*.rs`) — generation would shrink drift.

---

## 4. Opt-in / byte-identical-default strategy

**Yes: the system is incorrect-by-default relative to AE Classic behaviour**, on purpose.

- Screen-space effects: pixel radii do not scale with layer; off-screen content culled before effects — the opposite of AE (`effect_space.md`, BUILD_PLAN P5).
- Legacy blur: known non-conservative energy; halos systematically wrong for small sources (WP-02 audit table).
- Byte-identity gates (`wp/*/identity/`) **prove the wrong look is preserved**, then ship. That is excellent regression engineering aimed at the wrong north star.

Colour default (display-encoded blend) is the **correct** AE default — keep that opt-in linear model.

### Migration proposal (do not big-bang delete legacy)

1. **Version the film** (or engine `compat` generation): `engine: 2` / `"compat": "ae"` means `effect_space: layer`, `quality: exact` for blur-family, `motion_blur` semantics only.
2. **SDK / catalogue defaults** for *new* comps write AE-correct keys explicitly so JSON is self-describing even on old engines.
3. **CLI**: `comp check` warns on `compat: legacy` or missing `effect_space` when any blur/glow present (“screen-space blur will not match AE”).
4. Keep `quality: legacy` and `effect_space: screen` as named escapes for bit-stable archival films; stop making them the undocumented zero.
5. Identity suite: split **legacy-identity** (archive) vs **ae-corpus** (new gold). Stop spending WP gate budget only on the former.
6. Timeline: after first 12 AE cases (research §5), flip SDK default; after library migrated, flip engine default with compat pin. Cost **M** engineering + **L** content migration.

---

## 5. Verification without AE ground truth

What works today:

- Closed forms (blend 567 pairs/mode, blur erf edges, grain stats, shutter smear centroids).
- Seek=play for history (by construction + tests).
- Determinism hashes on probes.
- `check` / `inspect` / assertions / sheets.
- `refstudy diff` against reference *videos* (templates) — not AE.

What this cannot catch (research is right):

- Blurriness↔sigma (0.5 vs Skottie 0.3).
- AE GPU vs CPU noise floor.
- Glow / fractal noise / wiggle / dither algorithms.
- Nested mblur, adaptive sampling.
- Expression `valueAtTime` semantics.

**Without AE, the honest ceiling is “internally consistent renderer,” not “AE parity.”** Metamorphic tests (research §5: identity-at-neutral, time-shift, resolution independence, precomp equivalence) should be added **now** (cost **S–M**) as they need no AE — but they do not replace the corpus.

Owner has AE. **Top recommendation unchanged from research: build the corpus and ledger gate first.**

---

## 6. Platform risks (macOS / Metal / SkSL)

| Risk | Detail | Mitigation |
| --- | --- | --- |
| macOS/Metal only | `metal` + skia metal feature; BUILD_PLAN P13 accepts this | Keep; do not port (research) |
| skia-safe 0.87 pin | Graphite/Ganesh drift; SkSL limits (no compute, bounded loops) | Pin; golden on upgrade; class D for histogram/FFT |
| Float / libm | Cross-OS-release hashes fragile | Prefer tolerances; software `libx264` for stable mux (research §7) |
| VideoToolbox | Non-reproducible | Already opt-in; document |
| SkSL-only effects | No CPU oracle for most shaders | Add reference/oracle only where corpus fails (research) |
| Half-float | Fine for 8/10-bit; coarse vs AE 16-bpc near white | Optional f32 per pass later |

**Do not** start wgpu/Vello/Slang before corpus proves need. **Agree with research.**

---

## 7. Process cost of the WP gate

Gate (`BUILD_PLAN.md` §8): full release workspace tests + docs write + capability index + parity check + probe checks + dual-render shasums + probe screenshots + (in practice) full identity snap vs previous binary.

Measured process artefacts:

- Test suite ~**923** release tests (and growing ~40+/WP).
- Identity: **64** files across WPs — each WP re-renders large example sets twice.
- Reports: **1.2k+ lines** of prose per few WPs; WP-01 alone 136 lines, WP-02 108, WP-13 150.
- Serial rule: one WP at a time; stop after REPORT.

**What the gate buys:** real discipline, no silent golden churn, byte-stable opt-ins. **What it costs:** agent-days per WP; incentives to mark `done` from internal tests; **identity work optimises for preserving incorrect defaults**; blocks parallel effect work.

**Reform (keep spirit, cut waste):**

1. Add **measurement gate**: `done` requires `measured` or `waived` (research) — this is the missing tooth.
2. Make identity **opt-in / sampled** for WPs that cannot change default pixels; full identity only when touching default paths.
3. Split fast PR gate (`cargo test -p` affected + parity check) from nightly full workspace.
4. Cap REPORT length; put numbers in `probes/data.tsv` (WP-02 already does this well).
5. Allow parallel WPs that do not share files once measurement WP lands.

---

## Engagement with `research.md` recommendations

| Research recommendation | This audit |
| --- | --- |
| Do not rebuild architecture | **Confirm** |
| AE ground-truth corpus now | **Confirm — highest priority** |
| Ledger `measured` / tol / refuse unmeasured `done` | **Confirm**; call out Gaussian Blur `done`+`UNSURE` as exhibit A |
| Keep Skia/SkSL; no OpenFX | **Confirm** |
| Keep QuickJS for expressions; host lib + dependency log | **Confirm**; note today QuickJS is **authoring only**, properties use `anim/expr.rs` |
| Content-hash cache | **Confirm — do before more history/temporal effects** |
| Schema single source of truth | **Confirm — do before WP-12 surface area** |
| Classic 3D CPU plan; skip Advanced 3D / Roto / etc. | **Confirm** |
| Stop counting effects; stop “not pixel-matching” goal | **Confirm emphatically** |
| `@roi/@identity/@frames_needed/@ae` | **Confirm**; schedule with match-name column |
| Defer ROI tiling / wgpu | **Confirm** |

Disagreement with plan (shared with research): change goal sentence from “not pixel-matching” to “within declared tolerance of AE on a defined corpus.”

---

## Recommendation

### Keep

- Crate split core ← comp ← script ← cli; pure frame; frame graph; strict serde; Skia Metal; SkSL header plugins; blend table; colour opt-in linear; property model v2 mechanism; layer-space *implementation*; history-as-pure-recompute; agent loop check/inspect/sheet; parity TSV *structure*; WP culture of tests+REPORT.

### Refactor NOW (before more features)

| Item | Size | Why now |
| --- | --- | --- |
| AE corpus tooling + first 12 cases + ledger columns/gate | **S–M** | Unblocks honest `done`; may force blur/blend rework |
| Flip default strategy (compat version; SDK writes layer+exact) | **M** | Stops digging the incorrect-by-default hole |
| Schema generate (schemars → JSON Schema → TS) for film types | **M** | WP-11/12 and agents will explode surface area |
| Content-hash layer/pass cache | **M** | Throughput + determinism key; eval already pure |
| Plugin header: `@ae`, `@identity`, `@frames_needed` (roi default = full) | **M** | Cheap with match-name column |
| Metamorphic tests (neutral identity, time-shift, seek=play already partly there) | **S** | Free bugs, no AE |
| Split files >300 in eval/schema/fxlib when next touching them | **S** ongoing | Rule is already violated |

### Stop

- Marking effects `done` from self-tests alone.
- New blur-adjacent effects that default to `legacy`.
- Breadth sprint through `effects.tsv` todo list before Skottie-tier ~30 are `within_tol` vs AE.
- Renderer replacement talk (wgpu/Vello/Slang).
- OpenFX, OCIO/ACES, Roto, Warp Stabilizer, Advanced 3D, Cinema 4D.
- Silent adoption of third-party numerics without `provisional` + remeasure.
- Spending full identity gates on WPs that only add opt-in paths (sample instead).
- Starting WP-20 shading before camera/light **numeric** corpus cases.

### Order of remaining WPs (given findings)

Re-sequence relative to BUILD_PLAN §0 list:

0. **REVIEW follow-through / “WP-AE”** (new): corpus + ledger gate + goal sentence — **before** WP-11/12/20/30 breadth.  
1. **Defaults migration** (compat / SDK) — can overlap tooling.  
2. **Schema generation** (foundation).  
3. **Content-hash cache** (foundation).  
4. Finish blur-family honesty: measure Gaussian; then decide legacy deprecation — before more T0 effects.  
5. **WP-14** (mask order, collapse, switches) — semantics before 3D.  
6. **WP-11** then **WP-12** (expressions) — only with host-lib design and deterministic expression corpus.  
7. **WP-30** remainder (shape ops) — template-driven, not encyclopedia.  
8. **WP-20 → 22 → 23 → 24** (mesh/3D) — camera corpus first; contiguous groups.  
9. Effect porting loop **restricted** to Skottie-tier + template-needed, each with AE case.  
10. WP-33/36 particles, tracking imports — after mattes/switches.

Defer indefinitely: Advanced 3D, third-party effect clones as “parity,” Linux port, OpenFX.

---

## Top risks (cost-to-fix-later), condensed

1. First AE measurements invalidate “done” blur/glow/grain → rework.  
2. Incorrect defaults ossify in motions/templates.  
3. Schema/SDK drift under agent volume.  
4. Expression dual-stack forever.  
5. 3D templates blocked on fake mesh renderer.  
6. macOS/Skia lock + SkSL ceiling for class D effects.  
7. WP gate rewards identity-to-legacy over AE truth.  
8. Undocumented AE algorithms force `approx` forever (acceptable if labelled).

---

## Appendix: file:line anchors (quick index)

- Architecture contract: `docs/ARCHITECTURE.md:12–96`  
- Goal anti-pixel-match: `docs/BUILD_PLAN.md:69–71`  
- Effect space default screen: `crates/comp/src/catalogue/effect_space.rs:10–32`  
- Header tag allow-list: `crates/comp/src/fxlib/header.rs:89`  
- History cache 512 MiB: `crates/comp/src/render/history.rs:31–70`  
- Gaussian UNSURE: `parity/specs/gaussian_blur.md:56–61`  
- Mblur nesting deviation: `parity/specs/motion_blur.md:23–25`  
- Mesh painter’s algorithm: `crates/comp/src/mesh/draw.rs:1–2`  
- AE test ignored: `crates/comp/tests/ae_rotation.rs:16`  
- SDK drift = key names: `crates/script/tests/sdk_option_types.rs:1–39`  
- Blend table size: `crates/core/src/effects/blend/table.rs:79`  
- Property third-party measure: `parity/specs/property_model.md:29`
