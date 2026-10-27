# Add Grain (`add_grain`)

Written in my own words (BUILD_PLAN section 4, step 2). Source: the After Effects help text for Add Grain as I remember it
(Preset, Tint, Intensity, Size, Aspect Ratio, Softness, Saturation, Shadows/Midtones/Highlights, Animation, Seed), and
`tmp/clones/effectcraft/crates/effects/src/noise.rs` (read only, a hypothesis, nothing copied). **No After Effects dump is
available, so nothing below is a measurement of After Effects.** Every rule is a **definition** implemented and then measured
on the engine's own pixels (`render/grain_tests.rs`); the places where I chose are marked `UNSURE`.

## What it does

Adds a zero-mean, seeded, spatially correlated noise to the layer's colour, strongest where the tone response says, scaled by
coverage so a transparent pixel stays transparent. It is a plugin (`fxlib/builtin/add_grain.sksl`), `@space display`,
`@bounds none`.

## Parameters

Strengths are in **display numbers** (what the eye sees), also in a `linear_light` film: the effect declares `@space display`,
so the engine converts the picture to display numbers around it, and one `intensity` is the same grain in both modes
(`a_linear_light_film_gets_the_same_grain_because_the_strength_is_in_display_numbers`: std ratio 0.9999).

| name | AE name | unit | default | range | notes |
| --- | --- | --- | --- | --- | --- |
| `preset` | Preset | choice `custom`, `fine_35mm`, `coarse_16mm`, `vintage_8mm` | `custom` | | a preset replaces intensity, size, softness, saturation, shadows and highlights with the values below. **Not AE's stock names** (AE's presets are not reproduced) |
| `intensity` | Intensity | 4% of full scale | 0.5 | 0 to 10 | the standard deviation where the tone weight is 1: 1 = 10.2 levels of 255 |
| `size` | Size | px | 1.5 | 0.5 to 20 | autocorrelation length, below |
| `aspect` | Aspect Ratio | ratio | 1 | 0.25 to 4 | width over height of a grain |
| `softness` | Softness | px | 0 | 0 to 8 | extra Gaussian blur of the grain at constant strength |
| `saturation` | Saturation | 0 to 1 | 0.3 | | 0 = one grain for the three channels, 1 = one each |
| `shadows`, `midtones`, `highlights` | Shadows, Midtones, Highlights | multiples of intensity | 1 | 0 to 2 | tone response |
| `rate` | Animation (frames per second) | redraws per second of layer time | 24 | 0 to 60 | 0 = frozen |
| `seed` | Random Seed | | 0 | 0 to 100000 | another seed is another equally good pattern |

Presets (intensity, size, softness, saturation, shadows, highlights): `fine_35mm` 0.6, 1.2, 0, 0.15, 0.8, 0.45;
`coarse_16mm` 1.1, 2.2, 0.3, 0.3, 0.9, 0.5; `vintage_8mm` 1.8, 3.5, 0.6, 0.45, 1.0, 0.7.

**Not implemented** (and why): *Viewing Mode* (Final Output / Noise Only / Blend With Original) and the blend-mode choice of AE's
Add Grain: the effect always shows the final output, and a blend is a layer blend mode already; *Animate Smoothly* (the engine
redraws at `rate`, it does not interpolate between draws); *Tint* (`saturation` and a `tint` effect cover it).

## The field

A stationary Gaussian-like field on a lattice: unit-variance uniform white noise (`+-sqrt(3)`) at the lattice points, spread by a
Gaussian kernel of `KERNEL_SIGMA = 0.75` cells over 4 x 4 taps, sampled at pixel centres. The lattice spacing is
`sigma_px / 0.75` with `sigma_px = sqrt((size * aspect / 2)^2 + softness^2)` across and `sqrt((size / 2)^2 + softness^2)` down. The
field has unit variance (`VARIANCE_NORM`) and the autocorrelation `R(lag) = exp(-lag^2 / (L^2 + 4 softness^2))` with `L = size` down
and `size * aspect` across, so **`size` is the lag at which the correlation has fallen to 1/e**.

