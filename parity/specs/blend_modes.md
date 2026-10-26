# Blend modes (WP-05)

Layer key `"blend": "<mode>"` and an effect's `composite.blend`. 37 modes plus the alias `linear_dodge` (= `add`). Default
`normal`. Every film that does not name a new mode is rendered by the code that rendered it before: the original seventeen
are drawn by the same Skia modes (`wp/WP-05/identity/`: stills of every example and built film, before and after tables
identical).

## One mechanism

`turbomotion_core::effects::blend`. A mode is a `Blend` enum value; its row in `table.rs` (`MODES`) is the only place that
says anything about one mode in particular: name, AE menu group, summary, how it is drawn, and whether it must be a
picture of the whole frame. The parser, the schema's list of names and did-you-mean, the catalogue, the docs page
(`docs/reference/blend-modes.md`), the SDK type test and the choice of how to draw all read that table. `Blend::stage` is
the single way a mode is drawn: layers on the composite (`render/graph/run.rs`) and an effect's result on its input
(`render/composite.rs`) both call it through `render/blend.rs`.

| how | modes |
| --- | --- |
| Skia's own mode (the paint carries it) | the original 17, `stencil_alpha` (DstIn), `silhouette_alpha` (DstOut), `luminescent_premul` (SrcOver) |
| the shared blender `shaders/blend.sksl` (one runtime blender, the mode is a uniform number, numbered by table order) | `darker_color lighter_color linear_burn classic_color_burn classic_color_dodge linear_light vivid_light pin_light hard_mix classic_difference subtract divide stencil_luma silhouette_luma alpha_add` |
| a shader pre-pass `shaders/dissolve.sksl` | `dissolve dancing_dissolve` |

## What the numbers are

A mode is a function of the numbers the pictures hold. Default film: display-encoded sRGB numbers, which is After Effects'
8/16-bit project. `color.linear_light`: light, so the same mode computes on light (WP-03, `colour_pipeline.md`). Nothing in
the blend code knows which: the surfaces are tagged and Skia converts on draw. Measured in `blend_tests.rs` with a
backdrop (64,128,192) and a layer (224,48,128): multiply differs between the two by up to 5.4 levels, screen by 17.7,
overlay by 53.2, each equal to the sRGB-curve prediction within one level.

The layer's opacity scales its (premultiplied) colour and alpha before the mode sees it.

## Definitions

Colours below are straight (un-premultiplied), clamped to 0..1, per channel unless stated; `cb` is the base (what is below),
`cs` the layer, `ab`, `as` their alphas. The modes that mix colours (everything except the stencils, silhouettes, `add`,
`alpha_add`, `luminescent_premul` and the classics) are laid over by the W3C rule:

    out_premul = cs*as*(1-ab) + cb*ab*(1-as) + B(cb, cs)*as*ab        alpha = as + ab - as*ab

Where one alpha is 0 the other picture is the result. W3C Compositing and Blending Level 1 gives B for multiply, screen,
overlay, soft light, hard light, darken, lighten, color dodge, color burn, difference, exclusion and the four
non-separable modes (Lum = 0.3 R + 0.59 G + 0.11 B, SetLum, ClipColor, SetSat as published). The rest:

| mode | B(cb, cs) |
| --- | --- |
| `add` (= `linear_dodge`) | premultiplied sum, clipped at 1 (Skia Plus) |
| `darker_color`, `lighter_color` | the whole colour (`cs` or `cb`) with the smaller / larger sum R+G+B; a tie keeps `cb` |
| `linear_burn` | max(cb + cs - 1, 0) |
| `linear_light` | clamp(cb + 2 cs - 1, 0, 1) |
| `vivid_light` | cs <= 0.5: color burn(cb, 2 cs); else color dodge(cb, 2 (cs - 0.5)) |
| `pin_light` | cs <= 0.5: min(cb, 2 cs); else max(cb, 2 cs - 1) |
| `hard_mix` | 1 if cb + cs >= 1 else 0 |
| `subtract` | max(cb - cs, 0) |
| `divide` | cs = 0: 1 if cb > 0 else 0; else min(cb / cs, 1) |

Color dodge and color burn are Skia's, which are W3C's (`cb = 0` gives 0 for dodge, `cb = 1` gives 1 for burn, the
division clamped at 1).

