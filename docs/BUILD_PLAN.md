# turbomotion build plan: every step to make it create what After Effects can create

This is the **only** planning document. `docs/ARCHITECTURE.md` is the contract (where code lives); `docs/reference/` is
generated. Everything else about what to build, in what order, with which files and tests, is here.
Written 2026-10-26 against engine commit `ab256b6` (clean tree) and `motions/` at `4ba1f71`.

Audience: an implementing agent that follows steps literally. If a step cannot be done as written, do not improvise:
write `BLOCKED.md` (section 1, rule 9) and stop.

Claim tags: **[V]** verified in code or docs, **[P]** verified by a triage probe, **[?]** unverified: confirm before
building on it and correct this file in the same change.

---

## 0. First tasks (do exactly these, in this order)

0. **Make `cargo test --release --workspace` green.** On 2026-10-26 it is red: 423 pass, 3 fail, all in
   `crates/comp/tests/examples_check.rs` (30 of 188 catalogue examples and 28 of 115 reference-page examples fail `check`,
   mostly `low-contrast` errors on dark text over dark ground in the `extrude` and 3D examples, plus missing-camera warnings;
   and `an_effect_example_that_forgets_its_declared_layer_input_fails_instead_of_being_helped`). Fix the **examples** (give
   them readable colours, a camera, a larger image), not the lint rules. Regenerate docs. Nothing else starts while this is red.
1. Fonts (WP-52). 2. Parity ledger and `parity` command (WP-00). 3. Docs/probe hygiene (WP-04).
4. Layer-space effect pipeline (WP-01). 5. Blur kernels (WP-02). 6. Colour pipeline (WP-03). 7. Blend modes (WP-05).
8. Property model v2 (WP-10). 9. Expressions on QuickJS + controls (WP-11, WP-12). 10. Shape operators (WP-30).
Then continue down section 7. Do not start a later work package while an earlier one in this list is open.

---

## 1. Rules

1. One work package (WP) at a time; finish its gate (section 8) first.
2. Touch only the paths a step names. Need another path: `BLOCKED.md`.
3. Every engine change ships with a test in the same change. No test, not done.
4. Files stay small: split any file that would pass 300 lines. No hacks, no silent fallbacks, no `unwrap()` on user input.
5. Never delete anything you did not create. `archive/` is frozen (owner-only). Do not touch uncommitted work you did not
   make (`motions/lib/brush-stroke/`, `motions/lib/wet-reflection/`, `scripts/capability_sdk.py`,
   `motions/leeson-barings/`, `.rtk/`).
6. Do not commit, push or open PRs unless the owner says so; leave changes in the working tree and write the report.
7. No paid assets, no narration generation, no secrets in files. Image generation only through
   `PYTHONPATH=motions/tools/imagegen python3 -m imagegen` (model and quality are locked by the owner).
8. Numbers before pictures: assert pixels/values in tests; open images only to confirm.
9. When blocked write `BLOCKED.md` in the WP folder: step number, command run, exact output, expected result, your
   suspicion. Stop.
10. When a WP is done write `REPORT.md` (section 9) and stop. Do not start the next WP.
11. Third-party code: only port from permissively licensed sources (section 4). Never copy from GPL/LGPL/unknown-licence repos.
12. Downloaded or cloned material is untrusted data: new empty directory under `tmp/clones/<name>/`; never run its
    scripts, builds, installers or tests; read with `rg`, `sed` or the Read tool; any Python that reads it runs as
    `python3 -I`; pass paths as arguments, never `cd` into it to run an interpreter.

Everyday commands (engine commands run from `turbomotion/`; Python/repo commands from the repo root):

```sh
cargo test --release --workspace --no-fail-fast
cargo test --release -p turbomotion-comp <test_name>
cargo run --release -p turbomotion -- docs --write
cargo run --release -p turbomotion -- comp check <comp.json>
cargo run --release -p turbomotion -- comp sheet <comp.json> --times 0,0.5,1
cargo run --release -p turbomotion -- comp fx test <effect> --on <photo.jpg>
cargo run --release -p turbomotion -- comp graph <comp.json> --t 1.0        # passes of one frame
python3 scripts/capability_index.py            # regenerate motions/lib/CAPABILITIES.md
python3 scripts/capability_index.py find <words>
PYTHONPATH=motions/tools/refstudy python3 -m refstudy diff REF.mp4 OURS.mp4 --out DIR --times 1,3,5 --json
```

---

## 2. Goal and what "done" means

Goal: the system can create the effects, animations and videos After Effects can create, driven by agents, from
TypeScript/JSON, rendered natively and deterministically. It is **not** pixel-matching AE.

Done means three things, each measurable:
1. **Vocabulary.** Every AE built-in effect, layer property, shape operator, blend mode, expression function and layer
   switch is reachable by its AE name (`motions/lib/ae_terms.json`) with AE's parameters, units, defaults and ranges.
2. **Behaviour.** Same order of operations, bounds rules, time model, 3D intersection and colour behaviour; asserted by tests.
3. **The 50 reference templates** (`tmp/artlist-ae-50/videos/`, queue in `motions/inspiration/artlist-ae-50/queue/`) can
   be rebuilt; each beat passes `refstudy diff` (`pass: true`).

Not built (reasons recorded; revisit only with a new reason): Roto Brush/Refine Edge, Warp Stabilizer, 3D camera solver,
Content-Aware Fill, Cinema 4D renderer, Mocha, `.aep` import, Immersive/VR effects, third-party plugin clones
(Particular, Form, Shine, Sapphire: their jobs are covered by WP-33/36).

---

## 3. State of the system and the honest assessment

### 3.1 What exists and is good (keep) [V]
Crates `core` (GPU, encoder, effects registry, colour, easing), `comp` (schema, plan, eval, render, text, mesh, fxlib,
check), `script` (TS strip + QuickJS + SDK), `cli`. A frame is a pure function of time. Strict schema with did-you-mean.
Frame graph (`render/graph`: `Stage::{Backdrop,Paint,Effects,MaskCoverage,MatteCoverage,Multiply,Reflect,Wipe,
ShutterAverage,Composite,Snapshot,AdjustCoverage,AdjustMix}`). Plugins are one `.sksl` plus header comments
(`@effect @kind @summary @param @input @pass @example`; `fxlib/header.rs`). ~100 effects (32 shipped as files in
`motions/lib/effects/`, the rest in `crates/comp/src/fxlib/builtin/` and `core/src/effects/`). 56 transitions. Particles
(closed form), text animators, 3D camera with per-layer DoF, lights, repeat layer key, text `selector.wiggle`, skew tween,
particle path sprites. `check`, `inspect`, assertions, `sheet`. Speed 90-150 fps for 2D.

### 3.2 What is wrong, ranked by how much it blocks the goal

**P1. The mesh renderer is not a renderer [V].** `mesh/draw.rs`: CPU painter's algorithm; `mesh/shade.rs`: per-vertex
Blinn-Phong with a hard-coded sky; Skia draws one Gouraud triangle batch. No depth buffer, no per-pixel shading, no UVs, no
textures, materials are four floats (`schema::Material {ambient, diffuse, specular, shininess}`). So chrome, glass,
fresnel, inflated bevel, tube, refraction cannot be added as parameters: they need a per-pixel pipeline (WP-20, WP-21).
Self-overlap artefacts at bevel >= 14 [P].

**P2. 3D layers do not intersect [V/P].** The graph composites layer by layer; there is no depth. Hinged posters, cube
corners, planes crossing a floor are wrong (templates 003, 004, 018). WP-22.

**P3. Properties are scalar [V].** `anim/prop.rs`: keys are `(t, f64, ease)`. A position is three scalars; no spatial
bezier, no roving, no speed/influence handles, no auto-orient. WP-10.

**P4. Expressions return one number [V].** `anim/expr.rs` `Env::call -> Option<Result<f64>>`; only `ref`, `wiggle`,
`linear`, `amp`, `onset`, `beat`. No arrays, strings, `valueAtTime`, `toComp`, `lookAt`, `sampleImage`, markers, effect
access; no expression controls. WP-11, WP-12.

**P5. Effects run in screen space on comp-sized images, not in layer space [V].** `render/post.rs::paint_stage` renders every
layer into an image of `frame.width x frame.height` and `render/content.rs::paint_content` applies the layer transform
(`c.concat_44(&st.screen.to_m44())`) *while painting*; effects, masks, mattes then run on that already-transformed picture.
After Effects does the opposite: source -> masks -> effects (in layer space, in layer pixels) -> transform. Consequences:
an effect's pixel parameters do not scale or rotate with the layer (a 10 px blur on a layer scaled to 200% is 10 screen px,
AE gives 20); content that is off-screen is gone before a blur/glow/displacement can use it; nothing can grow past the comp
edge; every layer costs a full-frame half-float buffer (16 MB at 1080p; template 001 has 109 layers); a directional effect on a
rotated layer is in screen space. The earlier probe symptom ("`glitch_slices` clipped glyphs to the layer bounds") is a
symptom of this (verify the exact mechanism in WP-01 step 1). Also `tilt_shift.sksl` blurs with 16 spiral taps [V]. WP-01, WP-02.

