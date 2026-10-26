WP: WP-03 fix, dark levels of small solids and shapes in linear light (not a new work package)

Status: done

## Root cause, with evidence

Symptom (WP-05 worker, open question 7): in a `"color": {"linear_light": true}` film a solid or shape smaller than the frame
shows level 8 and level 16 both as 13, level 48 as 50. Full-frame solids are exact.

The suspected cause was wrong. Every picture of the pipeline is already RGBAF16 (`core::layer::picture_info`; the backdrop,
shutter accumulator, layer-space buffers, mesh batch, nested compositions and effect outputs all use it), and the only 8-bit
surfaces are the final BGRA frame (the one encode, at the end), decoded images and video frames (8-bit sources, converted
in the shader when drawn), the 128 px particle sprite (alpha only), and the layout-mask readback (alpha only). No linear
value is stored in an 8-bit surface between operations.

The real cause is in how Skia's GPU backend draws a **constant paint colour**. Skia converts the paint colour into the
picture's colour space on the CPU, then hands it to the fragment stage as an **8-bit-per-channel vertex attribute** of the
geometry (`GrQuadPerEdgeAA` colour type "byte" when the colour fits 0..1). In display encoding that is harmless: the colour was
8 bits to begin with. In linear light the converted colour is quantised to multiples of 1/255 of *light*: level 8 (0.00243)
and level 16 (0.00518) both become 1/255 (0.00392), which is level 13. A full-frame solid is drawn by Skia as a clear
(float) and is not affected; a shader's uniform is a float, so gradients are not affected either.

Measured directly on the GPU, one `RGBAF16` surface tagged linear sRGB, an 8x8 rect drawn at (20,20), the light value read
back unconverted (`1.000/255` is `0.00392`):

| level | wanted light | rect, AA | rect, no AA | path, AA | full-surface rect | Skia colour shader | runtime shader with `layout(color)` uniform |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 8 | 0.619/255 | 1.000/255 | 1.000/255 | 1.000/255 | 0.619/255 | 1.000/255 | **0.619/255** |
| 16 | 1.321/255 | 1.000/255 | 1.000/255 | 1.000/255 | 1.321/255 | 1.000/255 | **1.321/255** |
| 48 | 7.537/255 | 8.000/255 | 8.000/255 | 8.000/255 | 7.537/255 | 8.000/255 | **7.537/255** |

So anti-aliasing is not the cause, a rect and a path are the same, a Skia colour shader is folded back into the vertex
colour (still 8 bits), and a runtime shader's uniform is exact.

### Every place a flat dark colour reaches a picture (test `render/color_flat_tests.rs`, levels 8, 16, 48 in light against the default film)

| source | affected before the fix | reason |
| --- | --- | --- |
| solid smaller than the frame | yes | paint colour |
| shape fill, flat colour | yes | paint colour |
| shape stroke | yes | paint colour |
| text fill, text stroke, text mark | yes (`level 8 shows 13 in light and 8 on encoded numbers`) | paint colour |
| particle circle, square, star, ring, spark, path | yes | paint colour (`set_color`) |
| full-frame solid | no | drawn as a float clear |
| shape or text gradient | no | shader, float |
| composition `bg` | no | a float clear (`background` test passes before and after) |
| mesh faces | no | vertex colours are display-referred 8-bit sRGB, converted per fragment |
| shadow, outline, glow colours, bitmap glyph ink | no | colour filters, float uniforms |
| images, video frames | no (8-bit sources, converted in the shader on draw) | |
| soft particle sprite | no | alpha only, tinted by a colour filter |

One related finding that is **not** fixed here (open question 1): the same 8-bit carrier holds a layer's **opacity**, the
alpha of the paint the composite is drawn with. White at opacity 0.002 in linear light shows level 13 (the light says 6.6),
0.001 shows 0 (3.3), 0.01 shows 28 (25.5); 0.02 and 0.5 are within a level. Measured with `draw_image` of an `RGBAF16`
picture at paint alpha 0.5: 0.50195 (the nearest 1/255).

## The fix: one mechanism

`core::color::Ink` (`crates/core/src/color/ink.rs`, `shaders/ink.sksl`): `Ink::apply(paint, canvas, color)` makes a paint
paint a flat sRGB colour. When the canvas is tagged with a colour space (any managed film) the colour is the uniform of a
tiny runtime shader declared `layout(color) uniform float4`, so Skia converts it in float and it can never be folded into
the geometry; when the canvas is untagged nothing is converted, so the paint's colour is set as it always was (the default
film is byte-identical, below). `Ink` is compiled once with the other platform programs (`Shaders::ink`).

