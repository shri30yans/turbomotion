# Gaussian Blur (`blur`)

Written in my own words (BUILD_PLAN section 4, step 2). **No After Effects dump is available, so nothing below is a measurement of
After Effects.** Every rule is a definition implemented in `core/src/kernel` and then measured on the engine's own pixels
(`kernel/tests.rs`, `kernel/shape_tests.rs`, `effects/blur_tests.rs`, `render/blur_tests.rs`); the places where I chose are marked `UNSURE`.

## Audit: what the blur was, and where it lost light

The engine's one blur was `Layer::blurred`, Skia's image filter. It samples a Gaussian cut at 3 sigma and is exact to a sigma of about 4;
above that Skia works on a reduced-resolution copy, so its light depends on the **sigma and on the size of the picture** and a small source
can gain or lose it. Measured as the light of a lone pixel after the blur (1.000 is correct):

| picture | sigma 4 | 5 | 6 | 8 | 12 | 16 | 24 | 32 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 161 px square | 1.000 | 0.879 | 0.56 to 0.84 | 0.000 | | | | |
| 256 px square | 1.000 | 1.20 | 1.27 | 1.00 | 1.27 | 1.00 | 1.27 | 1.00 |

(`the_legacy_blur_counts_the_light_of_a_small_source_by_its_sigma_and_by_the_size_of_the_picture`.) A point source's halation halo peaks
at 0.98 of the analytic Gaussian at sigma 16, 32 and 64. `glow`, `bloom`, `shadow`, `film_halation`, `backdrop` and the `tilt_shift` plugin
were all built on it. The reach it was grown for (`3 * sigma`) is also not what a kernel needs to be exact.

## The exact kernel

One mechanism, `kernel::Blur { shape, edge }`, which every blur-like effect calls and which owns the only blur next to Skia's (the legacy path
is Skia's, kept as it was):

* **Gaussian.** Two separable passes (`shaders/kernel/separable.sksl`), one along x and one along y, each a weighted mean over the taps
  `exp(-i^2 / 2 sigma^2)` for `i` to `ceil(4 sigma)` each side. The mean divides by the sum of the weights it actually read, in 32-bit
  float, so the weights add to exactly 1 whatever the sigma, and no tap is skipped by a reduced-resolution trick. The cost is linear in the
  sigma (a sigma of 64 at 1080p takes about 17 ms; see the performance table in `wp/WP-02/REPORT.md`), so no pyramid is built.
* **Precision.** The weights and the sums are 32-bit float; the picture between the two passes is half-float, as every picture is. A smooth
  256-step gradient stays monotone with no step larger than the ideal (`a_smooth_gradient_stays_monotone_and_no_step_is_larger_than_the_ideal`).
* **Reach** is 4 sigma (`taps::REACH`): what is left past it is 6.3e-5 of the weight, under a hundredth of a level on a black and white edge
  (`the_cut_leaves_less_than_a_hundredth_of_a_level_on_an_edge`), and the weights are renormalised so no light is lost either way. The planner
  and the shader read the same function (`taps::gaussian_taps`), so a layer's buffer grows by exactly what the kernel reads
  (`an_exact_blur_grows_the_layer_by_the_pixels_its_kernel_reads_and_a_legacy_one_by_three_sigma`).
* **Limits are errors.** A sigma above 512 (`taps::MAX_TAPS`, 2048 taps each side) is refused with the limit named when the film is built;
  nothing is narrowed (`a_sigma_too_wide_for_the_pass_is_an_error_that_names_the_limit_and_nothing_is_narrowed`).

## Parameters

| name | unit | default | range | notes |
| --- | --- | --- | --- | --- |
| `radius` | px | 8 | 0 to 512 with `quality: exact`; no limit with `legacy` | Gaussian **sigma** |
| `quality` | choice `legacy`, `exact` | `legacy` | | `legacy` is the blur films were made with, bit for bit; `exact` is the kernel |
| `dimensions` | choice `both`, `horizontal`, `vertical` | `both` | | AE's Blur Dimensions; honoured by both qualities |
| `edge` | choice `transparent`, `repeat` | `transparent` | | AE's Repeat Edge Pixels; honoured by both qualities |

`radius` animates (it is read at the frame's time). The effect's encoding is `Linear`: in a `linear_light` film it mixes light, in a display
film it mixes the film's own numbers, as every effect of that encoding does.

**Edges.** `transparent` treats everything past the picture as transparent black, so light leaves the picture and the layer's buffer grows
by the reach (in layer space the tail is kept; in screen space it is clipped by the frame). `repeat` continues the outermost pixels outward,
so a flat picture stays flat to its edge and the layer does **not** grow (`a_blur_that_repeats_the_edge_keeps_a_flat_picture_flat_where_a_transparent_one_fades_it`).

## AE radius to sigma (`UNSURE`)

After Effects' Blurriness is not a sigma. I **chose** `sigma = blurriness / 2`: Blurriness reads as the visible width of the blur, and a
Gaussian that is visible out to about two sigma each side has that width. Nothing here checks it. What would settle it is a dump of AE's own
edge profile at a known Blurriness (the sigma of an error function is its 10% to 90% width divided by 2.563). Until then a film that wants
AE's Blurriness `b` writes `radius: b / 2`, and the SDK does not convert.

## Measured

| quantity | measured | closed form | asserted |
| --- | --- | --- | --- |
| impulse standard deviation across and down, sigma 1, 2, 4, 8, 16, 32, 64 | asserted | sigma | 3% |
| light of an impulse, display numbers and linear light, every sigma above | asserted | 1 | 0.5% |
| hard black/white edge, sigma 4, 8, 16, 24, display film | 0.65, 0.46, 0.51, 0.55 levels | `255 Phi(d / sigma)` | 1 level |
| same, `linear_light` film | 0.73, 0.52, 0.55, 0.56 levels | `encode(Phi(d / sigma))` | 1 level |
| smooth gradient | monotone | | no step larger than ideal |
| animated radius | read at the frame's time | | |
| layer scale 2 in layer space | the tail is twice as long | | 3 px |
| halation point-source peak, sigma 16, 32, 64 | 1.0006, 0.9990, 1.0053 | analytic Gaussian | 1% |

The legacy numbers on the same probe (`wp/WP-02/probes/data.tsv`): edge within 0.92 levels (display) and 3.26 (linear light), a lone 4 x 4
square at sigma 12 returns 1.263 of its light.