**P6. No shape-operator stack [V].** No Merge Paths, Offset Paths, Pucker & Bloat, Round Corners, Twist, Zig Zag, Wiggle
Paths, Wiggle Transform, gradient stroke, taper/wave. WP-30.

**P7. Blend modes 17 of ~38 [V]** (`core/src/effects/blend.rs`; mapped 1:1 to Skia `BlendMode`). WP-05.

**P8. Colour pipeline unspecified [?].** Layers are half-float premultiplied [V] but there is no working space,
no linear-light option, no profile handling. WP-03.

**P9. Order of operations differs from AE [V].** `render/graph/build.rs::layer_picture` runs Paint (transform already applied)
-> effects -> masks -> clip -> matte -> reflect -> wipe. AE: masks, then effects, then transform. WP-01 fixes the space; WP-14 adds the
opt-in mask-before-effects order.

**P10. Text [V].** No measured bounds (the whole text-bar/pill/chip family is blocked), selector has no "based on"
(chars/words/lines), no multi-selector modes, no auto-fit box, limited paragraph features. WP-32.

**P11. Time [V].** No Posterize Time, Time Displacement, Time Difference, frame interpolation, per-layer shutter. WP-13.

**P12. Output [V].** H.264 and ProRes 4444 only; no image sequence, EXR, GIF/WebP, H.265, VP9. WP-50.

**P13. macOS/Metal only [V].** Not a parity item; do not promise headless Linux rendering.

### 3.3 Process problems (fix, do not repeat)
- No beat has passed `refstudy diff`; only template 001's first 7.6 s of ~34 s exists as film (ready, versus built;
  remaining beats spikes, type-tunnel, black-friday, afterparty, grid-tunnel, reprise, outro) [V].
- 31 triage records (002-032) exist; **86% of "ready" verdicts (163 of 189) rest on catalogue text, not a probe** [V].
  Catalogue text misled agents repeatedly (echo is a soft trail, `torn_edge` is a noise dissolve, `ink_bleed` is a
  newsprint smear, mesh `color` cannot be a gradient). Treat every unprobed `ready` as unknown.
- Ranking by "beats blocked" hid cross-cutting causes: "mesh, 25 beats" is really P1 + P2 + P6.
- The triage records that remain valid are the beat lists and acceptance texts; the `gap` classification is stale for
  items already landed: `selector.wiggle`, skew tween, `repeat` layer key, particle path sprite, camera `dof`.
- Do not triage the last 19 templates (033-050) until WP-01..05 and WP-30..32 land. Use `parity/MATRIX.tsv` ids in
  new records instead of prose.

### 3.4 Verified state on 2026-10-26 (before any work package)
- Engine: `cargo test --release --workspace`: 423 passed, 3 failed, 2 ignored (the 3 failures are example-lint, section 0). Python
  tests (`scripts/test_*.py`): 35 pass. `artlist_triage.py check`: 31 records ok. Binary `turbomotion/target/release/turbomotion` exists.
- Template 001 rebuild (`rebuilds/001-esports-momentum/film.ts`): `sdk check` passes; 109 layers, 3 shots, 7.6 s, renders at ~122 fps.
  `refstudy diff` against the reference at 1, 3, 5, 6.5 s reports `pass: true` with 1 of 4 rows WARN (limit 25%). **That gate is weak:**
  `refstudy/diff.py` compares global scalars (colour distance, edge density, motion energy, palette overlap) per sampled frame, not
  structure, so a wrong layout with similar statistics passes. The queue still shows `0 verified` because no beat was submitted.
  WP-04 step 6 strengthens the gate before anyone trusts `pass: true`.
- Nothing in section 7 has been built yet. `turbomotion/parity/` does not exist; `motions/lib/fonts/` has 3 families (Manrope,
  Instrument Serif, JetBrains Mono); the SDK has no `texture()` helper; `loop` has `cycle|pingpong|offset` only (no `continue`);
  the text `Selector` has `start,end,offset,shape,smooth,random,seed,wiggle` only (no based-on, no modes); there is no colour-space code
  (only an sRGB colour struct); blend modes are the 17 in `core/src/effects/blend.rs`.
- Code health: 20 non-test files exceed the 300-line rule (largest: `eval/mod.rs` 783, `fxlib/mod.rs` 716, `schema/tokens.rs` 644,
  `schema/text.rs` 607, `schema/kinds.rs` 584, `cli.rs` 565, `anim/prop.rs` 531). Split a file when a WP touches it; do not do a
  repo-wide split as its own task.
- Uncommitted state: engine repo has staged deletions and edits (docs cleanup, `README.md`, `ARCHITECTURE.md`, a comment in
  `core/src/layer.rs`, new `docs/BUILD_PLAN.md`); main repo has the owner's in-flight work (rule 5) plus staged deletions. Nothing is committed.

---

## 4. Reference sources and the porting procedure

Clone into a new empty directory each: `mkdir -p tmp/clones && cd tmp/clones && git clone --depth 1 <url> <name>`.
`tmp/clones/effectcraft` already exists (commit `2961258`). Never build or run them.

| Name | URL | Licence | May copy | Use |
| --- | --- | --- | --- | --- |
| EffectCraft | github.com/storytold/effectcraft | MIT or Apache-2.0 | Yes, with notice | Pure-Rust AE clone (306 effects, keyframes, shape ops, layer styles, expressions, 3D, Lottie). Alpha, young, fidelity vs AE barely measured: treat its behaviour as a hypothesis |
| Skia (Skottie, SkSL) | github.com/google/skia | BSD-3 | Yes | AE-model Lottie renderer; SkSL idioms |
| ThorVG | github.com/thorvg/thorvg | MIT | Yes | Trim paths, stroke, mattes, gradients |
| lottie-web | github.com/airbnb/lottie-web | MIT | Yes | AE model on the web: shape ops, text animators, expression subset |
| three.js | github.com/mrdoob/three.js | MIT | Yes | PBR chunks (`src/renderers/shaders/ShaderChunk/`), env maps, shadows, Tube/Lathe/Text geometry |
| gl-transitions | github.com/gl-transitions/gl-transitions | per file | check each header | ~100 GLSL transitions to port to SkSL |
| pixi filters | github.com/pixijs/filters | MIT | Yes | Reference GLSL for glitch, bloom, godray, crt |
| OpenColorIO, OpenFX | github.com/AcademySoftwareFoundation/... | BSD-3 | Yes | Colour transforms, parameter vocabulary |
| Natron, openfx-misc, Friction, Synfig, Glaxnimate, Olive, Kdenlive, Blender | various | GPL/LGPL | **No** (read only) | Behaviour ideas only |
| Remotion | github.com/remotion-dev/remotion | custom, not OSI | No | Idea of deterministic seek only |

**P-LIC (once, before copying anything).** Write `docs/THIRD_PARTY_NOTICES.md`: one line per source you copy from
(name, licence, URL, commit). For each file to copy: read its header; run `rg -n "Natron|Friction|Synfig|GPL|copyright"
<file>`; any hit means do not copy that file. Put `// ported from <repo>@<hash> <path> (<licence>)` above each ported function.
EffectCraft claims clean-room origin; that is unverified, so apply this check to every file.

**EffectCraft map (paths under `tmp/clones/effectcraft/crates/`) -> target here (under `turbomotion/crates/`)**

| Concept | Read | Target |
| --- | --- | --- |
| Keyframes (temporal+spatial bezier, roving, hold); AE-measured ease samples | `keyframe/src/{lib,value}.rs`, `../docs/fidelity/keyframe-ease.json` | `comp/src/anim/prop/*` |
| Blend modes | `color/src/blend.rs` | `core/src/effects/blend.rs` + shader blend |
| Colour spaces, ICC | `color/src/{space,icc}.rs` | `core/src/color.rs` |
| Path ops, offset, zigzag, twist, wiggle | `path/src/{ops,boolean,offset}.rs` | `comp/src/shape/ops/*` |
| Variable-width stroke, taper, feather | `path/src/{varstroke,feather}.rs` | `comp/src/shape/stroke/*`, `render/masks.rs` |
| Layer styles | `render/src/styles.rs` | `core/src/effects/styles/*.sksl` |
| Text selectors, layout, variable fonts | `text/src/{selectors,layout,variable,path_text}.rs` | `comp/src/text/*` |
| Expressions (AE object model in JS) | `expr/src/{host,runtime,rewrite,noise,sample}.rs` | `comp/src/anim/expr/*`, `anim/expr_prelude.js` |
| 3D camera, DoF, bokeh | `render/src/three_d/camera.rs`, `gpu/src/bokeh.rs` | `comp/src/eval/camera.rs` |
| 3D intersection and compose | `render/src/three_d/compose.rs` | `comp/src/render/graph/depth3d.rs` |
| Advanced 3D, PBR, env | `render/src/three_d/adv/*`, `model/src/*`, `gpu/src/{adv3d,deferred}.rs` | `comp/src/mesh/*` |
| Effects (CPU) and GPU versions (WGSL ~ SkSL) | `effects/src/*.rs`, `gpu/src/fx_*.rs`, `../docs/effects.md` | `comp/src/fxlib/builtin/*.sksl` |
| Time effects, optical flow | `gpu/src/fx_time.rs`, `raster/src/flow.rs` | WP-13 |
| Keying | `effects/src/{keylight,keying,keying2,matte}.rs` | WP-35 |
| Warp, puppet, liquify | `raster/src/warp.rs`, `effects/src/distort*.rs` | WP-34 |
| Particles, simulation | `gpu/src/{particles,fx_particles,fx_sim}.rs` | WP-36 |
| Tracking | `track/src/*` | WP-41 |
| Lottie import/export | `lottie/src/*` | `comp/src/tools/lottie/*` |