The painters that held a colour in a paint now go through it: `render/content.rs` (solid, new `paint_solid`),
`render/shape.rs` (fill, stroke), `render/text.rs` (fill, stroke, mark), `render/particles.rs` (all but the soft sprite).
Nothing else changes: the pictures were already half-float, and the final encode to 8 bits still happens once, at the end,
with the output profile applied. No dithering, no table, no special case for small layers, no tolerance change in any
existing test. `depth` still accepts only `half`; no format needed to change, so there is no unsupported-format path.

## Before and after, in numbers

Probe films (`wp/WP-03/fix-darks/`, `make.py`, `run.sh`, `levels.py`): 256 swatches of 32 px in a 640 px frame (a solid; a
shape), linear light and default, red channel at each centre, all three channels checked equal. Tables in `before/` and
`after/`; the baseline binary is `git archive HEAD` built in an empty directory.

| film | levels more than 1 off | worst error | distinct outputs of 256 | levels not above the one below |
| --- | --- | --- | --- | --- |
| solid, linear light, before | 26 | 6 | 183 | 73 |
| solid, linear light, after | 0 | 0 | 256 | 0 |
| shape, linear light, before | 26 | 6 | 183 | 73 |
| shape, linear light, after | 0 | 0 | 256 | 0 |
| solid, default, before = after | 0 | 0 | 256 | 0 |

Levels 1..32 before: `1..6 -> 0`, `7..17 -> 13`, `18..25 -> 22`, `26..31 -> 28`, `32 -> 34`. After: each level to itself.
`darks.png` shows the first 64 levels (default film on top, linear light before, linear light after; brightened x4).

The failing tests before the fix (`cargo test --release -p turbomotion-comp color_darks color_flat`):

```
small shape: 26 levels are more than 1 off the level drawn: 2->0 3->0 4->0 5->0 6->0 7->13 8->13 9->13 10->13 11->13 15->13 16->13 17->13 18->22 19->22 20->22 24->22 25->22 26->28 30->28 31->28 32->34 36->34 40->38 44->42 48->50
small solid: (the same 26 levels)
half-alpha darks are not half the light: 8->6 9->6 14->6 15->6 16->6 17->6 18->13 19->13 20->13 24->13 25->13 26->18 27->18 31->18 32->22 36->22 40->25 44->28 48->34 57->38
stroke: level 8 shows 13 in light and 8 on encoded numbers
text: level 8 shows 13 in light and 8 on encoded numbers
particle: level 8 shows 13 in light and 8 on encoded numbers
```

After the fix: all pass (`a_small_solid_shows_the_level_it_is_drawn_in_at_all_256_levels` etc.). Unchanged and still passing:
the full-frame solids, the gradient (worst 0.56 level off the ideal ramp, largest step 12 against the ideal 11.1, the same
numbers as WP-03), the 50% white (188) and the encoded-number film.

Acceptance numbers of WP-03 still hold: 188 (50% white), 176 (add of two #808080), 181/192 (blur edge), gradient 0.56, 64
(contrast), luma matte 188, plugin 255/176 and 128: the `color_tests` / `color_media_tests` suites pass unchanged, and
`wp/WP-03/probes/comp_linear.json` renders **byte-identically** before and after (same sha, the probe is made of full-frame
layers and a gradient), so its recorded panel numbers (128 vs 188, 125/130 vs 186/189, 64/128 vs 138/188, 255 vs 176) hold.

### Probe films that changed (`identity/linear-before.tsv`, `linear-after.tsv`, stills at 0.5 s)

| film | changed | why |
| --- | --- | --- |
| `wp/WP-03/probes/comp_linear.json` | no | full-frame layers and gradients only |
| `wp/WP-05/probes/light.comp.json` | yes | the 224,48,128 disc is a small shape; its green (level 48) was quantised to 50 |
| `wp/WP-05/probes/old-modes-light.comp.json` | yes | small shapes in linear light (no recorded numbers; it is an identity input) |
| `wp/WP-03/fix-darks/*-light.comp.json` | (new probes) | |

`light.comp.json`, disc centre in the three panels: before (55,20,95) (226,135,208) (78,31,139); after (55,19,95)
(226,134,208) (78,30,139). The new green values are the closed forms of the light (multiply 18.8, screen 134.2, overlay 29.7
levels), so the old ones were products of the bug: WP-05's recorded numbers shift by one level in green and its tests (which
compute the closed form) are unaffected. `wp/WP-05/probes/{light.png,compare.png,README.md}` regenerated and updated with this
reason; `wp/WP-05/REPORT.md` is the delivered report and is left as written (it quotes the old green values).

