# Text bounds and the text `box` (WP-32 step 1)

Layer key `box: {fit: "text", padding: [x, y], radius, fill, min_width, align, per_line}` on a text layer, and the
measurement under it: After Effects' `sourceRectAtTime(t, includeExtents)`. Selectors, animator properties, paragraph
and variable fonts are the other steps of WP-32 and are not here.

## One measurement

`text::bounds::TextBounds::of(layout, glyph_states, stroke)` is the only place text is measured. Everything reads it:

| reader | where | what it gets |
| --- | --- | --- |
| the box | `eval/text/bounds.rs::box_state` -> `text/boxfit.rs` | the rectangles to draw |
| `comp inspect` | the layer's `content.bounds` (`block`, `extents`, `lines`) and `content.text_box` | the same numbers |
| `source_rect_at_time(ev, comp, layer, t, include_extents)` | `eval/text/bounds.rs` | a typed `SourceRect {left, top, width, height}`, or `None` when no character shows; an error naming the layer type for a layer that is not text. WP-12 binds it to expressions |
| `measure()` (SDK) | `text/measure.rs`, `bounds` | the same bounds with every character showing |

**What counts.** A glyph counts when it is drawn: its opacity is above `MIN_VISIBLE_OPACITY`, it is in front of the
viewer, and what is left of its transformed box after the animator's mask clip is not empty. Its box is the line's
(ascent to descent, so every character of a line is as tall as the line) by the character's advance, moved by what
the animators did to it (offset, scale, rotation, tracking). Whitespace has no glyph, so the bounds hug the first and
last visible character of a line. `extents` grows `block` by half the widest stroke drawn (After Effects'
`includeExtents`).

**Typed text.** A character with nothing showing after it on its line is as wide as it would be typed last: its own
advance and tracking (`Glyph::trail`), without the pair kerning toward a character that is not there. So a typed bar is the
width of the text typed so far, laid out alone. The last character of a ligature (`fi`) has nothing to restore.

## The box

* Width: the characters showing, plus `padding.x` on both sides. Height: the lines they are on, plus `padding.y` above
  and below. `per_line: true` is one rectangle per line that has a visible character; `false` is one round the block.
* `min_width` widens a narrower rectangle (padding included). `align` names the side that keeps its place:
  `left` grows to the right, `right` to the left, `center` both ways; absent, the text layer's own `align`.
* `radius` is held to half the shorter side (a pill). `padding` is two properties (WP-10): each may be a number, an
  expression, a tween or keys; a value below 0 at any time is an error naming the axis and the time.
* No character showing means no rectangles, and nothing is drawn.
* It is part of the layer: drawn first, behind the glyphs, in the layer's buffer and transform. In `effect_space:
  "layer"` the buffer's content bounds include the rectangles, so the effects and masks see (and are not clipped to) the box.
* `comp check` reads the box's colour as the background of the type (a text with a box is not low contrast against the
  ground behind it).

## Numbers (all in tests; `wp/WP-32/probes/data.tsv` has the drawn-against-measured table)

| claim | measured | test |
| --- | --- | --- |
| box width = measured width + 2 x padding, 12 strings x 3 fonts (36), kerning pairs, ligatures, edge spaces, 2-3 lines | worst 0.00002 px (limit 0.5) | `one_box_is_the_widest_extent_of_the_characters_plus_twice_the_padding_in_every_font` |
| kerning is in the measurement | AVATAR in Manrope is 17.6 px narrower than its letters alone, WAVE 11.2, ToYo 19.2; Instrument Serif 18.4, 11.7, 11.4; JetBrains Mono 0 | `kerning_pairs_make_a_proportional_font_narrower_than_its_letters_alone_and_a_monospace_font_the_same` |
| a typed bar equals the text typed so far, alone | 204 keystrokes (3 fonts x tracking 0 and 6 x 34 characters), every one within 0.5 px of the text typed alone, strictly growing | `a_typewriter_reveal_grows_the_box_to_the_width_of_the_text_typed_so_far_at_every_sampled_time` |
| a typed swap source | exact | `a_typed_swap_is_the_width_of_the_prefix_typed_so_far` |
| drawn box = inspected box, 4 edges, 12 cases | worst 0.003 px (limit 0.5) | `the_drawn_box_is_the_measured_box_within_half_a_pixel_on_every_side` |
| radius 24 removes (4 - pi) r^2 of area; radius 1e6 draws a pill | 494.46 against 494.44 px; 28357.4 against 28356.8 | `a_radius_takes_four_minus_pi_radius_squared_off_the_area_and_is_held_to_a_pill` |
| layer scale 2, both effect spaces | box 2.000 x the measured box, edges within 0.5 px | `at_scale_two_the_drawn_box_is_exactly_twice_the_measured_one_in_both_effect_spaces` |
| layer space buffer holds the box | `local.bounds_local` equals the box edges within 1 px, with a negative origin | `the_buffer_of_a_layer_space_layer_holds_the_box_not_only_the_text` |

## Known limits

* **Half a ligature.** Typing the middle of `ffi` shapes `ff` alone as a ligature of its own width; the box restores the
  character's own advance, so it is up to 3.7 px from the text typed alone at those keystrokes (pinned in
  `a_ligature_typed_half_way_is_close_to_the_text_typed_alone_and_exact_where_it_ends`). Whole clusters are exact.
* **Masked reveals follow the mask.** A `rise`/`reveal` preset clips each line to its own box, so the bounds (and the
  box) grow as the line comes out from behind the mask, not at the full size.
* A text with a `path` or `surface` is measured in its own layout space, as the text is; the box is a rectangle there,
  not bent with the path.
