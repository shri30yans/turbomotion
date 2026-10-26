# Colour pipeline (WP-03)

Opt-in colour management. Comp key `"color": {"linear_light": false, "working_profile": "srgb", "space": "srgb", "depth": "half"}`.
Left out, or equal to those defaults, the film is rendered by exactly the code that rendered it before: no surface is
tagged, Skia converts nothing, and every existing example renders byte for byte as it did (`wp/WP-03/identity/`: 88 stills
and the mp4/mov streams of ten renders, before and after tables identical).

## What the numbers of a picture mean

Every layer picture is half-float premultiplied (as before). The default meaning is "display-encoded sRGB": 0.5 is the
number a colour literal `#808080` gives (0.502), and a blend or blur averages those numbers. That is not light: half the
photons of white are 0.5 *in light*, which the display shows as 0.735 (level 188), not 0.5 (level 128).

With `color` set, a **pipeline** has two profiles (a gamut and a transfer curve each):

| | meaning | set by |
| --- | --- | --- |
| working | what the pictures of a composition hold; blending, blurring and resampling happen on these numbers | `linear_light` (curve), `working_profile` (gamut) |
| output | what the finished frame is encoded in; the file is tagged with it | `space` (gamut, root only; the curve is sRGB's) |

A nested composition takes the root's `linear_light` and `working_profile` field by field unless it says its own; its
picture is converted into the parent's working profile when it is laid on the parent. `space` belongs to the finished frame
and a nested composition that sets it is an error.

## One mechanism

Skia's colour management. A surface is tagged with the colour space of its profile (`core::color::Profile::space`), and
everything drawn into it is converted by Skia on the way in: colour literals, gradients (so a gradient interpolates in the
working profile), images, video frames and other pictures. Skia un-premultiplies, converts, and premultiplies again, so alpha is
coverage and is never encoded (test `premultiplication_is_undone_before_a_colour_is_converted`: a straight
`#c86432` at alpha 0.5 holds straight light `0.5776, 0.1274, 0.0319` and alpha 0.500 in a linear surface). The default
pipeline tags nothing; an untagged surface is not converted. There is no second implementation of any conversion: the only
sRGB curve and gamut matrices in the engine are Skia's, and the tests compare Skia to the published definitions.

* **Pictures** (`core::layer::picture_info`): half-float premultiplied, tagged with the same colour space as the canvas they
  are made beside, so a picture is in its composition's working profile wherever it was made (backdrop, shutter average,
  mesh batch, layout mask included). `graph::execute` makes a nested composition's pictures beside a canvas with *its*
  profile when that differs (`core::layer::with_working_space`).
* **The frame** (`Renderer`): the root composition is drawn into a working-profile canvas. When the working and output
  profiles differ the picture is first flattened over opaque black *in the working profile* (`post::over_black`), because
  converting a translucent picture and then flattening it is a different colour, then drawn onto the output surface, which
  is tagged with the output profile (BGRA8888). The encoder tags the file (`-color_primaries`, `-color_trc`, `-colorspace
  bt709`) and converts RGB to YUV with the BT.709 matrix it names (`encode::VideoFormat::profile`); an unmanaged film gets no
  tags and no filter, as before.
* **Images** (`media::image`): in a managed film each still is decoded in its own profile (the embedded ICC profile, or sRGB
  when it has none) and tagged with it, so the draw converts. An unmanaged film decodes untagged, as before.
* **Video** (`media::color`, `media::probe`): `ffprobe`'s `color_primaries` and `color_transfer` give the profile; an untagged
  stream is BT.709 as players take it, and the BT.709 and BT.2020 10/12-bit curves are read as sRGB's. PQ and HLG are an
  error naming the file and the way out (convert it to SDR first); so are primaries outside the five known.
  A video's RGB conversion still uses ffmpeg's default matrix, as before.