## Byte identity of the default mode

Baseline: `git archive HEAD` of the engine extracted into an empty `/tmp/fix-base`, built there, deleted afterwards (no git
worktree). `identity/snap.sh` renders stills at 0.5 s and 1.2 s of every `examples/comp/*`, the launch film and components,
six default-mode probe films (WP-03 `comp_default`, WP-05 `old-modes`, `display`, `modes`, and the two default probes of this fix)
and builds ten SDK films (`sdk build`, hashing the JSON): **100 rows, `diff before.tsv after.tsv` empty.** `identity/video.sh`:
five examples x software and alpha codec, one worker, md5 of the video stream: **10 rows, `diff video-before.tsv video-after.tsv` empty.**

The first run of this check was not identical: one pixel (553,220) of one built film (`001-esports-momentum`, t = 0.5 s) was
one level off. The cause was my own: a particle's 8-bit colour went through `Color4f::new(c / 255.0, ..)` where Skia's
`set_color` uses `c * (1 / 255.0f)` (a different last bit of the float). `Color4f::from(Color)` is the same conversion; the
row is identical since.

## Memory and speed

No buffer is added: the pictures were and are RGBAF16, the fix only changes how a colour reaches them. `comp graph` of a
1080p linear-light film (`examples/comp/19-solids` with the colour key, t = 1.0): 10 passes, at most 1 picture alive besides the
composite, largest picture 1920x1080 = 16,588,800 bytes (8 bytes x 2,073,600 px), identical before and after (33.2 MB live).
Time, software codec, one worker, three runs each: `19-solids` 90 frames of 1080p 0.81-0.88 s before, 0.82-0.84 s after;
`01-shapes` 120 frames 720p 0.96 s before and after; `02-type` 120 frames 0.60 s before and after. The first run of each pair
is slower in both (cold start). Cost of an ink: one runtime-shader instance (a float4 uniform) per fill, stroke or glyph; none
measurable here.

## Determinism

Software codec, one worker, two renders each: swatch film frames 0..8 `98ad526ce0ea40c2ecd6664dfbf50f5e` both times;
`old-modes-light` frames 0..6 `1e0ba164c4b05c7d6ba63ad1dc05bf4a` both times; the swatch still `4a1ea31fc16eb96c1dcde180087d9d3aa73c1f98`
both times. The in-test render-twice (`rendering_twice_gives_the_same_bytes_...`) passes.

## Changed files (paths under `turbomotion/`; nothing committed)

- core: `crates/core/src/color/ink.rs` (new), `color/ink_tests.rs` (new), `color/mod.rs`, `color/tests.rs` (one helper made `pub(super)`), `shaders/ink.sksl` (new), `shaders.rs`
- comp: `crates/comp/src/render/{content,shape,text,particles}.rs`, `render/mod.rs` (two test modules), `render/color_darks_tests.rs` (new), `render/color_flat_tests.rs` (new)
- ledger/spec: `parity/specs/colour_pipeline.md`, `parity/specs/blend_modes.md` (the known-limit bullet for this bug removed), `parity/MATRIX.tsv` (two rows)
- evidence: `wp/WP-03/FIX-darks.md`, `wp/WP-03/fix-darks/{make.py,run.sh,levels.py,*.comp.json,before/,after/,darks.png,identity/}`, `wp/WP-05/probes/{light.png,compare.png,README.md}`
- no generated doc page changed (`docs --write` rewrote the same 8 pages), `scripts/` and `motions/` untouched. File sizes: no file touched grew past 300 lines (new files 39, 71, 145, 78 lines; `content.rs` 87, `shape.rs` 135, `text.rs` 181, `particles.rs` 105); `render/mod.rs` gained two `mod` lines (it was 250).

## Tests added (15)

