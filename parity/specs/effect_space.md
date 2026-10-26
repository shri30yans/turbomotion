# Effect space (WP-01)

Where a layer's effects and masks run. Comp key `"effect_space": "screen" | "layer"`, default `"screen"`.

## Step 1: the clipping symptom, reproduced

Measured on the engine as it was at commit 5dff2ab (the same numbers come out of `effect_space: "screen"` today, which is
that pipeline byte for byte). Tests that keep them true: `render/layer_space_tests.rs`, `render/layer_space_plugin_tests.rs`.

**Which buffer, which size.** `render/post.rs::paint_stage` made one picture of `frame.width x frame.height` (half-float
RGBA, 1920x1080 = 16 MB) for every layer and `render/content.rs::paint_content` drew the layer into it with its whole
transform applied. Masks, effects and mattes then ran on that picture. There was no layer-sized buffer at all.

**Which `resolution` the shader sees.** The frame's: `uniform float2 resolution` was the picture's size, scaled by the pass
scale, whatever the layer was. `xy` was a frame pixel. `tests: in_screen_space_a_plugin_sees_the_whole_frame...` pins
resolution = buffer = 200 in a 200 px frame for a 20 px layer.

**Symptoms, with numbers.**

| what | screen space (before) | layer space (now) |
| --- | --- | --- |
| `blur` radius 10 on a 4x4 square: pixels lit (>= 3/255) each side of the centre, scale 1 | 15 | 15 |
| the same at layer scale 2 (an 8 px square) | 22 | 30 (2 x 15) |
| `blur` radius 10 on a 60 px square, tail past the edge, scale 1 and scale 2 | 22 and 22 | 22 and 45 |
| a 120 px square, centre 75 px left of the frame, `glow` radius 18: pixels the glow lights inside the frame | 0 | 29 columns |
| `glitch_slices` (`amount` 36) on 64 px text: width of what is drawn in the 250 px cell | 179 | 249 |
| `directional_blur` 70 px, angle 0, on a bar rotated -35: lit box | 166 x 76 (along the frame's x) | 152 x 118 (along the bar) |
| a plugin that moves pixels 30 px right, layer 20 px wide, no `@bounds` | shows at x 120..140 | stays at 90..110 (cannot leave its 20 px picture) |
| memory for one 40 px layer with a radius 10 blur at 1920x1080 | 16.6 MB (frame) | 100 x 100 x 8 = 80 KB |

**Mechanism of the "glyphs clipped to the layer bounds" symptom.** It is not reproduced in screen space: nothing is cut to
the glyphs, because the picture is frame sized. What does happen there is (1) anything the transform moves outside the
frame is gone before an effect can read it, (2) `glitch_slices` computes its bands from `resolution.y`, which is the frame's
height, so a line of text is two bands of ten rather than ten bands (`probes/layer_space_probe.png`, column 4), and (3)
pixel parameters stay in screen pixels. Layer space moves the clip to where it belongs: a layer-space buffer is the
content box grown by what the effects declare (`@bounds`), so a plugin that spreads and says nothing is cut at the content
box (the last row of the table), which is what the `fx-needs-bounds` lint reports.

## Design

* **Layer buffers.** `eval::buffer::LocalBuffer`: the content box (`local_bounds`, in layer pixels: solids their size, text
  the measured box, shapes the path bounds, images their fitted size, precomps their comp size, XX
  declared box) grown by the chain's growth and rounded out. `PaintLocal` draws content with the identity transform into
  it. Limits: 16384 px a side, 64 Mi pixels; past either, `check` fails naming the layer and what to lower.
* **Growth.** Platform effects: `Effect::grows(t, content)` (Gaussian 3 sigma for blur, glow, shadow; half the smear along
  its angle; disc reach for twirl and bulge; the transform's bounding box). Plugins: `@bounds none | expand=<expr> |
  expand_x=<expr> expand_y=<expr> | source=<input>` (numbers, float parameters, `+ - * /`, parentheses, `abs`, `min`, `max`;
  `source=` is parsed and reserved). Growth is summed along the chain over the running box. It is conservative: a larger
  buffer than needed is correct, a smaller one is the clip.
* **Plugin geometry.** `xy` counts from the content box's top-left, `resolution` is the content box (scaled by pass scale),
  `layer_origin` is the buffer's top-left relative to the content box (negative when it grew up or left) and `buffer_size`
  its size. `in_buffer(p)`, `in_buffer_x(x)` clamp to the buffer. In screen space `layer_origin` is 0 and `buffer_size` is
  `resolution`, so a shipped plugin behaves exactly as before.
* **Order.** content, effects, masks (all in layer pixels) | transform (`Place`, or fused into the composite) | lighting and
  the blur or smear of a transition (`Finish`) | `clip_to`, matte, reflect and wipe, which are defined on the frame.
* **Placing.** Catmull-Rom when the layer is not shrunk (scale >= 0.999), bilinear with mips when it is, through
  `concat_44` of the layer's 4x4 matrix so 3D layers are placed with perspective.
* **What stays in screen space** even under `"layer"`: layers with neither effects nor masks (nothing to run before the
  transform), adjustment layers and `look` (they work on the picture below). **What is refused** by `check`, naming the
  effect and `"effect_space": "screen"`: `backdrop`, a mesh with effects or masks, a composite mask taken from another layer,
  plugin inputs that read another layer or the layer's history.

## Tests

`render/layer_space_tests.rs` (tails, off-screen, rotation, determinism, default identity, mask), `eval/buffer_tests.rs`
(buffer size equals the bounds-grown layer size), `render/graph/layer_space_tests.rs` (stages, peak images),
`render/layer_space_plugin_tests.rs` (what a plugin is told), `plan/layer_space.rs` (refusals), `tests/effect_space_schema.rs`
(key, default, did-you-mean), `crates/script/tests/{sdk,sdk_option_types}.rs` (SDK typing), `core/src/effects/bounds.rs`.
