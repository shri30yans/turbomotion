# Motion blur (WP-13)

Opt-in After Effects motion blur. Comp key `"motion_blur": {"shutter_angle": 180, "shutter_phase": -90, "samples": 16}` and `"mblur": true` on the layers that blur. Left out, every film renders byte for byte as before (`wp/WP-13/identity/`: 96 stills, the ten video streams and four blurred films, before and after identical).

## The model

The frame at time `t` is the root composition evaluated at `samples` moments of the open shutter and averaged. The shutter is open from `t + phase/360 * d` for `angle/360 * d`, where `d = 1/fps`. Sample `i` is in the middle of the `i`th of `samples` equal slices: `t + ((i + 0.5)/samples + phase/angle) * angle/360 * d`. One evaluator (`Evaluator::frame`) produces every moment, so transforms, WP-10 temporal and spatial paths, keys and their speed/influence handles, the camera, 3D, text animators, particles and repeaters all blur by the same code, at their true sub-frame values, never a straight line through the speed at `t`.

Defaults: angle 180, phase -90 (a 180 degree shutter centred on the frame), samples 16. Ranges: angle 0 to 720, phase -360 to 360, samples 2 to 16; the schema refuses the rest with the range. The centre of the window is `t + (phase + angle/2)/360 * d`, so a 360 degree shutter at the default phase is centred a quarter frame late; use phase -180 to centre it.

## What is averaged

* Pictures are premultiplied, so a sum of samples divided by `samples` is the right alpha and the right colour over what is behind (`a_half_covered_pixel_takes_half_its_colour...`).
* The average is made in the film's working colour space. With `color.linear_light` the sum is of light: a half-covered black and white edge is level 188 on screen; averaging display numbers gives 128.
* Effects run once per sample (an effect is a function of the moment, as a transform is).
* A layer without the switch is drawn once, sharp, at `t`. A layer that does not move over the shutter is drawn once and equals the unblurred render bit for bit; "does not move" is decided from the evaluated state of every moment, and any layer with effects, a matte, a clip or media is never taken for still.
* A frame of a film with no switched layer evaluates no extra moments and costs nothing.

## Sampling and its limit

The sampling is fixed: `samples` moments, midpoint rule. The mean of a smear is exact but its outer extent is one sample step short (50/16 = 3.1 px of a 70 px smear), and a smear much longer than the moving thing shows a ripple of one sample (16 levels of 255 at 16 samples). More samples cost proportionally more, so 16 is the cap. After Effects' Adaptive Sample Limit is not implemented.

## Nested compositions

Only the layers of the root composition have a switch. A precomp layer with the switch blurs everything it shows, whatever its inner layers' switches say; an inner switch does nothing (`comp check` warns, `motion-blur-nested`). After Effects blurs inside a precomp when the precomp layer's switch is on and the inner layer's is on too; this engine has one shutter, the root's, and one average per root layer.

## The older key

`"mblur": {"samples": 8, "shutter": 180}` on the composition keeps its meaning exactly: centred shutter, 8 samples by default, a layer is blurred only if it is visible at the frame time, no still detection. Both keys together are an error. Reason for the second key rather than a change: the older maths is a legitimate model, and changing it would change existing renders.

## Proof

Tests: `crates/comp/src/render/shutter_tests.rs` and `shutter_world_tests.rs` (pixel measurements against closed forms), `crates/comp/tests/motion_blur_schema.rs`, `crates/comp/src/check/motion_blur.rs`, `crates/script/tests/`. Probes with measured against expected numbers: `wp/WP-13/probes/data.tsv`.