* **Flat colours** (`core::color::Ink`, test `every_level_painted_into_linear_light_arrives_as_the_light_the_curve_says`):
  the colour of a solid, a shape fill or stroke, a glyph, a text mark and a particle is painted by a shader that returns
  its colour as a float uniform (`shaders/ink.sksl`, `layout(color)`), not as the paint's colour. Skia's GPU backend
  carries a constant paint colour to the fragment stage as an 8-bit attribute of the geometry, after converting it to the
  picture's colour space; in linear light that holds the colour in 8 bits of *light*, where level 8 and level 16 are both
  1/255 (level 13). A shader uniform is a float on the GPU and Skia converts it exactly. A full-frame solid is not affected
  (Skia draws it as a clear), nor a gradient, a background, a mesh, an image or a colour filter's colour (all evaluated in
  float). A picture that is not tagged is not converted, so `Ink` sets the paint's colour as it always was.
* **Mesh shading** is display-referred (a light's colour is multiplied by a surface colour as in the original); the batch is
  drawn into a surface with the composition's tag, so its output is converted like any other picture.

## Effects declare the encoding they compute in

An effect written against what the eye sees (brightness, contrast, hue, tint, grain, grade, a threshold at 0.5) is
`display`. An effect that moves or spreads light (blur, glow, shadow, exposure, the warps, resampling) is `linear`. In a
film whose working profile is linear light, a `display` effect is run with its input converted to display numbers and its
output converted back (Skia's `with_working_color_space` on the shader or colour filter, `core::color::Encoding`): no extra
pass, and the slider feels the same in either mode. When the working profile is not linear, both are the same numbers.

* Platform effects: the registry carries the encoding (`registry!` entries). Linear: blur, backdrop, directional_blur,
  radial_blur, glow, shadow, outline, exposure, wave, twirl, bulge, pixelate, chromatic_aberration, rgb_split, opacity,
  transform. Display: brightness, contrast, saturation, hue, gamma, invert, tint, grade, grain, scanlines, vignette.
  Glow's threshold step is a display luma; its halo spreads light.
* Plugins: header tag `// @space display|linear` (default `display`). A `color` parameter is declared `layout(color)`, so it
  arrives in the encoding the shader computes in (a `#808080` parameter of a linear plugin is `0.2158`, and is shown as
  `#808080` again). The shipped pack: `tilt_shift` (a plain average) is `linear`; the other 69 are `display` until each is
  ported (their thresholds and screen blends are display-referred).
* Luma mattes, luma bands (`composite.mask` luma/edges) and lighting read and multiply display numbers: a level-128 luma
  matte lets 0.502 of a white layer through, which shows as 188 in light (`a_luma_matte_reads_the_brightness_the_eye_sees_in_light_too`).
* `parity/effects.tsv` `linear` column: `yes`/`no` for every row that names engine effects, checked against the catalogue by
  `parity check` (`check::linear`).

## Numbers (tests assert all of these)

On Skia's CPU raster (`core/src/color/tests.rs`; the raster pipeline approximates the curve's power function):

| what | measured | bound |
| --- | --- | --- |
| sRGB -> linear, all 256 levels, off the IEC 61966-2-1 curve | 0.00022 | 3e-4 |
| sRGB -> linear -> sRGB round trip, worst level (of 255) | 0.019 | 0.05 |
| `#c86432` at alpha 0.5 into a linear surface, alpha | 0.500 | 1e-6 |
| Display P3 red as sRGB (extended range) | 1.0930, -0.2267, -0.1501 | the P3 -> sRGB matrix to 2e-3 |

On the GPU, end to end through a film (`render/color_tests.rs`, `render/color_media_tests.rs`):

| what | light (`linear_light`) | default (encoded numbers) |
| --- | --- | --- |
| 50% white over black, level shown (expected 187.5 / 127.5) | 188 | 128 |
| two `#808080` solids, the second `add`, level shown (expected 175.6 / clip) | 176 | 255 |
| blur radius 6 across a black/white edge: the two pixels either side, levels | 181 and 192 (light 0.462 + 0.527, mean 0.495) | 118 and 135 (mean 126.5) |
| the same pair's mean level on screen | 186.5 | 126.5 |
| 256 px black-to-white gradient: worst error against the ideal ramp, levels | 0.56 (never decreases; largest step 12 against the ideal 11.1) | 0.50 |
| `contrast` 2 on `#606060` (display effect; expected 64.5) | 64 | 64 |
| level-128 luma matte over a white layer (expected 187.5 / 127.5) | 188 | 128 |
| plugin doubling what it sees on `#808080` | `@space display` 255, `@space linear` 176 | 255 and 255 |
| plugin painting its `#808080` colour parameter | 128 | 128 |
| sRGB red in a film leaving in Display P3, as bytes (expected 234, 51, 35 from the CSS Color 4 matrix) | 234, 51, 35 | (same, `space` only) |
| P3-tagged PNG of P3 red in a P3 film | 255, 0, 0 | 255, 0, 0 (untagged: drawn as it is) |
| sRGB-tagged PNG of red in a P3 film | 234, 51, 35 | 255, 0, 0 |
| two renders of a linear-light film | identical bytes | identical bytes |
| `color: {}` against no `color` | identical bytes | |

Dark levels (`render/color_darks_tests.rs`, `render/color_flat_tests.rs`, probes `wp/WP-03/fix-darks/`): 256 small swatches
(a 4 px solid, then a 4 px shape, in a 128 px frame) over black with the `normal` blend:

| what | measured | bound |
| --- | --- | --- |
| every level 0..255, small solid and small shape in light: output against the level drawn | exact (0 levels off), 256 distinct outputs | 1 level; levels 1..32 each within 1; strictly rising |
| the same sheet on encoded numbers (default film) | exact | 1 level |
| full-frame solids (levels 0..32, every 15th above, 254, 255) | within 1 | 1 level |
| 50% white, small solid and small shape, in light | 188 | 187.5 within 1 |
| levels 8..64 at 50% alpha over black, small solid, in light | half the light of the level, within 1 | 1 level |
| gradient across a layer smaller than the frame, 256 steps | worst 0.56 off the ideal light ramp, largest step 12 against the ideal 11.1 | 1 level, ideal step + 1 |
| stroke, text, particle and background of levels 8, 16 and 48 in light against the default film | equal within 1 | 1 level |

Before the fix (same films, baseline binary): 26 levels were more than 1 off (worst 6), 183 distinct outputs of 256, 73
levels did not rise above the one below it; levels 7..17 all showed 13, 18..25 showed 22, 26..31 showed 28.

Banding: a half-float working picture adds none of its own. The measured gradient is within 0.56 level of the ideal ramp, and the
largest step is the ideal ramp's own steepest (the dark end of a light-linear ramp is steep when shown in 8 bits: 12 levels
between the first two pixels), so there is no band beyond 8-bit quantisation. Dithering the 8-bit output is not done.

## Not done (deliberately)

* Rec.2020 / ACES working spaces, HDR (PQ/HLG) video, 32-bit float depth, ICC output profiles, LUT display management:
  the ledger row "Colour management beyond P3" stays `none`.
* Porting the display-referred plugins of the shipped pack to `@space linear` one by one (each has thresholds or screen
  blends that need re-deriving in light).
* A layer's opacity is carried to the GPU as the alpha of a paint, which is also 8 bits: in linear light white at 0.2%
  opacity shows level 13 where the light says 6.6 (0.1% shows 0 against 3.3, 1% shows 28 against 25.5; 2% was within a
  level). The default film has the same 1/255 step, which costs it far less: its levels are encoded. Moving opacity to a float needs the
  blend stage to take it as a uniform (the shader blends and the dissolve already read it as one); the ledger row stays `part`.
* The 8-bit alpha codec still clears opaque black (an existing behaviour); `Codec::Alpha` frames are not colour-converted any
  differently from the others.