**The porting procedure (use for every feature `F`: effect, style, operator, blend mode...)**
1. Open/add the row for `F` in `parity/MATRIX.tsv` (or `parity/effects.tsv`); status `in_progress`.
2. Write `parity/specs/F.md` in your own words: AE name; each parameter with unit, default, range (from
   `tmp/clones/effectcraft/docs/effects.md` and AE documentation; where sources disagree write both and mark `UNSURE`);
   formula in pseudo-code; edge cases; grows bounds? (yes/no, how much); linear light? (yes/no).
3. Write the tests first (section 5.3). Run them; they must fail for the right reason.
4. Implement (section 5 for effects; the WP for other features). Port from a permissive reference where it helps.
5. Contact sheet / probe image; open it.
6. `turbomotion docs --write`; `python3 scripts/capability_index.py`; add the AE term to `motions/lib/ae_terms.json`; add
   one runnable example that `check` passes.
7. Gate (section 8); only then set the row `done`.

---

## 5. How to add things (the mechanics every WP uses)

### 5.1 A shader effect (no Rust)
1. File: `crates/comp/src/fxlib/builtin/<name>.sksl` (shipped; `build.rs` picks it up). Reusable project effects instead go in
   `motions/lib/effects/<name>.sksl` (used via `fx_path`).
2. Header comment lines, first lines of the file, exactly: `// @effect <name>`, optional `// @kind generator`,
   `// @summary <one sentence>`, then `// @param <name> <float|vec2|color|choice ...> <default> [min max] -- <doc>`,
   `// @input <name> layer|history=N depth=D`, `// @pass <name> scale=S`, `// @example {json}`. Copy the layout of
   `fxlib/builtin/tilt_shift.sksl`. Unknown tags are errors. Names `image`, `resolution`, `time`, and the composite key
   are reserved. Parameter types today: `float`, `vec2`, `color`, `choice` (`fxlib::Ty`); WP-01 adds more.
3. Body: `half4 main(float2 xy)`; use the prelude (`straight(xy)`, `premul(rgb, a)`, hash, noise, fbm, hsv, luma, blends;
   see `fxlib/prelude.sksl`).
4. Test in `render/fx_tests.rs` using `render/testkit.rs` (`render(json) -> BGRA bytes`, `render_at`, `bgra`).
5. `turbomotion comp fx test <name> --on photo.jpg`; open the sheet.

### 5.2 A platform (Rust) effect
Declare it once in the `registry!` in `crates/core/src/effects/mod.rs` (name, kind, parameters with default/range/doc);
`Effect::build` checks uses against that declaration; implement under `core/src/effects/<area>.rs`. Docs are generated from
the same lines. Use this only when a plugin cannot do it (separable kernels, analysis, multi-image).

