# Stroke taper (WP-30, taper slice)

`taper: {start: {length, width}, end: {length, width}, ease}` on a shape item whose `stroke` thins toward either end of
the outline. The two-colour fold, gradient strokes, wave and dashes on a tapered stroke are not here.

## One mechanism

A tapered stroke is a **variable-width stroke**: a filled outline built from the offset curves of the path. The mechanism is
`shape/stroke/` and a taper is only one profile of it.

| piece | where | what it does |
| --- | --- | --- |
| `Profile` | `shape/stroke/mod.rs` | the width at a distance along a contour (a trait). A taper is one; a profile written as positions and widths, or a width that folds, is another, and nothing below changes |
| the walk | `shape/stroke/sample.rs` | walks a contour into points; a step is kept when the centre line and both edges stay within 0.02 px of the chord at a quarter, a half and three quarters (`TOLERANCE`); otherwise it is halved down to a step of that size, where a bend is a corner and is flagged |
| the pieces | `shape/stroke/pieces.rs` | one quadrilateral per step, a bevel wedge, a round fan or a miter (limit 4, Skia's) at each corner, a cap (butt, round, square) at each end of an open contour, all added with the same orientation so a nonzero fill is their union and a tight bend cannot leave a hole |
| the taper | `shape/stroke/taper.rs`, read from the film in `eval/shape/taper.rs`, written in `schema/taper.rs` | `Taper::profile(contour length, full width)` |

## The numbers

* `length`: pixels, or `{"fraction": f}` of the contour's length. 0 or more; held to half the contour, so the two ends never
  overlap (a 5000 px taper on a 400 px line is 200 px each, full width at the middle only).
* `width`: pixels, or `{"fraction": f}` of the stroke's `width`; default 0 (a point). 0 or more.
* `ease`: a GSAP name, default linear. The width at distance d from a tip is `tip + (full - tip) * ease(d / length)`, the same
  ease measured from each tip, so one ease gives two ends that mirror each other. Between the tapers the width is the stroke's `width`.
* Lengths and widths are properties (WP-10) and animate; one that is below 0 or not finite at some time is an error naming
  the end, the field and the value.
* Several contours are each tapered along their own length. With `trim`, the taper follows the trimmed outline.
* A **closed** contour is walked from its start point back to it. `start` tapers the first stretch and `end` the last stretch
  before the start point, so with both the stroke pinches at the start point from both sides; with one, the width steps from
  the tip's width to full at the start point (joined with the stroke's join).
* Strict schema: `taper` needs a `start` or an `end`, a `stroke`, and no `dash`; unknown keys are refused with the nearest
  valid one; `cap` and `join` are checked.
* The bounds of the layer grow by half the widest the stroke gets (a tip wider than the stroke counts).

## Measured

Width across the stroke at points along the path, from rendered frames (encoded and linear-light films, both `effect_space`
modes; coverage in light for linear films), against the width written out in the test: within 0.25 px (`render/taper_tests.rs`).

| test | what |
| --- | --- |
| `the_width_at_the_start_the_middle_and_the_end_is_the_one_asked_for_for_several_eases` | straight, 400 px, eases none, power2.out, sine.inOut, 8 distances |
| `a_curved_path_is_tapered_by_the_distance_along_the_curve` | cubic, fractions, power2.inOut, 9 distances |
| `a_tip_width_given_as_a_fraction_is_a_fraction_of_the_full_width` | |
| `lengths_longer_than_half_the_path_are_held_to_half` | |
| `a_closed_path_is_tapered_from_its_start_point_and_pinches_there_from_both_sides` | |
| `a_tapered_stroke_follows_the_trimmed_outline` | |
| `a_tapered_stroke_takes_the_colour_of_its_copy` | a repeater palette on a tapered stroke |

Against Skia's own stroker: a constant-width outline differs by at most 0.35 coverage in any pixel and by under 0.5 % in area for
every cap and join (`an_outline_of_constant_width_is_what_skia_strokes_for_every_cap_and_join`), and its area is the width times
the path's length within 0.2 % (`the_area_of_a_constant_width_stroke_is_its_width_times_the_length_of_the_path`).
