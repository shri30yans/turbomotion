# Property model v2 (WP-10)

How a property is evaluated at a time, for every consumer (layer transform, effect parameters, shape items, text
animators, camera, light, particles). Keys written as before mean what they meant before: `wp/WP-10/identity/` holds the
stills and film hashes of every example and built film, before and after, identical. The new vocabulary is additive and
opt-in: a key written as an object with handles, a property with `spatial` / `auto_orient` / `separate_dimensions`.

## One mechanism

`anim::prop::Track::sample(t) -> Value`. A track is the keys of one property of one type (`Kind`: scalar, 2D or 3D vector,
colour, switch, text) plus what it does after its last key (`Loop`) and, for a position, the motion path.

* `Prop` is a scalar track and an expression (`value` is the track's sample); `Prop::base` is `Track::sample`.
* `VecProp::Axes` is one `Prop` per dimension (`[x, y]`, `separate_dimensions`, vector keys written as arrays);
  `VecProp::Joined` is one track for the whole vector (keys written as objects). `ColorProp::Channels` / `Joined` likewise.
* Every consumer calls `VecProp::at_with` / `ColorProp::at` / `Prop::at`; none keeps its own interpolation.
* Segment shape (`shape.rs`): `Shape::{Hold, Named(ease), Handles}`. A named ease is `a + (b - a) * ease(x)` exactly as
  before (bit-identical: `every_old_key_list_gives_the_same_bits_as_before`).

## Temporal interpolation (speed and influence)

On a segment of `dur` seconds from value `a` to `b`, normalised time `x = (t - t_a) / dur`, each side has a speed `s`
(units per second) and an influence `i` (fraction 0..1 of the segment's time; the JSON writes percent). The value is the
cubic Bezier on the graph of value against time with control points

    P0 = (0, a)   P1 = (i_out, a + s_out * i_out * dur)   P2 = (1 - i_in, b - s_in * i_in * dur)   P3 = (1, b)

and the time is solved from the x coordinate: `u` such that `x(u) = x`, then the value is `y(u)`. Influences are kept as
given, they are not scaled when they overlap (EffectCraft measured this against After Effects 26.3, its keyframe-ease
run: easy, high influence, asymmetric, overshoot, descending, equal endpoints, maximum influence, minimum influence).

* a linear side is the segment's own speed `(b - a) / dur` with influence 1/3; two linear sides are a straight line;
* Easy Ease is speed 0, influence 33.333 %: `value(t) = a + (b - a)(3x^2 - 2x^3)`;
* Auto Bezier: speed = slope from the key before to the key after (per dimension), influence 16.667 %; the first and last
  key have speed 0. (The end-key rule is the one part not measured in After Effects; see the report.)
* continuous: both sides one speed;
* hold (leaving side only): the value stays at `a` until the next key's time, then is `b` exactly;
* the solver (`anim::bezier::solve`) keeps Newton's method inside a bisection bracket, so it is exact where the time curve
  is flat (influence 100 % on both sides stalls the plain method at the middle): `bezier::solve` returns `x(u) = x` to
  1e-12 for every pair of influences in steps of 10 %, and `cubic(x1, y1, x2, y2)` eases use the same function.

A speed that disagrees with the neighbouring key overshoots: the value leaves the range of its keys, as in After Effects.

## Spatial interpolation (motion paths)

A position written as keys with `{"t", "v"}` is a path: one cubic Bezier piece between each two keys. A key's handles are
offsets from the key: control points are `p + out` (leaving) and `p_next + in` (arriving).

* `auto` (Auto Bezier): `out = (next - prev) / 6`, `in = -out`; the first and last key use their one neighbour; a side that
  faces a key in the same place has no handle. The path is smooth through the key: the direction before and after agree to
  1e-9.
* `linear`: no handles. `bezier`: the given `in` and `out`. `continuous`: one handle given, the other its mirror (or both,
  on one line pointing opposite ways).
* the path is travelled by arc length: a temporal curve (above) maps time to distance along the path, with speeds in pixels
  per second along it. Arc length is measured by 5-point Gauss-Legendre per 1/64 of a piece, refined by Newton.
* beyond either end (a temporal overshoot) the position continues straight along the end tangent.
* roving: a key with no time. The fixed keys around it carry one temporal curve over the concatenation of the pieces; the
  roving key is passed when the distance reaches it, so the speed does not change there.
* `auto_orient: "path"`: the layer faces the direction of travel. A flat layer turns about its parent's z axis by
  `atan2(dy, dx)` (the same turn `along.orient` gives); a 3D layer's depth axis runs along the direction (`Aim::Travel`).
  Where the curve stands still at a key (handles of length zero) the direction is that of the piece's chord. Refused with
  `aim` or `along.orient`.

## Types

* scalar, 2D vector, 3D vector, colour: interpolate per component. Colour keys interpolate the channels as written
  (0..255 and alpha 0..1) and the result is clamped. The colour pipeline (`colour_pipeline.md`) keeps the numbers a film
  holds display-encoded, with Skia the one converter and no sRGB curve in the CPU code; an interpolation "in linear light"
  would need one. So a colour fade is a fade of the numbers, as the blend and the fill of a layer already are. This differs
  from After Effects' linear-light option and is recorded as an open question in the report.
* switch and text: change only at a key (a track of them has no handles). They are in the track, not yet in any JSON form.

## Validation (`comp check`, at plan time)

Refused with the key and what to change: tangent of the wrong dimension, influence outside 0..100, key times going back,
two shapes for one segment (a named ease with handles), hold as an arriving side, a handle with no speed (unless
`continuous`), speeds of the wrong count, `in`/`out` on an auto or linear key, a continuous key whose handles do not
oppose, roving keys first or last or with a time, path fields on a property that has no path, unknown key fields with
the nearest name.

## Not in this package

Orientation and rotation stay per axis (each axis is a scalar property and takes handles); a path value that morphs is
the scalar `morph` of a shape item. Expressions on a whole-vector track are not offered: use `separate_dimensions`.
