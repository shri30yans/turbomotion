# Repeater palette cycle (WP-30, palette slice)

`palette: [colours]`, `cycle: {mode, offset, reverse}` and `paint: "fill" | "stroke" | "both"` on a repeater give every
copy its own colour. Two repeaters take them, through one typed definition: the layer `repeat` and the shape item
`repeat`. Merge, offset, pucker, zig-zag and the rest of WP-30 are not here.

## One definition

| piece | where | what it is |
| --- | --- | --- |
| the keys | `schema/palette.rs`: `Cycling::take` reads and removes `palette`, `cycle`, `paint` from a repeat object | the only parser; empty palette, non-colour entries and a `cycle` without a `palette` are errors that say what to write |
| which colour | `anim/cycle.rs`: `slot(mode, n, copy, offset)` | arithmetic only: `Pick(i)` or `Blend {from, to, by}` |
| what colour | `eval/palette.rs`: `resolve` reads the palette and the offset at the time, `Cycled::colour(copy)` | the one place a slot becomes a colour; a gradient mixes with `Profile::mix` (`core/color/mix.rs`) |
| the shape repeater | `eval/shape/repeat.rs` keeps a `Tint`; `render/shape.rs` asks it for each copy | |
| the layer repeater | `schema/repeat.rs` stamps `swatch: {palette, cycle, paint, index}` on the original (index 0) and each copy; `eval/palette.rs::apply` replaces the layer's colour | |

## What copy i takes

`place = i + offset` colours along the palette of n colours (`reverse` reads the palette last colour first before this).

| mode | colour |
| --- | --- |
| `step` | colour `floor(place) mod n`. A whole offset moves every copy on by that many colours; a fractional offset holds the colour until it reaches the next whole number; a negative one moves back |
| `ping_pong` | the index runs `0 1 .. n-1 n-2 .. 1 0 1 ..` (period `2 (n-1)`, the end colours not doubled; one colour is one colour) |
| `gradient` | with `u = place mod n` and `k = floor(u)`: colour k blended toward colour `(k+1) mod n` by `u - k`; the last colour blends back into the first. Alpha is blended straight |

`offset` is a property (WP-10): a number, an expression, a tween or keys, so it animates; a value that is not finite is an
error naming the offset. Colours in `palette` are properties too.

## Which space a gradient interpolates in

The film's **working colour space** (`parity/specs/colour_pipeline.md`): the numbers pictures hold, which is what blending
happens on. In an ordinary film that is the sRGB-encoded values, so a gradient is a straight mix of levels, as an editor's
gradient is. In a `linear_light` film it is light: the colours are decoded with Skia's own sRGB curve, mixed and encoded
again, so the mid-point of black and white is level 188, not 128. The reason is agreement: a Skia gradient drawn in the same
film interpolates in the working profile (colour pipeline, "one mechanism"), and the test
`a_gradient_in_linear_light_matches_a_skia_gradient_drawn_in_the_same_film` holds the palette cycle to it within 1 level.
Only the transfer curve matters; a change of gamut is a matrix, and a matrix commutes with a mix of light.

## Which paint takes it

`paint: "fill"` (default), `"stroke"`, `"both"`. For a text or solid layer, "fill" is its own colour; "stroke" is the text's
`stroke` colour. A shape item takes a fill colour only when its fill is one flat colour (a gradient or metal fill, or no fill, is refused
with the reason) and a stroke colour only when it has a stroke. On a layer repeater the layer must be a text, solid or shape
layer; any other type is refused when the film is planned (`Swatch::check`). A text with a `fill` (which is drawn instead of `color`)
is refused for a fill palette.

## Interplay

The repeater's own `opacity` factor is applied per copy as before, to the coloured copy: copy i at `opacity^i` over black
is `colour * opacity^i` (encoded) or the light of the colour times `opacity^i` (linear light); measured, within 1 level.

**Not built, and said so:** the repeater has whole copies only (`count`, `copies` are integers) and a per-copy factor, not
After Effects' start/end opacity ramp. There is no fractional copy to colour; the cycle depends only on the integer copy index, so it
will compose with them when the WP-30 repeater gets them.

## Measured (all in `render/palette_tests.rs`, in encoded and linear-light films, both `effect_space` modes, tolerance 1 level)

| test | what |
| --- | --- |
| `step_copy_i_shows_palette_i_plus_offset_mod_n_in_every_film` | 7 cases of N, n, offset (0, 1, -1, 2.5, -0.5, 7, -7.25) and reverse |
| `ping_pong_shows_the_exact_index_sequence` | 0 1 2 3 2 1 |
| `a_gradient_is_the_closed_form_mix_in_the_working_space_of_each_film` | offsets 0, 0.25, -0.5, 1.75; closed form in levels (encoded) or in light (IEC 61966-2-1) |
| `a_keyed_offset_changes_the_assignment_as_time_passes` | offset 0 to 4 over 1 s, sampled at 5 times |
| `the_palette_works_with_the_opacity_ramp_of_the_repeater` | opacity 0.8 per copy |
| `a_layer_repeater_colours_solids_shapes_and_text_by_their_place_in_the_row` | and the text test with an opacity ramp |
