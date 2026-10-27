# Unsharp Mask (`unsharp_mask`)

Written in my own words (BUILD_PLAN section 4, step 2). **No After Effects dump is available**; defined, then measured
(`effects/blur_tests.rs`, `parity/probes/unsharp_mask.json`).

## What it does

`out = in + (amount / 100) * (in - blur(in))` per colour channel, where `blur` is the engine's exact Gaussian of sigma `radius` with the
edge **repeated** (so the layer's own border is not mistaken for a detail). A channel whose difference is under `threshold` levels (of 255) is
left alone. The colour is clamped to `[0, alpha]`; **coverage (alpha) is never changed**, so the picture's silhouette stays where it is.
Encoding `Display`: the sharpening is in the film's own numbers, in a linear-light film too (a halo of light would overshoot differently).

## Parameters

| name | unit | default | range |
| --- | --- | --- | --- |
| `amount` | percent | 50 | 0 to |
| `radius` | px, Gaussian sigma | 1 | 0 to 100 |
| `threshold` | levels of 255 | 0 | 0 to 255 |

`@bounds none`: it reads the pixels it sharpens and adds none.

## Measured (`wp/WP-02/probes/data.tsv`)

| quantity | measured | closed form | asserted |
| --- | --- | --- | --- |
| edge 100 to 160, sigma 2, 4 at 100%, sigma 4 at 200% | 0.6, 0.53, 0.65 levels off | `v + a (v - 100 - 60 Phi(d / sigma))` | 1 level |
| flat grey 100, amount 300 | 0 pixels change | no detail, no change | exact |
| coverage | unchanged | | `unsharp_adds_back_a_fraction_of_the_detail_and_leaves_flat_ground_and_coverage_alone` |