### 5.3 Minimum tests for an effect
(a) identity: neutral parameters return the input byte for byte; (b) one numeric behaviour (for example "a 4x4 white square
blurred with radius 10 has non-zero alpha 14 px from its centre"); (c) determinism: two renders equal; (d) bounds:
`grows_bounds` matches the ledger row; (e) time-dependent effects: seek equals play.

### 5.4 A layer key / schema field
`crates/comp/src/schema/*` (strict serde types, `deny_unknown_fields`) -> `plan/` (resolve) -> `eval/` (numbers at time t)
-> `render/` (pixels) -> `catalogue.rs` example -> SDK type in `crates/script/sdk/` -> `docs --write` -> test per stage
(schema errors are strings, plan tables, eval numbers, render pixels).

---

## 6. Architecture decisions (binding for the WPs that cite them)

**ADR-1 Effect model v2** (runs in layer space after WP-01). Header gains: `@bounds expand=<expr>` (pixels each side, may reference params), or `expand_x/expand_y`,
or `source=<input>`; `@space linear|display`; `@cost cheap|medium|heavy`. New parameter types: `point`, `angle`, `checkbox`,
`curve` (256-entry LUT from keys), `gradient` (N stops), `path` (mask/shape reference), `point3d`. First-class separable kernel
pass `@pass name kernel=gaussian|box|disc axis=x|y radius=<expr>` implemented in `core` (correct sigma, mip pyramid above
radius 32). Effects stay animatable by keys or expressions.

**ADR-2 Per-pixel mesh pipeline.** Deterministic CPU depth-buffered rasteriser (rayon over tiles, f32, fixed tile order,
4x4 coverage AA) writes a G-buffer (depth, world position, normal, UV, material id, part id) to float images; then a
**material shader** (SkSL, `fxlib/materials/*.sksl`, same header grammar, `kind=material`) shades from the G-buffer, lights,
an environment image and the backdrop. Keeps "new pixels are shaders", stays deterministic, gives a real depth buffer. Only
`mesh/draw.rs` and `mesh/shade.rs` are replaced; `mesh/{tessellate,primitives,glyphs}.rs` stay. Budget: 1080p, 200k triangles,
under 60 ms/frame; decide at the end of WP-20 with the measured number (fallback in section 11).

**ADR-3 3D group depth compositing.** All contiguous `three_d` layers form one group composited by `Stage::Depth3D` with a
per-pixel depth test; transparent fragments ordered per pixel (K=8, overflow warns `depth-overflow`). Flat layers yield depth
analytically from their plane; meshes from the G-buffer. 2D layers composite by layer order (AE rule).

**ADR-4 Property model v2.** Typed properties `scalar|vec2|vec3|colour|path|text|bool`; keys hold typed values with per-key
interpolation `hold|linear|bezier(temporal speed+influence in/out)`; position adds spatial tangents and `roving`;
`separate_dimensions`. All existing JSON forms stay valid; new key form:
`{"keys":[{"t":0,"v":[0,0],"out":[100,0],"in":[0,0],"ease_out":{"speed":0,"influence":33}}],"spatial":"bezier","auto_orient":"path"}`.

**ADR-5 Colour pipeline.** Project `color: {depth:"half", space:"srgb", linear_light:false, working_profile:"srgb"}`; default
reproduces today byte for byte (whole golden suite must stay identical). With `linear_light:true`, blends/blur/glow/gradients
operate in linear light; conversion at import and encode; profiles read from images/ffprobe.

**ADR-6 Parity ledger.** `parity/MATRIX.tsv` (feature, category, status, test, doc, ae_ref, wp), `parity/effects.tsv` (name,
category, recipe_class, tier, status, grows_bounds, linear, ae_params), `parity/specs/`, `parity/probes/`, `parity/golden/`.
`effects.tsv` also carries `engine` (catalogue names that implement the row, comma separated) and `test` (the test function that proves a `done` row; `check` greps for `fn <test>(` under `crates/` and `def <test>(` under `../scripts/`). Core rows name `test` and `doc` (a path relative to the engine root, `../` allowed); effect rows need `parity/specs/<slug>.md` and `parity/probes/<slug>.json`. Per-WP reports live in `wp/<WP>/REPORT.md`. `turbomotion parity check` fails when a `done` row lacks test/golden/doc, an effect marked `done` is not in the catalogue, or a
catalogue entry has no row. `parity report` prints coverage per category.

---

## 7. Work packages: the steps

Sizes: S under 1 day, M 1-3 days, L 1-2 weeks, XL longer (one agent). Every WP ends with the gate (section 8).

### Phase 0: agent velocity (done before any template agent runs; owner request 2026-10-26)
Evidence: seven clone builders used 96-115 tool results and 150-250 KB of tool output each, with zero code written. Causes: a 150 KB reading
list, images re-read on every call, six throwaway measuring scripts re-implemented per agent, wrong-python (`-I`) and `fc-list` retries, usage dumps.
1. `scripts/rs` wrapper (refstudy, tm, it = check + render sheet) and `motions/lib/fonts/INSTALLED_FAMILIES.txt`. [done]
2. One tool, `refstudy clone SRC` (analyze: beats, element tracks, fitted eases, finish; scaffold: a runnable film.ts; check: render and numeric compare, 5 worst items), plus `sample`/`series`, quiet `detail`, one-line usage errors. No OCR (motion is the point). Spec: `tmp/research/refstudy-v2/SPEC.md`. [in progress]
3. One document, `motions/GUIDE.md`, replaces every brief, cheat sheet and prompt (tiers FAST/STANDARD/RIGOROUS); the two skills are one-line pointers to it. [done]
4. Engine ergonomics, still open: SDK errors that fail early with the valid names (argument order, `fx` after construction, `fx_path` after `comp()`);
   a `texture()` helper for stand-in plates; `sdk inspect` JSON mode; WP-01 layer-space effects (blur scales with layer scale).
5. WP-02 shrinks: Skia's blur is already used (`core/src/layer.rs:66`); only radius-to-sigma documentation and tests remain.
6. Ordering: template-first. Rebuild templates with existing vocabulary (86% of beats), and take engine WPs only when a beat is blocked (13%: mesh, text bounds, palette cycle).
Pilot gate: one 6 s clip (reel 04) under FAST; record tool calls, output KB and images, and compare with the baseline above before launching more.

### Phase A: ratchet, assets, hygiene

#### WP-52 Fonts (S; owner approved any open licence from any source)
1. `mkdir -p tmp/fonts-incoming`. For each family below: `mkdir tmp/fonts-incoming/<Family>`; fetch via sparse clone of
   github.com/google/fonts (`git clone --depth 1 --filter=blob:none --sparse <url> <empty dir>`, then
   `git sparse-checkout set ofl/<family>`) or fontsource. One family per directory.
2. Read each licence file. Accept only OFL-1.1, Apache-2.0, Ubuntu Font Licence. Anything else: record `rejected`.
3. Write `scripts/font_info.py` (reads a path argument, prints family, weights, variable axes, glyph count, whether U+2588 and
   U+00A0 exist, via `fontTools`; run as `python3 -I scripts/font_info.py <file>`). Do not run anything inside download folders.
4. Copy files and licence to `motions/lib/fonts/<Family>/`; append to `motions/lib/fonts/fonts.json`
   `{family, files, weights, axes, tags, has_block_elements, licence, source_url, sha256}`.
5. Render `motions/lib/fonts/SPECIMEN.png` (one line per family via `comp sheet`); `check` must find each family by name
   without a path. If Skia cannot load `.woff2`, convert with `fontTools` first.
6. Make `scripts/capability_index.py` list the fonts and the `comp({fonts})` snippet.

Families: chunky rounded (Fredoka, Baloo 2, Lilita One, Titan One, Chango, Bowlby One); heavy extended (Archivo with width
axis, Rubik Mono One, Dela Gothic One, Unbounded); condensed (Bebas Neue, Anton, Oswald, League Gothic, Staatliches, Big
Shoulders Display); marker/hand (Permanent Marker, Rock Salt, Caveat Brush, Gloria Hallelujah, Sedgwick Ave Display);
pixel/digital (Press Start 2P, Silkscreen, VT323, DotGothic16, Pixelify Sans, DSEG7); classical serif (Cinzel, Cormorant,
Playfair Display, DM Serif Display, Abril Fatface); comic (Bangers, Luckiest Guy, Boogaloo); typewriter (Special Elite,
Courier Prime); display (Monoton, Bungee, Righteous, Rubik Glitch, Climate Crisis). JetBrains Mono (bundled) may already
cover U+2588: record it.

#### WP-00 Parity ledger (M)
1. Create `turbomotion/parity/{MATRIX.tsv,effects.tsv}` and dirs `specs/ probes/ golden/`.
2. Seed `MATRIX.tsv` from section 7a (one row per line) and `effects.tsv` from section 10 (one row per effect).
3. Add `crates/cli/src/parity/{mod,check,report,init_effect}.rs`; register `parity` in the CLI. `init-effect <name>`
   scaffolds the `.sksl` header, probe JSON, test stub and ledger row.
4. Make the catalogue print a `not_supported` list per feature from the ledger (answers the repeated friction item); show it
   in `docs/reference/*.md` and `CAPABILITIES.md`.
5. Tests: a fixture ledger with a `done` row missing its test fails `check`.

#### WP-04 Docs and probes as tests (M)
1. The harness already exists (`crates/comp/tests/examples_check.rs`); it is red (section 0, task 0). After it is green, extend it to the
   effect `@example` blocks of every plugin in `motions/lib/effects/` and to the TypeScript snippets in `docs/reference/sdk.md`.
2. `turbomotion sdk new probe NAME`: one-command probe film (comp with `fx_path`, `block_path`, a text layer, an effect, a
   sheet). Document `TURBOMOTION_BIN`, the `fx` key, `solid(color, opts)`, `texture(shader, opts)`.
3. SDK: throw on unknown layer fields, on `solid({color})` with the message "solid(color, opts)", on `layer.effects =`
   after construction; type `texture`/`raw` params and text `italic/outline`; error when `matte` source on an adjustment layer is
   hidden.
4. `scripts/capability_index.py find`: any-word match ranked by hits; `--fields <layer>`; tag generator-kind built-ins as
   `generator`; add `matte`, `group`, `texture` entries.
5. `refstudy strip`: default one contact sheet, `--cols`, `--crop`, clamp `--to`, `-nostdin` on every ffmpeg call, fix the
   `END_EPS` NameError; `shots.json` writes `soft_cuts` and a plate-change detector. (Python, `motions/tools/refstudy`; add a test
   that runs `strip` on a tiny clip.)
6. Strengthen `refstudy diff` (it is a gate): add per-region structural similarity (SSIM on a 8x8 grid), a text-box layout
   comparison (OCR-free: compare dark/light region bounding boxes), and require every sampled row to meet a floor, not only a
   25% warn budget; add tests with a deliberately wrong film that must FAIL.

### Phase B: correctness foundations

#### WP-01 Layer-space effect pipeline and bounds (XL; depends WP-00; the most important change in this plan)
Goal: effects, masks and mattes run in **layer space** on a buffer sized to the layer's content plus the effects' growth,
then the result is transformed and composited. This gives AE's order, makes pixel parameters scale/rotate with the layer, lets
effects grow past the comp edge, and stops every layer costing a full-frame buffer.
Read first: `render/post.rs::paint_stage`, `render/content.rs::paint_content`, `render/graph/{mod,build,run}.rs`, `render/fx.rs`,
`render/composite.rs`, `eval/mod.rs` (`LayerState.screen`, `bounds`, `project_bounds`), `fxlib/{header,mod}.rs`,
`core/src/effects/mod.rs`, `core/src/layer.rs`.
Rollout is opt-in: comp key `"effect_space": "layer" | "screen"` (default `"screen"` = today's behaviour, every existing golden
unchanged). Flip the default only with the owner's approval after reviewing `motions/` films.
1. Reproduce and explain the clipping symptom with a test: text + `glitch_slices` and `blur` on a 4x4 square, record the actual
   mechanism in `parity/specs/effect_space.md` (which buffer, which size, which `resolution` the shader sees).
2. Add `LayerState.local_bounds` (content rectangle in layer pixels, per content kind) next to `screen` in `eval/mod.rs`; for text use
   the measured box, shapes the path bounds, images/solids their size, precomps their comp size, particles a declared box.
3. New `Stage::PaintLocal {layer}`: render content with the **identity** transform into a buffer of `local_bounds` expanded by the
   chain's growth (step 5), origin recorded. `paint_content` gets a `transform: bool` argument; the screen path stays for
   `effect_space: screen`.
4. Run effects and masks on that buffer (`Stage::Effects`, `Stage::MaskCoverage` see layer-space buffers; `resolution` is the buffer
   size and a new `uniform float2 origin` is the buffer's offset inside the layer, so `xy` stays in layer pixels).
5. Growth: parse `@bounds` in `fxlib::Header::parse` (`expand=<expr>`, `expand_x=`, `expand_y=`, `source=<input>`; params allowed; unknown
   keys error); platform effects get `fn grows(&self, params, t) -> [f32; 4]` (blur, directional_blur, radial_blur, glow, shadow,
   outline, bloom, halation, echo, wave, ripple, shockwave, displacement_map, turbulent_displace, liquid, twirl, bulge, glitch_slices,
   datamosh, rgb_split, chromatic_aberration, prism, light_streak, anamorphic, god_rays, comet, heat, crumble, pixel_smear,
   roughen_edges, highlighter, scratches). The builder sums growth over the chain.
6. New `Stage::Place {layer}`: draw the layer-space result into a comp-sized transient (or directly into the composite) through the
   layer matrix, including the 4x4 perspective matrix for 3D layers (Skia `concat_44` already does this), with sampling per the
   layer (`linear`, mip for reductions). Mattes/clips/wipes/reflect that are defined in screen space move after `Place`.
7. Precomps: the nested comp is rendered at its own size, treated as content with `local_bounds` = comp size; continuous
   rasterisation (`collapse`, WP-14) later.
8. `inspect` prints `bounds_local`, `bounds_post_fx`, `bounds_screen`. `check`: `fx-needs-bounds` warning when a growing effect lacks growth.
9. Tests (`render/fx_tests.rs`): with `effect_space: layer` a blur of radius 10 on a 4x4 square paints alpha 14 px away and scales with
   layer scale 2 (20 px), a layer half off-screen still blurs/glows into the frame with its off-screen content, rotated directional blur
   follows the layer angle, two renders identical, an unchanged comp with `effect_space: screen` is byte-identical to before, memory
   for a 100-layer small-layer comp drops (record peak images from `comp graph`).
10. Done when probes from triage records 002 (glitch on text) and 005 (echo) render unclipped and `motions/` films render identically
    in `screen` mode (run every `rebuilds/*/film.ts` and `motions/launch-film`, compare hashes).

#### WP-02 Blur kernels and the blur family (L; depends WP-01)
1. In `core/src/effects/blur.rs` implement separable Gaussian with `sigma = blurriness / 2` [verify against AE docs and write
   the derivation in `parity/specs/gaussian_blur.md`], repeat-edge option, H/V/both, mip pyramid above radius 32.
2. Add kernel passes (`gaussian`, `box` x3 for Fast Box, `disc` with iris polygon, blades, rotation, curvature) to the
   shader runner so plugins declare them (ADR-1).
3. Effects: Gaussian Blur, Box Blur, Fast Box Blur, Directional Blur (true line integral), Radial Blur (spin + zoom), Channel
   Blur, Cross Blur, Smart Blur, Bilateral Blur, Compound Blur (map layer), Camera Lens Blur, Unsharp Mask, Sharpen.
4. Rewrite `tilt_shift` to use the kernel (same name and params).
5. Tests: impulse response measured sigma within 3%; energy conserved within 0.5%; a smooth gradient stays monotone after a
   large blur (no banding); bounds per WP-01.

#### WP-03 Colour pipeline (L; depends WP-00)
1. Schema: project `color` block (ADR-5), default unchanged. Files: `core/src/{layer,color,grade}.rs`,
   `render/composite.rs`, `media/*` (import conversion), `core/src/encode.rs` (tagging).
2. `@space` header tag; the graph wraps `linear` effects with sRGB<->linear conversions when `linear_light` is on.
3. Import: read ICC/colour tags from images and `ffprobe` for video, convert to the working space.
4. Tests: with `linear_light` off the **entire existing suite is byte-identical**; with it on: 50% grey over black at 50%
   opacity gives the physically correct value; Add of two 0.5 greys; blur midpoint of a black/white edge; a P3 swatch converts to
   known sRGB values.
5. Record in the ledger which effects honour `@space linear`.

#### WP-05 Blend modes (M; depends WP-03)
1. Add the missing modes to `core/src/effects/blend.rs` enum: dissolve, dancing_dissolve, darker_color, lighter_color,
   linear_burn, linear_dodge (alias add), vivid_light, linear_light, pin_light, hard_mix, subtract, divide, stencil_alpha,
   stencil_luma, silhouette_alpha, silhouette_luma, alpha_add, luminescent_premul, classic_color_burn, classic_color_dodge,
   classic_difference.
2. Modes Skia lacks: a shader blend in `Stage::Composite` (read below + layer, write composite); formulas from
   EffectCraft `color/src/blend.rs`.
3. Tests: each mode on 3x3 pixel pairs against the formula, in display and (WP-03) linear space.

#### WP-14 Layer switches and semantics (M; depends WP-00)
1. Add layer keys: `guide`, `shy`, `solo`, `quality` (`draft` skips effects tagged `@cost heavy`),
   `preserve_transparency`, `use_as_matte_for_below`, `collapse` (continuous rasterisation of precomps), layer markers with
   `duration` and `comment`, `auto_orient` (`camera`, `path`).
2. `ae_order` comp flag (default false): when true, `layer_picture` runs Paint -> masks -> effects (AE order). Add test with both
   orders; flip the default only with owner approval after reviewing goldens.
3. Matte on an adjustment layer with a hidden source: make it work or error loudly (`check`).
4. Verify 3D `orientation` overrides `rotation` order X Y Z against AE; document.

### Phase C: motion model

#### WP-10 Property model v2 (L; depends WP-00)
1. Split `anim/prop.rs` (531 lines) into `anim/prop/{key,value,loop,spatial,parse}.rs` with no behaviour change; the existing
   suite stays green (commit point 1).
2. Add `Value::{F,V2,V3,C,Path,Text,B}`; `eval(prop, t, env) -> Value`.
3. Add `Interp::{Hold,Linear,Bezier{out_speed,out_influence,in_speed,in_influence}}` per key; map to a cubic bezier exactly as AE
   (influence = x-fraction, speed = slope). Use `tmp/clones/effectcraft/docs/fidelity/keyframe-ease.json` samples as test vectors.
4. Spatial bezier for position with in/out tangents, arc-length parametrisation; `roving`; `separate_dimensions`;
   `auto_orient: path` sets z rotation from the tangent.
5. Update `schema/mod.rs` position/anchor types, `eval/pose.rs` transform evaluation, `schema/timeline/*` and the SDK `tl.to`
   (vectors, handles, `path`).
6. Tests: tangent-free keys equal today's output on all goldens; a quarter-circle path hits 45 degrees at the arc-length midpoint
   within 0.5 px; influence 33/speed 0 equals `cubic(0.33,0,0.67,1)`; roving keys keep constant speed; auto-orient gives the
   tangent angle.
7. Done when a circular path with 4 keys renders a smooth circle with a banked object.

#### WP-11 Expression controls and loops (M; depends WP-10)
1. Add pass-through effects (no pixels) `slider`, `checkbox`, `color_control`, `angle_control`, `point_control`,
   `point3d_control`, `layer_control`, `dropdown` in `core/src/effects/controls.rs` with `pixels:false`.
2. Expression access `effect("Name")("Control")` and by index.
3. `loop` key gains `continue` (extrapolate last velocity) and `loopIn`; document `cycle|pingpong|offset`.
4. Tests: a slider drives two layers' opacity; a checkbox switches a branch; `continue` extrapolates.

#### WP-12 Expressions on QuickJS (XL; depends WP-10, WP-11)
1. Add `rquickjs` to `crates/comp/Cargo.toml` (already used by `script`).
2. `anim/expr/host.rs`: Rust host API (`getProp(layer, path, t)`, `layerCount`, `compSize`, `markers`, `sampleImage` on a layer
   rendered in a prior stage).
3. `anim/expr_prelude.js`: AE object model (`thisComp`, `thisLayer`, `thisProperty`, `effect()`, `value`, `time`, `index`,
   `valueAtTime`, `velocityAtTime`, `key`, `nearestKey`, `numKeys`, `loopIn/loopOut` for all types, `linear`, `ease`, `easeIn`,
   `easeOut`, `clamp`, `length`, `normalize`, `dot`, `cross`, `lookAt`, `toComp`, `fromComp`, `toWorld`, `fromWorld`,
   `random`, `gaussRandom`, `noise`, `seedRandom`, `posterizeTime`, `timeToFrames`, `framesToTime`, `sourceRectAtTime`,
   `comp("name").layer()`, marker access, `createPath`). Reference: EffectCraft `expr/src/host.rs`.
4. Determinism: fresh context per evaluation batch; remove `Math.random`, `Date`, `performance`, timers; supply seeded versions.
5. Keep the old tiny syntax working: parse first with the existing parser; QuickJS only for strings it rejects (documented and
   tested rule, not a silent fallback).
6. Dependency graph at plan time; reject cycles with a path.
7. Tests: 80 expressions with expected values (from AE documentation semantics); determinism (same inputs, same outputs, no
   clock); cycle rejection; cost test (10k expression evaluations under a budget recorded in the report).
8. Done when one `effect("Master")("Amount")` slider drives scale, colour and trim of 12 layers.

#### WP-13 Time (L; depends WP-01)
1. `posterize_time`/`step_fps` as layer key and effect: quantise the layer's evaluated time before keys/expressions (the whole
   subtree). Replaces hand-cycled `doodle_boil`.
2. `time_displacement` (map layer), `time_difference`, `frame_blend`, `pixel_motion_blur`, `timewarp`: block-matching optical
   flow in two shader passes (3 pyramid levels). Reference: EffectCraft `raster/src/flow.rs`.
3. Per-layer motion-blur toggle; comp `shutter_angle`, `shutter_phase`, `samples` (cap 16, cost warning).
4. `reverse: true` on video/precomp.
5. Tests: posterize 12 fps gives 12 distinct states per second; keys read at quantised time; frame blend A/B at t=0.5 equals the
   mix; flow recovers an 8 px translation within 1 px.

### Phase D: vocabulary

#### WP-30 Shape operators and strokes (XL; depends WP-00)
1. Operators as an ordered list on a shape group, applied to the group's paths before fill/stroke, each animatable:
   `merge {mode: merge|add|subtract|intersect|exclude}` (Skia PathOps), `offset_paths`, `pucker_bloat`, `round_corners`,
   `twist`, `zig_zag`, `wiggle_paths`, `wiggle_transform`, `trim {mode: simultaneous|individually}`,
   `repeater {copies (fractional, animatable), offset, anchor, position, scale, rotation, start_opacity, end_opacity,
   composite: above|below}`. Files: `crates/comp/src/shape/ops/<name>.rs`, one per operator; schema in `schema/shape.rs`,
   `schema/path.rs`, `schema/repeat.rs`; eval in `eval/shape.rs`; render in `render/shape.rs`.
2. Strokes: `cap`, `join`, `miter_limit`, `dashes` + phase, gradient stroke, taper (start/end length and width, ease), wave, variable
   width profile `[(pos,width)...]` built from offset curves (reference `path/src/varstroke.rs`); sign flip of width = fold, two
   face colours (the triage's tapered ribbon arrow).
3. Mask extras: per-vertex feather (variable width) in `render/masks.rs`; mask path bindings via expressions.
4. First-class `text_to_path`, `path_to_points` (also used by the mesh sweep).
5. Tests: boolean results by area; zig_zag vertex count; wiggle determinism by seed; repeater opacity ramp; taper endpoint width
   within 0.25 px; dash phase animation.

#### WP-31 Layer styles (L; depends WP-01)
1. Effects on the layer's alpha, usable on any layer, in AE order: `drop_shadow`, `inner_shadow`, `outer_glow`, `inner_glow`,
   `bevel_emboss` (style outer/inner/emboss/pillow; technique smooth/chisel; depth, direction, size, soften, angle, altitude,
   highlight/shadow colour and mode), `satin`, `color_overlay`, `gradient_overlay` (angle, scale, linear/radial/angle/reflected/
   diamond, reverse, dither), `pattern_overlay`, `stroke` (outside/inside/centre; colour/gradient); layer key `fill_opacity`
   and `knockout`. Reference `render/src/styles.rs`; write as SkSL under `core/src/effects/styles/` or
   `comp/src/fxlib/builtin/`.
2. Tests: bevel lights the top-left edge brighter than bottom-right at angle 120/altitude 30; inner shadow only inside alpha;
   inside stroke never exceeds original bounds.
3. This is the cheapest route to plastic letters, sticker type, emboss and inflatable-type fakes before any 3D.

#### WP-32 Text (L; depends WP-01, WP-10)
1. Expose measurement to the engine: `sourceRectAtTime(t, include_extents)` for expressions (WP-12) and layer key
   `box: {fit:"text", padding:[x,y], radius, fill, min_width, align, per_line}`; `padding` animatable; width follows the
   characters currently revealed (so a typed bar grows). Existing measurement: `text/measure.rs`.
2. Selectors (`text/selectors` in `text/animate.rs`): `based_on: chars|chars_no_spaces|words|lines`; multiple selectors per animator
   with `mode: add|subtract|intersect|min|max|difference`; `shape: square|ramp_up|ramp_down|triangle|round|smooth`; `ease_high`,
   `ease_low`; `randomize_order` with seed; wiggly selector complete (`correlation`, `lock_dimensions`, `temporal_phase`,
   `spatial_phase`).
3. Animator properties: `tracking_amount`, `line_anchor`, `line_spacing`, `character_offset`, `character_value`, `blur`,
   `fill_opacity`, `stroke_opacity`.
4. Paragraph: all justify modes, first-line indent, space before/after, leading animation, all-caps/small-caps,
   super/subscript, OpenType `features`, variable-font `variations` (animatable). Source-text keyframes with hold.
5. Tests: box width = measured width + 2*padding within 0.5 px for 10 strings in 3 fonts (including kerning); typed box grows
   monotonically; word selector selects exactly the words (check via `inspect`); intersect weights; variable weight changes advance.

#### WP-33 Effects long tail (XL, parallel; depends WP-00, WP-01, WP-02, WP-03)
A loop: take the next `todo` row of `parity/effects.tsv` ordered by tier then name; run the porting procedure (section 4) and
shader steps (section 5.1); repeat. Max five effects per sitting, then report. Authoring rules: parameter names/order/units/
defaults/ranges are AE's; each effect declares `@bounds`, `@space`, `@cost`; premultiplied correctness through `straight()/premul()`;
all animatable; angles in degrees; positions in layer pixels unless AE's parameter is a fraction; at most 4 full-size half-float
images; no loops over more than 32 taps (use kernel passes). Order of work: T0 (section 10) first.

#### WP-34 Warps (L; depends WP-01, WP-20)
Corner Pin (4 points, homography, AA), Bezier Warp (12 points), Mesh Warp (grid with tangents, rendered as a subdivided textured
mesh through the WP-20 rasteriser), Liquify (recorded stroke list accumulated into a deterministic displacement field),
Puppet pins (as-rigid-as-possible 2D deformation, fixed iteration count), Polar Coordinates, Spherize, Reshape, Offset, Smear,
Magnify, Optics Compensation (`barrel` + FOV mode), Rolling Shutter Repair, CC Bend It/Bender/Page Turn/Slant/Tiler/RepeTile/
HexTile/Cylinder/Sphere/Split. **Motion Tile** with output size %, tile centre, mirror edges, phase, gutter colour/size.
**Page curl** implemented once as a mesh deformer (WP-20) and exposed as `cc_page_turn`.

#### WP-35 Keying and mattes (M; depends WP-33)
Keylight-class keyer (screen colour, gain, balance, clip black/white, despill bias, edge colour, matte shrink/grow/softness,
replace method), Colour Key, Colour Range, Difference Matte, Luma Key, Linear Color Key, Inner/Outer Key, Spill Suppressor, Matte
Choker, Simple Choker, Refine Hard/Soft Matte. Tests on synthetic chroma images with known alpha; spill reduction removes at
least 80% of residual green on the test card.

### Phase E: 3D (strictly sequential; do not start before Phase B and WP-30..32 are done)

#### WP-20 Mesh pipeline: rasteriser and G-buffer (XL; depends WP-00, WP-03)
1. `mesh/raster/{mod,tiles,edge,coverage,gbuffer}.rs`: tile binning, perspective-correct interpolation of normal/UV/position,
   near-plane clipping (Sutherland-Hodgman against `w > epsilon`), optional backface cull, 4x4 ordered coverage, deterministic
   order across threads.
2. Geometry (`schema/kinds.rs::Geometry` gains variants; keep old ones): `plane {w,h,seg_x,seg_y}`, `rounded_box`,
   `rounded_rect_extrude`, `lathe`, `sweep {profile, path, segments, taper, twist}`, `inflate {path|text, amount,
   profile: round|soft}` (signed-distance height field: `h = amount*sqrt(1-(1-d/dmax)^2)`, triangulated at 2 px), text gains
   `bevel_profile: chamfer|round|concave|inflate` and `bevel_segments`. Fix the bevel >= 14 self-overlap by offsetting contours
   inward per ring (test: no overlapping triangles by area check).
3. Deformers in `mesh/deform/`: `bend`, `twist`, `taper`, `wave`, `noise`, `page_curl {progress, axis_angle}` (needs plane seg >= 64).
4. UVs for every primitive; layer `texture` (an image/precomp/video id) as base colour/normal/height/roughness.
5. Benchmark: 1080p, 200k triangles, 4 text meshes, record ms/frame. Over 60 ms: apply the fallback in section 11 and tell the owner.
6. Tests: cube golden at 3 rotations; two intersecting quads cut along the expected line within 1 px; identical bytes with 1 vs 8
   rayon threads; near-plane clip; a plane bent 90 degrees keeps arc length; page_curl at progress 0 equals flat.

#### WP-21 Mesh materials (L; depends WP-20, WP-03)
1. New plugin kind `material` (`fxlib/materials/*.sksl`, same header, plus `@gbuffer` inputs).
2. Built-ins: `standard` (base colour/texture, metalness, roughness, clearcoat, emissive, normal map, two-stop face gradient
   `gradient:{top,bottom,by:"y"|"z"}`), `chrome`, `glass` (refraction of the backdrop snapshot via `Stage::Snapshot`, `ior`,
   `dispersion`, `thin_film`), `plastic_film`, `toon`, `matcap`, `iridescent`. Mesh `color` accepts gradient objects. Old
   four-float `material` maps to `standard`.
3. Environment: comp-level `environment` (image layer id or procedural `studio|sky|sunset|neon`), prefiltered roughness mip
   chain computed once and cached; Schlick fresnel; reference three.js `ShaderChunk`.
4. Tests: chrome sphere in a two-colour horizon shows the horizon bent at the analytic row within 2 px; roughness 1 equals the mean
   environment colour; glass sphere inverts the scene behind it; gradient face maps top/bottom colours to the glyph's top/bottom
   rows; deterministic.
5. Done when probes render for: chrome sphere, glass with thin film, inflated gradient type "MOVE" (template 020), glossy tube
   squiggle (template 018).

#### WP-22 3D group depth compositing (XL; depends WP-20)
New `Stage::Depth3D` in `render/graph/mod.rs` and `build.rs`; collect contiguous `three_d` layers into one group; per-pixel
fragment lists of depth K=8 in an RGBAF16 accumulation texture array (collect pass, resolve pass); flat layers' plane depth computed
analytically from the layer `screen` matrix inverse. Tests: two planes crossing at 90 degrees intersect on the right line; a hinged
three-face corner is right from three camera angles; a transparent plane in front of a mesh shows the mesh; with no intersections
the whole existing golden suite is unchanged. Done when templates 003 and 018 hinge/floor cases need no ordering hack.

#### WP-23 Lights and shadows (L; depends WP-20, WP-22)
Light keys: `cast_shadows`, `shadow_darkness`, `shadow_diffusion`, intensity above 100 and negative, `falloff`, spot
`cone_angle/cone_feather`. Layer keys: `casts_shadows`, `accepts_shadows`, `accepts_lights`, `light_transmission`. Shadow maps from the
WP-20 rasteriser (directional ortho; spot/point perspective), PCF 5x5 plus diffusion blur; flat layers cast as alpha quads;
`shadow_catcher`. Tests: sphere over a plane shadows at the analytic centre within 2 px; darkness 50% halves; deterministic.

#### WP-24 Camera and depth of field v2 (M; depends WP-02, WP-22)
Camera properties `focal_length`, `film_size`, `angle_of_view`, `f_stop`, `focus_distance`, `blur_level`, `iris`
{shape,rotation,roundness,aspect}, `one_node|two_node`, `auto_orient`; `dof {focus, blur}` stays as an alias. Convert exactly as AE
(aperture = focal_length / f_stop; zoom = focal_length * comp_width / film_size). DoF becomes per pixel from the depth buffer
with the Lens Blur iris kernel (WP-02); the current per-layer blur in `eval/mod.rs` stays only for layers alone in a non-3D group.
Tests: a tilted plane's blur grows with depth; focus plane has zero blur; circle-of-confusion formula against hand calculation.

### Phase F: simulation, I/O, tools

#### WP-36 Particles v2 (XL; depends WP-20, WP-33)
Keep closed form where possible. Add: image sprite per particle (layer id or atlas with random frame), per-life curves for size/
rotation/colour/opacity, 3D emitters (box, sphere, mesh surface, glyph outline), analytic force fields (gravity, wind, turbulence,
vortex, attractor), motion-blur trails, sub-emitters. Effects: CC Particle World/Systems II, Rainfall, Snowfall, Drizzle, Bubbles,
Scatterize, Star Burst, Pixel Polly, Ball Action, Particle Playground (subset). Reference: EffectCraft `gpu/src/{particles,fx_*}.rs`.

#### WP-37 Rigid-body bake (L; depends WP-20)
`crates/comp/src/physics/`: fixed-step (1/240 s) spheres, boxes, planes with gravity, restitution, friction, sleeping, seeded
initial conditions; baked to keyframes at plan time and cached by input hash. Layer key `physics {body, collider, mass,
restitution, friction, initial_velocity}`; comp `gravity`, `seed`. Tests: a dropped sphere rests at radius above the plane; two
spheres interpenetrate by at most 1 px at 25 fps; identical across runs and thread counts.

#### WP-40 Matte and segmentation import (M; depends WP-14)
`subject_matte` layer source: an offline-produced matte clip/sequence imported frame-accurately. Optional helper
`scripts/segment_subject.py` (documented, run as `python3 -I`, own folder, open-licence model). Engine only reads the matte.

#### WP-41 Simple tracking (M; depends WP-14)
Single-point and planar-corner template tracker (normalised cross-correlation, search window, parabola sub-pixel) producing
keyframes via a `track` CLI writing `props.json`; stabilise as inverse transform. No camera solver. Reference `track/src/*`.

#### WP-50 Output formats (M)
Image sequences (PNG 8/16-bit, JPEG), EXR (half/float, scene-linear when WP-03 is on), ProRes 422 HQ/4444/4444 XQ, H.265, VP9/
WebM with alpha, GIF and WebP (per-frame palette, dithering), `render_range` (work area), render-queue file, `--proxy`. Tests:
gradient round trip through EXR and ProRes within tolerance; GIF palette size; straight alpha at the file boundary.

#### WP-51 Import (S-M)
Lottie/Bodymovin JSON -> comp JSON (shapes, transforms, trims, simple effects) in `comp/src/tools/lottie/`; improve SVG import
(gradients, clip paths, transforms; `tools/svg.rs`). Lottie export is out of scope.

### Phase G: devices (pure TypeScript in `motions/lib/<name>/`, typed `Theme`, README, probe, `refstudy diff` pass)

A device is written only after its primitive passes its gate. Existing in-flight work that is not yours: `brush-stroke`,
`wet-reflection` (review, test and commit only if the owner asks).

| Device | Needs |
| --- | --- |
| `text_bar`, `chip`, `pill`, `lower_third` | WP-32 `box` (no per-string constants) |
| `scatter_in`, `wonky_title`, `stacked_type` | `selector.wiggle` (landed), repeater palette cycle (WP-30) |
| `torn_paper_wipe`, `ink_wash` | new directional shader `torn_front {progress, angle, rim}` (`torn_edge` stays the dissolve; document), WP-01, WP-33 noise |
| `melt_type` | WP-34 displacement, `pixel_smear` on a separate copy, repeater palette cycle |
| `plastic_letters` | WP-31 styles, turbulent displace |
| `retro_window`, `laptop_and_tablet`, `phone` | WP-20 `rounded_box`, WP-34 corner pin |
| `folding_poster`, `cube_corner` | WP-22 |
| `bendable_plane` | WP-20 deformers |
| `puppet_character` | WP-30 path morph, WP-10 spatial paths, WP-34 puppet |
| `anamorphic_text` (logic module) | per-glyph ray-depth solve |
| `paper_unfold` | WP-20 plane + hinge rig (still `unsure`) |

### Phase H: finish the templates
After WP-01..05 and WP-30..32: triage 033-050 (new records cite ledger ids, each `ready` cites a probe image path; make
`scripts/artlist_triage.py check` fail otherwise), then run the queue (`python3 scripts/artlist_queue.py next --agent NAME`), build
beats through the `template-rebuild` skill, and accept only beats whose `refstudy diff` reports `pass: true`.

### 7a. Core-model ledger seed (copy into `parity/MATRIX.tsv`)

Status key: have, part, none, unverified.
Transform/3D transform: have. Per-component orientation vs rotation: unverified (WP-14). Spatial bezier, roving, auto-orient: none (WP-10).
Separate dimensions, temporal/spatial ease split, influence handles: none (WP-10). Hold keys: have. Looping: part (WP-11).
Expressions scalar: have; full object model: none (WP-12). Expression controls: none (WP-11). Layer switches: part (WP-14).
Parenting: have. Track mattes: have. Matte on adjustment layer: part (WP-14). Masks: have; per-vertex feather: none (WP-30).
Blend modes: part 17/38 (WP-05). Markers: part (WP-14). Precomp time remap: have. Collapse transformations: unverified (WP-14).
Frame blending/pixel motion blur/Timewarp: none (WP-13). Posterize/Time displacement/difference: none (WP-13). Motion blur
per layer: part (WP-13). Colour management: none (WP-03). Output: part (WP-50). Lights shadows: none (WP-23). Camera f-stop/iris: none (WP-24).
Layer styles: part (WP-31). Shape operators: none (WP-30). Gradient stroke, taper, wave: none (WP-30). Text bounds: none (WP-32).
Text selector based-on and modes: none (WP-32). Variable font axes: none (WP-32). Environment/reflection/refraction: none (WP-21).

---

## 8. Gate for every WP (all must pass; paste output tails into REPORT.md)

1. `cargo test --release --workspace --no-fail-fast`: zero failures; existing goldens unchanged unless the WP says so.
2. `cargo run --release -p turbomotion -- docs --write`; `git diff --stat docs/` shows only expected files.
3. `python3 scripts/capability_index.py`; `... find <feature>` returns it.
4. `turbomotion parity check` exits 0 (after WP-00).
5. `turbomotion comp check` on every probe JSON: no errors and no unintended warnings.
6. Determinism: render the probe twice; `shasum` equal.
7. Probe images saved in `<WP>/probes/`; open them and state in one sentence what each shows.

## 9. REPORT.md format
```
WP: <id and name>
Status: done | partial | blocked
Changed files: <list>
Tests added: <names>
Gate output: <tails>
Deviations from this plan: <each with the reason>
Open questions for the owner: <list>
Estimated cost to finish (if partial): <hours>
```

## 10. Effects worklist (seed for `parity/effects.tsv`)

Recipe classes: A single-pass shader; B multi-pass or kernel; C history; D CPU/Rust analysis; E composition-level.
`have` = a same-job effect exists (parameters may need aligning to AE); `part` = exists with different semantics/quality; no tag = to build.
Tiers: T0 templates need it (do first); T1 AE core; T2 long tail.

**Blur & Sharpen:** Bilateral Blur [B,T1], Box Blur [B,T0], Camera Lens Blur [B,T0], Channel Blur [B,T1], Compound Blur [B,T1],
Directional Blur [part], Fast Box Blur [B,T0], Gaussian Blur [part: `blur`], Radial Blur [part], Smart Blur [B,T1], Unsharp Mask [B,T0],
Sharpen [part], Reduce Interlace Flicker [A,T2].
**Channel:** Arithmetic [A,T2], Blend [A,T1], Calculations [A,T2], Compound Arithmetic [A,T2], Invert [have], Minimax [A,T0], Remove Color
Matting [A,T1], Set Channels [A,T1], Set Matte [A,T1], Shift Channels [A,T1], Solid Composite [A,T1], Cineon Converter [A,T2].
**Colour Correction:** Auto Color/Contrast/Levels [D,T1], Black & White [A,T0], Brightness & Contrast [part], Broadcast Colors [A,T2],
Change Color [A,T1], Change to Color [A,T1], Channel Mixer [A,T1], Color Balance [A,T1], Color Balance (HLS) [A,T2], Colorama [A,T0],
Curves [A,T0], Equalize [D,T2], Exposure [have], Gamma/Pedestal/Gain [part: `grade`], Hue/Saturation [part], Leave Color [part:
`chroma_hold`], Levels [A,T0], Levels (Individual) [A,T1], Lumetri Color [B,T1], Match Color [D,T2], Photo Filter [A,T1], Selective
Color [A,T1], Shadow/Highlight [B,T1], Tint [have], Tritone [A,T0], Vibrance [A,T1], Apply Color LUT [A,T0] (`.cube` loader in core).
**Distort:** Bezier Warp [E,T1], Bulge [have], Corner Pin [A,T0], Displacement Map [have], Liquify [D,T1], Magnify [A,T1], Mesh Warp
[E,T1], Mirror [have], Offset [A,T1], Optics Compensation [part: `barrel`], Polar Coordinates [A,T1], Reshape [D,T2], Ripple [have],
Rolling Shutter Repair [D,T2], Smear [A,T2], Spherize [A,T1], Transform [have], Turbulent Displace [have], Twirl [have], Wave Warp
[part: `wave`; types sine, square, triangle, sawtooth, circle, semi-circle, uncircle, noise, smooth noise].
**Expression Controls:** Angle, Checkbox, Color, Dropdown, Layer, Point, 3D Point, Slider (WP-11).
**Generate:** 4-Color Gradient [A,T0], Advanced Lightning [part: `lightning`, `bolt`], Audio Spectrum [D,T1], Audio Waveform [D,T1], Beam
[A,T1], Cell Pattern [have: `cells`], Checkerboard [A,T1], Circle [A,T1], Ellipse [A,T1], Eyedropper Fill [D,T2], Fill [A,T1], Fractal [A,T2],
Grid [have], Lens Flare [B,T0], Paint Bucket [D,T2], Radio Waves [A,T1], Ramp [part: `gradient`], Scribble [D,T1], Stroke [D,T1], Vegas [D,T1],
Write-on [D,T1], CC Light Burst [B,T1], CC Light Rays [part: `god_rays`], CC Light Sweep [part: `light_sweep`], CC Threads [A,T2].
**Keying/Matte (WP-35):** Color Difference Key, Color Key, Color Range, Difference Matte, Extract, Inner/Outer Key [D], Keylight [B,T0],
Linear Color Key, Luma Key, Spill Suppressor, Matte Choker [B,T0], Refine Hard/Soft Matte, Simple Choker.
**Noise & Grain:** Add Grain [part: `grain`, `film_grain`], Dust & Scratches [part], Fractal Noise [part: `noise`, `clouds`; needs 17 noise
types, complexity, sub-influence/scaling/rotation/offset, evolution + cycle, seed, opacity, blend] [B,T0], Match Grain [D,T2], Median
[B,T1], Noise [A,T1], Noise Alpha/HLS [A,T2], Remove Grain [B,T2], Turbulent Noise [A,T1].
**Perspective:** Bevel Alpha [B,T0 via styles], Bevel Edges [B,T0], CC Cylinder [A,T1], CC Sphere [B,T1], CC Spotlight [B,T1], Drop Shadow
[part: `shadow`; add shadow-only, opacity, softness, direction], Radial Shadow [B,T1].
**Simulation (WP-36):** Card Dance [D,T2], Caustics [B,T2], Foam [D,T2], Particle Playground [D,T2], Shatter [D,T1], Wave World [B,T2],
CC Ball Action, Bubbles [B,T1], Drizzle, Hair, Mr. Mercury, Particle Systems II [D,T1], Particle World [D,T1], Pixel Polly [D,T1], Rainfall [B,T1],
Scatterize [D,T1], Snowfall [B,T1], Star Burst [B,T1].
**Stylize:** Brush Strokes [B,T0], Cartoon [B,T0], Color Emboss [A,T1], Emboss [A,T1], Find Edges [A,T0], Glow [part: `glow`; needs radius,
intensity, threshold, composite original on/behind/none, colours A/B, looping, colour phase], Mosaic [part: `pixelate`], Motion Tile [A,T0],
Posterize [have], Roughen Edges [A,T1; corrected 2026-10-26: the catalogue has no `roughen_edges`, `highlighter` is a different effect], Scatter [A,T0], Strobe Light [A,T0], Texturize [A,T1], Threshold [A,T0], CC Block Load [A,T2],
CC Burn Film [A,T2], CC Glass [B,T0], CC HexTile [A,T1], CC Kaleida [A,T2], CC Mr. Smoothie [A,T2], CC Plastic [B,T0], CC RepeTile [A,T1],
CC Threshold [A,T1], CC Vignette [part: `vignette`].
**Text:** Numbers [A,T1], Timecode [A,T1] (text sources, WP-32).
**Time (WP-13):** CC Force Motion Blur [C,T1], CC Wide Time [C,T2], Echo [part: hard copies via repeater], Pixel Motion Blur [D,T1], Posterize
Time [T0], Time Difference [C,T1], Time Displacement [C,T0], Timewarp [D,T1].
**Transition (per-layer effects):** Block Dissolve [A,T0], Card Wipe [D,T1], Gradient Wipe [A,T0], Iris Wipe [A,T0], Linear Wipe [A,T0],
Radial Wipe [A,T0], Venetian Blinds [A,T0], CC Glass Wipe [B,T1], CC Grid Wipe [A,T1], CC Image Wipe [A,T1], CC Jaws [A,T2], CC Light Wipe
[A,T1], CC Line Sweep [A,T1], CC Radial Scale Wipe [A,T1], CC Scale Wipe [A,T1], CC Twister [D,T1], CC WarpoMatic [A,T2]. Port the existing
GLSL transitions from gl-transitions as shader transitions too.
**Utility:** Apply Color LUT [A,T0], Color Profile Converter [A,T1 with WP-03], Grow Bounds [A,T0 with WP-01], HDR Compander [A,T2], HDR
Highlight Compression [A,T2], Cineon Converter [A,T2].

About 205 effects are in scope; roughly 130 are class A/B and mechanical; about 25 need real code (class D); T0 is about 45 and is the first milestone.

---

## 11. Risks and the decisions that stay open

| Risk | Mitigation |
| --- | --- |
| Software rasteriser too slow at 1080p (WP-20) | Benchmark gate; tile + rayon; if over 60 ms, rasterise only the G-buffer on the CPU and keep Skia for flat shading; report the number. **Open decision for the owner at that point:** build 3D in-engine (this plan) or render 3D scenes with three.js outside the engine as a cached alpha-video "provider" layer (a layer kind `provider` playing an offline-rendered, hash-cached alpha PNG sequence). Not started; do not build it without the owner's instruction |
| Per-pixel fragment lists cost (WP-22) | K=8 cap; group only 3D layers; skip groups whose depth ranges do not overlap |
| Colour changes break goldens | Default off; whole suite byte-identical before turning anything on |
| Property v2 breaks the SDK and every film | Keep all legacy forms; run every `motions/**/film.ts` build in a test; only formatting differences allowed |
| Effects drift from AE meaning | Spec transcription in the ledger; reviewer reads the AE row, not the shader |
| Scope creep into clones nobody uses | Tiers; T2 only when idle; the reference videos define T0 |
| More unverified "ready" verdicts | Rule: `ready` needs a probe image path in the record; `artlist_triage.py check` enforces |
| In-flight uncommitted work | Do not touch or commit (rule 5) |

Effort sanity range: Phases A-B about 3 weeks; C about 4 weeks; D open-ended (the effects long tail is about 130 mechanical tasks);
E about 6-8 weeks; F and G open. EffectCraft's own estimate for full After Effects parity is 850-1,500 agent-hours: a range, not a promise.

Definition of done for the programme: `parity report` at least 95% `done` for T0+T1 with the not-built list documented; every
reference template has a beat passing `refstudy diff` and 15 complete films; no catalogue example fails `check`; full suite green and
goldens stable with `linear_light` off; `docs/ARCHITECTURE.md` updated for effect model v2, the mesh pipeline, the 3D group and the colour
pipeline.