- core `color/ink_tests.rs`: `every_level_painted_into_linear_light_arrives_as_the_light_the_curve_says` (all 256 levels on a half-float GPU surface, within 2/2048 of the curve's light), `alpha_is_coverage_and_scales_the_light`, `a_picture_that_is_not_tagged_takes_the_colour_as_the_paint_always_did`
- comp `render/color_darks_tests.rs`: `a_small_solid_shows_the_level_it_is_drawn_in_at_all_256_levels`, `a_small_shape_shows_the_level_it_is_drawn_in_at_all_256_levels` (each level within 1, levels 1..32 individually, strictly rising), `a_small_solid_is_the_same_on_encoded_numbers_as_it_was`, `a_full_frame_solid_shows_the_level_it_is_drawn_in`, `half_covered_white_is_half_the_light_whatever_the_size_of_what_covers_it` (188), `half_covered_dark_greys_add_their_light_from_black_in_proportion`, `a_gradient_across_a_small_layer_follows_the_light_ramp_without_banding`
- comp `render/color_flat_tests.rs`: `the_stroke_of_a_shape`, `the_letters_of_a_text_layer`, `the_sprites_of_a_particle_layer`, `the_background_of_the_composition`, `the_faces_of_a_mesh` (levels 8, 16, 48 in light against the default film)

## Gate output

1. `cargo test --release --workspace --no-fail-fast` -> exit 0; 40 test suites, **605 passed, 0 failed, 2 ignored** (the two existing ignores; before: 590 passed, so +15). Tail: `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s` (the doc-tests of `turbomotion_core`).
2. `cargo run --release -p turbomotion -- docs --write` -> `wrote 8 pages to docs/reference`; `docs --check` -> `docs are current (8 pages in docs/reference)`; `git diff --stat docs/` empty (no page changed).
3. `python3 scripts/capability_index.py` -> `motions/lib/CAPABILITIES.md: 712 entries`; `find linear_light` -> `comp-key | color.linear_light | true keeps pictures in linear light (default false: display-encoded numbers). |`; `python3 -m unittest discover -s scripts -p 'test_*.py'` -> `Ran 47 tests in 2.235s` `OK`.
4. `turbomotion parity check` -> `parity check: ok (79 core rows, 264 effect rows, 97 catalogue effects)`, exit 0 (was 77 core rows).
5. `comp check` of every probe film (the five of this fix, `comp_linear`, `light`, `old-modes-light`): each `ok: ... 0 assertions; 0 errors, 0 warnings`.
6. Determinism: see above.
7. Probe images: `fix-darks/darks.png` (the first 64 grey levels: the default film, linear light before the fix where levels 1..6 are black and 7..17 are one grey, and linear light after the fix, which is the default film's ramp); `wp/WP-05/probes/light.png` and `compare.png` regenerated.

## Deviations from the brief

- The brief's likely cause (an intermediate 8-bit surface) is not the cause; the evidence is above. The fix is not a surface-format change because no format was wrong.
- Layer **opacity** is carried by the same 8-bit mechanism and is not fixed (below).
- The default-mode path of `Ink::apply` sets the paint colour as before (untagged canvas). That is a branch, but it is the byte-identity requirement itself: an untagged picture is not converted, so the colour is exact in 8 bits.
- Packs painting their own colours (`PackCtx` gives them `like` only) are not routed through `Ink`; no pack is registered in this repository besides the one in the pack tests, which clears with a float colour.
- `parity/specs/blend_modes.md`: the "Known limits" bullet about this bug is removed (it no longer holds).

## Open questions for the owner

1. Layer opacity in linear light is quantised to 1/255 (white at 0.2% shows level 13 where the light says 6.6, 1% shows 28 against 25.5). The default film has the same step, which costs it far less. Moving opacity to a float needs `blend::prepare`, `draw_warped`, `over`, the shutter average and the native-mode blends to take it as a uniform or colour-filter scale instead of the paint alpha (the shader blends and the dissolve already treat it as a uniform), and the default path must stay as is for byte identity. Ledger row added as `part`. Should it be its own WP (before halation/bloom, which fade low-opacity layers)?
2. Packs (`Pack::paint`) draw with their own Skia calls. Should `PackCtx` offer `Ink` so an external pack's flat colours get the same treatment?
3. Images are 8-bit sources converted in the shader at draw time; a bilinear or mipmapped sample of an sRGB-tagged 8-bit texture averages the encoded values before conversion (WP-03 deviation 2). Not touched; do you want light-correct resampling of images (half-float import, more memory) before a glow/bloom WP?
4. WP-05's report quotes the old green values of the light probe (20, 135, 31). Left as delivered; say if the report should be amended.

Estimated cost to finish (if partial): none, status done (question 1 would be about 4 hours).
