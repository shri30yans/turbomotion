# Film halation (`film_halation`)

Written in my own words (BUILD_PLAN section 4, step 2). Halation is not an After Effects effect; it is what film does: light that
passed through the emulsion reflects off the film base and exposes it again, ringing the brightest parts with a wide red-orange
halo. **Nothing below is a measurement of a film stock.** Every rule is a definition, implemented as a platform effect
(`core/src/effects/halation.rs`) and measured (`effects/halation_tests.rs`, `render/halation_tests.rs`).

## Naming

The engine already had a shader plugin `halation` (a 20-tap ring sample with a hard `smoothstep(0.7, 1)` luma gate). Films use it, so
it stays exactly as it was (`halation` is `have`, engine-only). A plugin cannot share a platform effect's name, so the new one is
`film_halation`. They are different effects: the legacy one is not energy-conserving, has no falloff law and reaches only `radius` px.

## Parameters

| name | unit | default | range | notes |
| --- | --- | --- | --- | --- |
| `threshold` | luma, 0 to 1 | 0.6 | 0 to 1 | light above it halates. In a `linear_light` film it is a threshold on **light itself**; in an ordinary film it is on the numbers the film works with (display-encoded), like every effect of its encoding |
| `radius` | px | 40 | 0 to | Gaussian sigma of the halo |
| `strength` | | 0.5 | 0 to | the luma added back as a fraction of the light that passed the threshold |
| `color` | colour | `[255, 90, 40]` | | the hue of the halo; only the hue counts, its luma is normalised away |

## Formula

Encoding `Linear` (`@space linear`: light in a linear-light film, the working numbers otherwise), `@bounds all sides 3 * radius`
(`GAUSSIAN_REACH`), so in layer space the halo is not cut at the layer's edge.

1. **Excess.** `e = max(0, dot(rgb, (0.2126, 0.7152, 0.0722)) - threshold * a)` on the premultiplied colour: a pixel of straight luma `L`
   and coverage `a` gives `(L - threshold) * a`. Stored as grey light with the picture's alpha.
2. **Blur** with `Layer::blurred(radius)`, the engine's one Gaussian (no second blur).
3. **Add back** `light = (color / luma(color)) * strength * blurred_e` to the premultiplied colour; alpha becomes
   `a + (1 - a) * min(1, max(light))`, so a halo on a transparent picture has coverage.

**Energy.** The blur keeps its sum and the tint has unit luma, so summed over the picture **the luma added is `strength` times the summed
excess**: a 4 x 4 white square (16 px, excess 0.5 each) at `strength 0.5` adds **4.000 luma** (measured within 0.5%, per-channel sums match
`tint / luma(tint)`).

## Measured

| quantity | measured | closed form | asserted |
| --- | --- | --- | --- |
| added luma, 4 x 4 square, threshold 0.5, strength 0.5, sigma 4 | 4.000 within 0.5% | 4 | 0.5% |
| point source (1 px white), sigma 4: sum of light | 1.0002 | 1 | 0.5% |
| peak and 0, 2, 4, 8, 12 px out, across and down | within 3% | `g(d) g(0)` of the 3-sigma-cut sampled Gaussian | 3% |
| standard deviation along a row, sigma 4 | 3.967 | the cut kernel's, about 3.97 (4 less the tails the 3-sigma cut takes) | 0.5% |
| 64 px square, sigma 24, profile 0 to 64 px out | 3.0% low at the centre, 0.3% at 64 px | the square summed against the kernel | 4% of the peak |
| 0.4 under a threshold of 0.5 | exactly no change | 0 | exact |
| 0.6 over 0.5 on 256 px | within 1% of 25.6 | 25.6 | 1% |
| half-covered white (128/255), threshold 0.5 | `a + 0.5 a` | | 0.1% |
| halo hue on transparent, green/red, blue/red | 0.353, 0.157 | 0.353, 0.157 | 0.002 |
| strength 0; threshold 1 | unchanged | | exact |
| film, layer space against screen space | at most 1 level apart | | 1 level |
| film, tail at scale 2 against scale 1 | 2x in layer space, 1x in screen space | | 3 px |
| film, threshold in a linear-light film | level 160 (light 0.35) halates nothing, level 200 (0.58) does; in a display film level 160 (0.63) does | | |

## What is not exact (found, not fixed)

Skia's GPU Gaussian is exact up to a sigma of 4 and works at reduced resolution beyond it. Measured on the engine's own
`Layer::blurred`, a **one-pixel source returns 1.0002x of its light at sigma 4, 1.20x at 5, 1.27x at 6, 1.20x at 7, 1.00x at 8,
0.86x at 8.5, 1.11x at 9, 1.20x at 10 and 1.27x at 12**, a 64 px square at sigma 40 returns 0.95x, a 32 px square 1.03x. The
error is the reduced-resolution sampling of a source smaller than its cells and shrinks as the source grows. `glow`, `bloom` and
`shadow` inherit it. Making the blur exact would change their pictures (and the identity table), so it is **not** changed; a
cascade of passes of sigma at most 4 is exact but costs `(radius / 4)^2` passes (100 at the default radius). Recorded as an open
question: an energy-exact wide blur, versioned.
