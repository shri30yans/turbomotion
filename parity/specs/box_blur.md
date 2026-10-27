# Box Blur (`box_blur`)

Written in my own words (BUILD_PLAN section 4, step 2). **No After Effects dump is available**; every rule is a definition implemented in
`core/src/kernel` and measured on the engine's own pixels (`kernel/shape_tests.rs`, `effects/blur_tests.rs`, `parity/probes/box_blur.json`).

## What it does

Every pixel becomes the mean of the `2 r + 1` pixels centred on it along each axis (two separable passes of `shaders/kernel/separable.sksl`,
the same shader as the Gaussian with flat weights), repeated `iterations` times. It owns no blur of its own: it is `kernel::Shape::Box`.

* **Fractional radius.** The last tap has the weight of the remainder (`taps::box_taps`), so a radius of 3.5 is half way between 3 and 4 and an
  animated radius does not step. A radius of 0 is the identity.
* **Iterations.** The variance of `n` boxes is `n` times one box's, `(w^2 - 1) / 12` with `w = 2 r + 1`; three or more is a close bell
  (`repeated_boxes_widen_as_the_variances_add`). Radius 6 and 3 iterations is a Gaussian of sigma 6.48; their edge profiles are at most 10.6
  levels apart (probe), so it is *near* a Gaussian, not one. 1 to 16 iterations (`kernel::MAX_ITERATIONS`); more is an error.
* **Reach** is `iterations * radius` pixels each side (rounded up), the pixels the passes read; a layer in layer space grows by exactly that
  (`a_box_and_a_lens_reach_the_pixels_they_read`).
* `dimensions` (`both`, `horizontal`, `vertical`) and `edge` (`transparent`, `repeat`) as in `gaussian_blur.md`.

## Parameters

| name | unit | default | range |
| --- | --- | --- | --- |
| `radius` | px each side | 5 | 0 to 2048 (`taps::MAX_TAPS`; more is an error naming it) |
| `iterations` | | 1 | 1 to 16 |
| `dimensions` | choice | `both` | |
| `edge` | choice | `transparent` | |

Encoding `Linear`. No `quality`: this effect exists only as the kernel.

## Measured (`wp/WP-02/probes/data.tsv`, 8-bit stills of `parity/probes/box_blur.json`)

| quantity | measured | closed form | asserted |
| --- | --- | --- | --- |
| hard edge, radius 6, one iteration, linear light | 0.4 levels off the ramp | `clamp((d + 6.5) / 13)` | 1 level |
| 8 x 8 square, radius 6: light | 0.9988 | 1 | 0.5% (float) |
| same: lit width | 20 px | 8 + 2 * 6 | exact |
| flat top: a box's pixels are equal to the pixel | | | `a_box_is_flat_over_exactly_its_width_and_keeps_its_light` |