The field depends only on `(frame key, seed, field number, lattice cell)`: `key = (floor(time * rate + 0.5), seed)`, hashed with the
prelude's `hash21`/`hash22`. A pixel's value is therefore a pure function of where it is and when, which makes a render byte-identical
to every other, and the same in **layer space and screen space** because the lattice is in layer pixels (`@space` only decides what
the pixels are, not how the grain is made).

Channels: `grain_c = sqrt(1 - saturation) * shared + sqrt(saturation) * own_c`, so every channel keeps unit variance and two
channels correlate `1 - saturation`.

## Strength and tone

`out = clamp(src + grain * intensity * 0.04 * tone(l), 0, 1)` on straight display numbers, with `l = luma(src)` and three bands that sum to
one at every brightness: `dark = 1 - smoothstep(0, 0.5, l)`, `light = smoothstep(0.5, 1, l)`, `tone = shadows * dark + midtones * (1 - dark - light) + highlights * light`.
The standard deviation of the result, in levels of 255 on a flat grey, is `sqrt((intensity * tone * 0.04 * 255)^2 + 1/12)` (the `1/12` is
the rounding to 8 bits). Clipping at 0 and 255 lowers it where the grain reaches the ends (a grey at 3 sigma from white loses about 4%).

## Measured (numbers asserted in `render/grain_tests.rs`; more in `wp/WP-33/finish/add_grain-table.md`)

| quantity | measured | expected | tolerance asserted |
| --- | --- | --- | --- |
| std / spec at intensity 0.5, 1, 2 (flat 128) | 0.991, 0.994, 0.994 | 1 | 3% |
| mean of one frame (flat 128) | 127.80 to 128.23 (8 frames) | 128 | 0.5 level |
| autocorrelation, size 3, lag 1, 2, 3, 4 | 0.895, 0.643, 0.371, 0.172 | 0.895, 0.641, 0.368, 0.169 | 0.05 |
| autocorrelation, size 6, lag 4 | 0.640 | 0.641 | 0.05 |
| aspect 2, lag 2 across, down | 0.896, 0.644 | 0.895, 0.641 | 0.05 |
| softness 2, std against softness 0 | 1.008 | 1 | 4% |
| saturation 0, 0.5, 1: R-G correlation | 1.000, 0.497, -0.010 | 1, 0.5, 0 | 0.04 |
| tone weights 1.246, 0.5, 1.009 (grey 64, 128, 192), std / spec | 0.995, 0.991, 0.994 | 1 | 5% |
| linear-light film against display film | std ratio 0.9999, mean 127.914 against 127.914 | 1 | 2% |
| frame 0 against frame 1, seed 0 against 1 (correlation) | 0.006, -0.0004 | 0 | 0.03 |

Byte identity: the same film renders the same bytes (`the_same_film_renders_the_same_bytes_and_a_frame_or_a_seed_changes_the_grain`).

## Legacy grains (audit, `wp/WP-33/finish/legacy-grain-table.md`)

Three noise makers existed. They stay, because films use them (the identity table in the report proves it); `add_grain` is the only
one with a specification, and it reuses the prelude's hash rather than adding a fourth.

* `grain` (platform): white uniform noise from `fract(sin(dot(...)))`, std `amount * 255 / sqrt(12)` (measured 7.35 for `amount 0.1`), the
  same at grey 32, 128 and 224, no size, mono (R-G correlation 1.000), lag-1 correlation -0.004. Its hash depends on `sin` precision.
* `film_grain` (project plugin, `motions/lib/effects`): value noise mixed with white noise, tone weighted (std 3.6, 4.8, 2.3 at grey 32,
  128, 224 for `amount 0.12`), lag-1 correlation 0.47, R-G correlation 0.79. Not statistically defined; not reproducible by a formula.
* the `gradient` generator adds `+-0.75` levels of white noise of its own to hide banding. It is one more private dither;
  the film-wide `color.dither` (`parity/specs/dither.md`) is the proper one. It is left as it is (its output is in the films).

`add_grain` is not a reimplementation of either: matching either would pin a formula that has no statistics. Both are `have`
in the ledger as engine-only; `Add Grain` is `done` with `add_grain`.

## Open

`UNSURE`: that AE's Intensity maps to a standard deviation of 4% of full scale per unit (chosen so 1 is a clear but ordinary grain);
that its Size is a 1/e correlation length (it is the length I can state and test); AE's three-band tone shape.
