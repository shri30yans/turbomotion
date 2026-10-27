# WP-33 finish stack: probe images and data

Every number below is computed from the pixels by the scripts beside this file (`make_films.py` writes the films,
`make_evidence.py`, `make_legacy_audit.py` and `make_codec_table.py` measure them; commands are in `../REPORT-finish-stack.md`).
The ledger probes are `parity/probes/add_grain.json` and `parity/probes/film_halation.json`; the dither films are in `films/`.

## Images

* `add_grain-presets.png`: one black-to-white ramp in four 320 px panels (clean, `fine_35mm`, `coarse_16mm`, `vintage_8mm`, seed 3); the grain's standard deviation at mid-grey is 6.2, 11.4 and 18.1 levels, within 1.0%, 1.2% and 1.5% of the spec.
* `add_grain-zoom.png`: a 64 x 64 px crop at the ramp's middle, enlarged 4x, of the four panels (the first is clean); the grain clumps are about 1.2, 2.2 and 3.5 px across and the red-green correlation falls from 0.85 to 0.55 as the presets' saturation rises.
* `halation-before-after.png`: the same lamps without (top) and with (bottom) `film_halation` threshold 0.6, radius 40, strength 0.6, tint 255/90/40 as the film's `look`; the halo adds 14 : 5 : 2 (R : G : B) 40 px above the lamp, the grey `#666666` disc (0.40) under the threshold is unchanged (5, 6, 10).
* `dither-ramp-x16.png`: the level 0 to 16 ramp rounded three ways (top to bottom: none, ordered, blue noise), amplified 16x; none shows 17 flat bands, both dithers show no band and 80 distinct 16-column means against 33.
* `dither-zoom-x16.png`: a 64 x 32 px crop of the same three, enlarged 8x and amplified 16x (none on top, flat); ordered is the regular 16 x 16 cross-hatch, blue noise has no pattern, and no pixel is more than one level from the ideal ramp.

## Add Grain against its formula (`add_grain-table.md`)

Standard error is large for the correlation columns: a 12-row band of grain 1 to 4 px across holds only about 150 independent samples, so
expect 6% on a standard deviation and 0.08 on a correlation. The tight numbers (0.5% to 1%) are in `render/grain_tests.rs`, on 65 536 pixels.

| preset | brightness | measured std (levels) | spec std (levels) | measured/spec | R-G correlation | spec 1 - saturation |
|---|---|---|---|---|---|---|
| fine_35mm | 0.20 | 5.38 | 5.33 | 1.008 | 0.846 | 0.85 |
| fine_35mm | 0.50 | 6.19 | 6.13 | 1.010 | 0.848 | 0.85 |
| fine_35mm | 0.80 | 3.96 | 3.95 | 1.003 | 0.845 | 0.85 |
| coarse_16mm | 0.20 | 10.89 | 10.50 | 1.038 | 0.717 | 0.70 |
| coarse_16mm | 0.50 | 11.35 | 11.22 | 1.012 | 0.698 | 0.70 |
| coarse_16mm | 0.80 | 7.58 | 7.59 | 0.999 | 0.673 | 0.70 |
| vintage_8mm | 0.20 | 19.02 | 18.36 | 1.036 | 0.568 | 0.55 |
| vintage_8mm | 0.50 | 18.09 | 18.36 | 0.985 | 0.499 | 0.55 |
| vintage_8mm | 0.80 | 14.22 | 14.79 | 0.961 | 0.456 | 0.55 |

## Legacy grains (`legacy-grain-table.md`)

| effect | grey level | mean | std (levels) | lag-1 autocorrelation | R-G correlation |
|---|---|---|---|---|---|
| `grain` amount 0.1 | 32 | 32.0 | 7.35 | -0.003 | 1.000 |
| `grain` amount 0.1 | 128 | 128.0 | 7.34 | -0.004 | 1.000 |
| `grain` amount 0.1 | 224 | 224.0 | 7.33 | -0.004 | 1.000 |
| `film_grain` amount 0.12 | 32 | 31.9 | 3.59 | 0.470 | 0.794 |
| `film_grain` amount 0.12 | 128 | 127.9 | 4.77 | 0.471 | 0.796 |
| `film_grain` amount 0.12 | 224 | 223.9 | 2.32 | 0.466 | 0.786 |

## Halation (`halation-table.md`)

| measurement | value |
|---|---|
| 10 px above the lamp (y=280): R, G, B | 43, 19, 16 (plain frame: 5, 6, 10) |
| 40 px (y=250): R, G, B | 19, 11, 12 (plain frame: 5, 6, 10) |
| 80 px (y=210): R, G, B | 7, 7, 10 (plain frame: 5, 6, 10) |
| 160 px (y=130): R, G, B | 5, 6, 10 (plain frame: 5, 6, 10) |
| halo added at 40 px, R : G : B | 14 : 5 : 2 = 1 : 0.357 : 0.143 (the tint 255 : 90 : 40 is 1 : 0.353 : 0.157) |
| 25 px above the grey #666666 disc (0.40 as a number, under the 0.6 threshold) | [5.0, 6.0, 10.0] against plain [5.0, 6.0, 10.0] |

## Dither on the 1280 px ramp, level 0 to 16 (`dither-table.md`)

| rounding | distinct levels | distinct 16-column means | max step between 16-column means | max slab-mean error (levels) |
|---|---|---|---|---|
| none | 17 | 33 | 0.500 | 0.300 |
| ordered | 17 | 80 | 0.227 | 0.017 |
| blue_noise | 17 | 80 | 0.208 | 0.007 |

## Dither after an encode (`codec-table.md`)

| encode | rounding | distinct luma values | distinct 16-column means | max step between 16-column means (luma levels) | luma of the ramp's ends |
|---|---|---|---|---|---|
| libx264 crf 0 (lossless) | none | 15 | 29 | 1.000 | 0.0 to 16.0 |
| libx264 crf 0 (lossless) | ordered | 15 | 72 | 0.414 | 0.1 to 15.9 |
| libx264 crf 0 (lossless) | blue_noise | 15 | 72 | 0.415 | 0.1 to 15.9 |
| libx264 crf 18 | none | 15 | 29 | 1.000 | 0.0 to 16.0 |
| libx264 crf 18 | ordered | 15 | 29 | 1.000 | 0.0 to 16.0 |
| libx264 crf 18 | blue_noise | 15 | 29 | 1.011 | 0.0 to 16.0 |
| h264_videotoolbox 20 Mbit/s | none | 15 | 30 | 1.000 | 0.0 to 16.0 |
| h264_videotoolbox 20 Mbit/s | ordered | 15 | 30 | 1.000 | 0.0 to 16.0 |
| h264_videotoolbox 20 Mbit/s | blue_noise | 15 | 16 | 2.000 | 0.0 to 16.0 |

Lossless libx264 keeps part of the pattern (max step 0.41 against 1.00); libx264 crf 18 and VideoToolbox keep none of it. A PNG keeps all of it.