**Classic modes** (`classic_color_burn`, `classic_color_dodge`, `classic_difference`): the layer is first faded toward the
colour that leaves the base alone (1 for burn, 0 for dodge and difference) by its alpha, `cs' = n + as (cs - n)`, then the
formula runs at full strength on the opaque result: `out = cs*as*(1-ab) + ab*F(cb, cs')`, `F` as the non-classic mode but
with black burning to black (burn: cs' = 0 gives 0) and white dodging to white (dodge: cs' = 1 gives 1). The classic modes
differ from the new ones exactly where alpha or opacity is below 1 and at the black/white edges (test
`the_classic_modes_differ_from_the_new_ones_at_the_edges_and_under_opacity`). The fade rule is EffectCraft's
(`tmp/clones/effectcraft/crates/color/src/blend.rs`, issue #494, measured by its reporter against After Effects at 32 bpc);
I could not measure After Effects myself.

**Stencil and silhouette.** `stencil_alpha`: out = below * as. `silhouette_alpha`: out = below * (1 - as). `stencil_luma`:
out = below * luma, `silhouette_luma`: out = below * (1 - luma), luma = Rec. 709 weights (0.2126, 0.7152, 0.0722) of the
layer's premultiplied colour, clamped to 0..1 (so a transparent layer has luma 0). All four act on everything below,
including the composition's `bg`. A stencil therefore clears the whole frame outside the layer (as = 0, luma = 0), which is
how I read After Effects' stencil (UNSURE, not measured); the layer must be a picture of the whole frame, which the compositor arranges (`Mode::whole_frame`,
a layer-space layer is placed first). A silhouette leaves what is outside the layer alone. The frame is shown over black,
so a cleared pixel shows black.

**`alpha_add`**: alpha = min(as + ab, 1) instead of the union; colour is the over colour rescaled to keep its strength:
two abutting half-covered edges of one colour make a fully opaque seam-free pixel where `normal` leaves a dip.

**`luminescent_premul`**: AE's mode for premultiplied light whose colour may exceed its alpha. The engine's pictures are
half-float premultiplied and unclamped, so such light already survives a plain over: the mode is `normal`, listed so films
that name it load, and the test `luminescent_premul_keeps_light_above_alpha_that_normal_would_clip_to_it` pins that.

## Dissolve and dancing dissolve

Each pixel is kept whole (as its straight colour, alpha 1) or dropped, never partly mixed. A pixel is kept when
`pattern(x, y, seed) < alpha * opacity`, so over a large flat area the kept share is that product. `pattern` is a function of
the pixel and the seed only: a Feistel network on two 12-bit halves with whole-number arithmetic that is exact in a 32-bit
float (every product below 2^24, every remainder by a power of two). SkSL for runtime effects has no unsigned integers and
no bit operators, which is why it is built so. Nothing is rounded, so the pattern is bit identical on every backend and on
repeated renders. `dissolve`: seed 0. `dancing_dissolve`: seed = the frame number, `round(t * fps)` of the composition being
drawn (so frame 0 is the plain pattern). The pattern is made at frame size, so a dissolve is placed first like a stencil.

Measured on a 512 x 512 flat area (262144 independent pixels; the standard deviation of a share is at most 0.001):
the kept share is within 0.0045 of the opacity at 0.05 .. 0.95; every 128 x 128 block of it within 0.016; pixels 1, 2, 3, 5,
8 and 64 apart (rows and columns) agree as often as chance says within 0.0045, as do frames 100 / 101 / 102 / 1000 / 70000.

## Tolerances

* Closed form (core, 567 pairs of straight colours and alphas, every mode, 32-bit float raster): worst error 3.8e-6
  (Skia's color dodge and burn, which divide by a small difference). Bound 1e-5.
* On the GPU through the whole renderer (half-float pictures, 8-bit output): worst 0.88 level of 255 over every mode, two
  colour pairs, two opacities. Bound 1 level: half-float steps of 2^-11 and rounding to a level.
* The test colours are multiples of 1/16 (core) or 8-bit levels with no sum of two channels equal to 255 (comp), so a mode
  that compares a sum with a threshold breaks no tie differently from the closed form.

## Where this differs from EffectCraft's `color/src/blend.rs`

That file is the plan's source for the formulas; it works on 32 bpc values. This engine's pictures are half-float and its
modes are the 8/16 bpc modes, so:

* `darker_color` and `lighter_color` compare the sum of the channels (Adobe's description of the mode: "compares the total
  of all channel values"), EffectCraft compares Rec. 709 luma. Test
  `darker_and_lighter_colour_compare_the_sum_of_the_channels_not_the_luma` holds the choice.
* `alpha_add` rescales the over colour to the new alpha, so two abutting edges of one colour close to that colour;
  EffectCraft leaves the premultiplied over colour, which darkens the seam to 0.75 of the colour for two half-covered edges.
* `divide` and the classic burn are clamped to 0..1 (EffectCraft keeps values above 1 and negatives).
* The inputs of every new mode are clamped to 0..1; EffectCraft lets some modes see values above 1 (HDR). The old
  seventeen are Skia's, unchanged.

All other modes agree with it: dissolve (kept when `pattern < alpha`, as the straight colour at alpha 1), stencils and
silhouettes, `luminescent_premul` (= over), the classic fade, linear light, vivid light, pin light, hard mix, subtract.

## Known limits

* Luma stencils and silhouettes weigh the working-space numbers, not display-referred ones (a luma matte, `matte` mode
  `luma`, reads display numbers). In a linear-light film a mid grey stencil keeps less than half.
* `divide` by zero follows EffectCraft (a black base stays black, a lit base becomes white); After Effects' own result is UNSURE.
* A sized solid or shape drawn in a linear-light film is quantised to 8 bits of light in the darks before it is blended
  (level 8 and 16 both show 13, 48 shows 50): a WP-03 property found while testing this, not changed here. Full-frame
  solids are exact, which is what the linear-light tests use.
