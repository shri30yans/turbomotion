# Output dither (`color.dither`)

Written in my own words (BUILD_PLAN section 4, step 2). Not an After Effects effect: it is the choice of how the finished frame
is rounded to 8 bits. Implemented in `core/src/dither/` and `core/src/shaders/dither.sksl`; key `color.dither` (root only).

## Where it sits

Every picture in the engine is half-float. The finished frame is rounded to a byte per channel **once**, when `Renderer::draw_state`
draws the composite into the BGRA8888 frame surface (after the effects, the `look`, and the output profile's conversion). With no
dither (default) that draw is Skia's own round-to-nearest and none of the new code runs: **every existing film is byte-identical**.
With a dither the same draw goes through `dither.sksl`, whose picture child shader Skia converts into the output profile first, so the
numbers it sees are the numbers the file will hold.

## Rule

`out = floor(v * 255 + t) / 255`, `t = (rank + 0.5) / 256`, `rank` the pixel's entry in a tiled pattern, `v` the converted channel,
alpha forced to 1 (the frame is opaque over black). Every rank 0..255 appears equally often in a tile, so the mean threshold is exactly
0.5 (asserted exactly) and `out - 255 v` lies strictly within (-1, 1) with mean error under 1/512 level.

| value | pattern |
| --- | --- |
| `none` (default) | no dither: round to nearest |
| `ordered` | the 16 x 16 Bayer matrix, 256 ranks once each |
| `blue_noise` | a 64 x 64 void-and-cluster tile (Gaussian sigma 1.5, torus, fixed xorshift seed), ranks `r / 16`, each used 16 times; the tile is pinned by checksum so it is the same on every build |

The pattern is **static** (the same every frame): a moving pattern would need a frame number in the render, and a static one compresses
better. Open question in the report.

## Measured (`core/src/dither/tests.rs`, `render/dither_tests.rs`, `wp/WP-33/finish/dither-table.md`)

A dark ramp, level 5.1 to 15.3 over 2048 px, 64 rows, judged on 16-column slabs (1024 px):

| rounding | pixel levels | distinct slab means | max step between slabs | max slab-mean error | mean error | max pixel error |
| --- | --- | --- | --- | --- | --- | --- |
| none | 11 | 19 | 1.000 | 0.463 | -0.004 | 0.501 |
| ordered | 12 | 128 | 0.102 | 0.016 | -0.0001 | 0.999 |
| blue_noise | 12 | 128 | 0.088 | 0.009 | 0.0002 | 0.997 |

(The ramp itself climbs 0.08 level per slab.) The blue-noise tile's 8 x 8 block means vary at 1/12.6 of white noise's variance.

**Flat colours.** `floor(k + t)` is `k` exactly, but a half-float holds `k / 255` to `k / 2048` levels, so a threshold inside that gap moves
a pixel one level: at most `k / 2048 + 1/256` of the pixels of a flat colour (asserted at levels 0, 1, 17, 64, 128, 200, 254, 255; black
and white are exact). This is why dither is opt-in.

## Limits (the encoder boundary)

`wp/WP-33/finish/codec-table.md` decodes the luma of the dark ramp after each encode. libx264 lossless (`--crf 0`, `yuv420p`): the dither
survives in part (max step 0.41 against 1.0 for none; the limited-range 4:2:0 conversion re-quantises it). libx264 `--crf 18` and
`h264_videotoolbox`: **the dither is gone** (the numbers equal the undithered ones). A PNG still keeps it entirely. So for a delivered
H.264 the cure for banding is a **10-bit export** (ProRes 4444 is 10-bit, but the frames reach the encoder as 8-bit BGRA); deferred.
