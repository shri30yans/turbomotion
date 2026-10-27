# Radial Blur and Directional Blur (`radial_blur`, `directional_blur`)

Written in my own words (BUILD_PLAN section 4, step 2). **No After Effects dump is available.** Both effects keep the original sampled shader
as `quality: legacy` (the default, bit for bit) and gain `quality: exact`, the kernel's line integral (`kernel::Smear`,
`shaders/kernel/smear.sksl`): the mean of the picture along a path, one sample per pixel of path (`MAX_SMEAR_TAPS` 2048), centred on each pixel.

* `directional_blur` smears `distance` pixels along `angle`. Exact: centred on each pixel (a box of that length along the line, bilinear).
* `radial_blur` with `mode: zoom` smears each pixel along its ray from `(x, y)` out to `1 + amount` times; with `mode: spin` (exact only; legacy is
  an error that says so) it smears along the circle about `(x, y)` through `amount` degrees, centred on the pixel.
* The centre is a fraction of the frame, mapped into the layer's own pixels, so a spin turns about the same point wherever the layer sits
  (`a_spin_turns_about_the_centre_of_the_content_wherever_the_layer_sits`).
* **Reach** (planner): half the path each way plus the pixel its bilinear read adds; a spin's reach follows its farthest corner. A path longer than
  the pass's 2048 samples at its worst pixel is an error naming the limit.

## Measured (`parity/probes/radial_blur.json`; `wp/WP-02/probes/data.tsv`)

| quantity | measured | closed form |
| --- | --- | --- |
| spin 20 degrees, 4 x 4 square at 200 px: radius spread, angle extent, light | 4.9 px, 21.0 deg, 1.004 | about 4 px, 20 + source width (1.1 deg), 1 |
| spin 60 degrees | 4.9 px, 60.9 deg, 1.011 | about 4 px, 61.1 deg, 1 |
| unit tests: a line smear is a box the length of the path; a spin moves a point along its circle and a zoom along its ray | `shape_tests.rs` | |
