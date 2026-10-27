//! `motion_tile` (BUILD_PLAN 5.3): identity, the closed-form pixel positions of tiling, mirroring, phase, the tile centre,
//! the output crop and the gutter, and determinism. Seams, sub-pixel centres, animation, layer space and the colour
//! encoding are in `motion_tile_seam_tests.rs`.
//!
//! The test card is a 32 x 16 picture of four quadrants, each a different colour, in a 96 x 48 frame with the card layer
//! in the middle (its top-left corner at frame pixel (32, 16)). With `effect_space: layer` and an output of 300 % the
//! layer's picture grows to exactly the frame, so frame pixel (X, Y) is layer pixel (X - 32, Y - 16). Every expected
//! pixel below is that layer pixel's source texel, worked out on whole numbers from the definitions in
//! `parity/specs/motion_tile.md`.

use serde_json::{json, Value};

use super::testkit::{bgra, render};

pub(super) const FRAME: [usize; 2] = [96, 48];
pub(super) const CARD: [i32; 2] = [32, 16];
/// The card's top-left corner in the frame: the card is centred.
pub(super) const ORIGIN: [i32; 2] = [32, 16];
const QUADRANTS: [&str; 4] = ["#e02020", "#20c020", "#2040e0", "#e0e020"];

/// The colour of card texel (x, y), `0 <= x < 32`, `0 <= y < 16`: top-left, top-right, bottom-left, bottom-right.
pub(super) fn card_rgb(x: i32, y: i32) -> [u8; 3] {
    let hex = QUADRANTS[usize::from(x >= CARD[0] / 2) + 2 * usize::from(y >= CARD[1] / 2)];
    [1, 3, 5].map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or_default())
}

/// The card as a nested composition, and the film that shows it as a layer with `fx` on it.
pub(super) fn card_film(fx: Value, space: &str) -> Value {
    let (w, h) = (CARD[0] / 2, CARD[1] / 2);
    let quadrant = |i: usize| json!({"type": "solid", "color": QUADRANTS[i], "size": [w, h], "pos": [w / 2 + w * (i as i32 % 2), h / 2 + h * (i as i32 / 2)]});
    json!({"width": FRAME[0], "height": FRAME[1], "fps": 10, "dur": 1, "bg": "#000000", "effect_space": space,
        "comps": {"card": {"width": CARD[0], "height": CARD[1], "dur": 1, "layers": [quadrant(0), quadrant(1), quadrant(2), quadrant(3)]}},
        "layers": [{"type": "precomp", "comp": "card", "pos": [FRAME[0] / 2, FRAME[1] / 2], "fx": [fx]}]})
}

/// The effect with its output at 300 % (every tile of the picture shows), plus `extra` parameters.
pub(super) fn tiled(extra: Value) -> Value {
    let mut fx = json!({"effect": "motion_tile", "output_width": 300, "output_height": 300});
    if let (Some(base), Some(more)) = (fx.as_object_mut(), extra.as_object()) {
        base.extend(more.clone());
    }
    fx
}

pub(super) fn rgb(px: &[u8], x: usize, y: usize) -> [u8; 3] {
    let [b, g, r, _] = bgra(px, FRAME[0], x, y);
    [r, g, b]
}

/// Every pixel of the frame is `want(layer x, layer y)` within `tolerance` levels; the first that is not is the message.
pub(super) fn assert_pixels(px: &[u8], tolerance: u8, want: impl Fn(i32, i32) -> [u8; 3]) {
    for y in 0..FRAME[1] {
        for x in 0..FRAME[0] {
            let (lx, ly) = (x as i32 - ORIGIN[0], y as i32 - ORIGIN[1]);
            let (got, expected) = (rgb(px, x, y), want(lx, ly));
            assert!(got.iter().zip(&expected).all(|(a, b)| a.abs_diff(*b) <= tolerance), "layer pixel ({lx}, {ly}) is {got:?}, the closed form says {expected:?}");
        }
    }
}

fn wrap(x: i32, period: i32) -> i32 {
    x.rem_euclid(period)
}

#[test]
fn motion_tile_with_neutral_parameters_returns_the_input_byte_for_byte() {
    for space in ["screen", "layer"] {
        let fx = json!({"effect": "motion_tile"});
        let mut plain = card_film(fx.clone(), space);
        plain["layers"][0]["fx"] = json!([]);
        assert_eq!(render(card_film(fx, space)), render(plain), "{space}: 100 % tiles, 100 % output, centred, no phase");
    }
}

#[test]
fn an_output_of_300_percent_repeats_the_card_with_exact_pixels_at_every_tile_origin() {
    let px = render(card_film(tiled(json!({})), "layer"));
    assert_pixels(&px, 1, |x, y| card_rgb(wrap(x, CARD[0]), wrap(y, CARD[1])));
    // the nine tile origins, from the closed form: x in {-32, 0, 32}, y in {-16, 0, 16}, each the top-left quadrant
    for (tx, ty) in (-1..=1).flat_map(|j| (-1..=1).map(move |i| (i, j))) {
        let (x, y) = ((ORIGIN[0] + tx * CARD[0]) as usize, (ORIGIN[1] + ty * CARD[1]) as usize);
        assert_eq!(rgb(&px, x, y), card_rgb(0, 0), "tile ({tx}, {ty}) starts with the card's top-left texel at frame pixel ({x}, {y})");
    }
}

#[test]
fn mirror_edges_flips_every_other_tile_and_leaves_the_middle_one_alone() {
    let px = render(card_film(tiled(json!({"mirror_edges": "on"})), "layer"));
    assert_pixels(&px, 1, |x, y| {
        let flip = |v: i32, size: i32| if v.div_euclid(size) % 2 != 0 { size - 1 - wrap(v, size) } else { wrap(v, size) };
        card_rgb(flip(x, CARD[0]), flip(y, CARD[1]))
    });
    let middle = |x: usize, y: usize| rgb(&px, ORIGIN[0] as usize + x, ORIGIN[1] as usize + y);
    assert_eq!(middle(0, 0), card_rgb(0, 0), "the middle tile is the card as it is");
    assert_eq!(rgb(&px, ORIGIN[0] as usize + 32, ORIGIN[1] as usize), card_rgb(31, 0), "the tile to its right is its mirror image: it starts with the card's last column");
}

#[test]
fn a_phase_of_a_quarter_turn_shifts_each_row_a_quarter_of_a_tile_further_than_the_one_before() {
    // 90 degrees is a quarter of a tile: 8 px of the 32 px width a row. Row 0 is the one the tile centre is in, and the
    // shift grows by 8 px a row, downwards and upwards: row j reads the card from texel 8 j further along.
    let px = render(card_film(tiled(json!({"phase": 90, "horizontal_phase_shift": "on"})), "layer"));
    assert_pixels(&px, 1, |x, y| card_rgb(wrap(x + 8 * y.div_euclid(CARD[1]), CARD[0]), wrap(y, CARD[1])));
}

#[test]
fn without_the_horizontal_switch_the_phase_shifts_each_column_up_and_down() {
    // a quarter of the 16 px height is 4 px a column
    let px = render(card_film(tiled(json!({"phase": 90})), "layer"));
    assert_pixels(&px, 1, |x, y| card_rgb(wrap(x, CARD[0]), wrap(y + 4 * x.div_euclid(CARD[0]), CARD[1])));
}

#[test]
fn a_phase_of_a_whole_turn_is_a_whole_tile_and_changes_nothing() {
    let still = render(card_film(tiled(json!({"horizontal_phase_shift": "on"})), "layer"));
    let turned = render(card_film(tiled(json!({"phase": 360, "horizontal_phase_shift": "on"})), "layer"));
    assert_eq!(still, turned);
}

#[test]
fn moving_the_tile_centre_moves_the_tiles_with_it() {
    // centre 4 px right and 2 px up: the whole grid, and the card in it, is 4 px right and 2 px up
    let px = render(card_film(tiled(json!({"tile_center": [0.5 + 4.0 / 32.0, 0.5 - 2.0 / 16.0]})), "layer"));
    assert_pixels(&px, 1, |x, y| card_rgb(wrap(x - 4, CARD[0]), wrap(y + 2, CARD[1])));
}

#[test]
fn tiles_of_half_the_size_hold_the_whole_card_reduced_two_to_one() {
    // 16 x 8 tiles centred on the layer's middle: tile edges at 8 and 24 across, 4 and 12 down; each tile pixel is the
    // average of two card texels a side, which never straddle the card's quadrant boundary
    let px = render(card_film(tiled(json!({"tile_width": 50, "tile_height": 50})), "layer"));
    assert_pixels(&px, 1, |x, y| card_rgb(2 * wrap(x - 8, 16), 2 * wrap(y - 4, 8)));
    // and the centre moves that grid by the same 4 px and 2 px
    let moved = render(card_film(tiled(json!({"tile_width": 50, "tile_height": 50, "tile_center": [0.5 + 4.0 / 32.0, 0.5 - 2.0 / 16.0]})), "layer"));
    assert_pixels(&moved, 1, |x, y| card_rgb(2 * wrap(x - 8 - 4, 16), 2 * wrap(y - 4 + 2, 8)));
}

#[test]
fn an_output_of_50_percent_keeps_the_middle_half_of_the_layer_and_clears_the_rest() {
    let px = render(card_film(json!({"effect": "motion_tile", "output_width": 50, "output_height": 50}), "layer"));
    assert_pixels(&px, 1, |x, y| if (8..24).contains(&x) && (4..12).contains(&y) { card_rgb(x, y) } else { [0, 0, 0] });
}

#[test]
fn a_gutter_leaves_a_gap_of_its_colour_between_tiles_and_makes_the_pitch_tile_plus_gutter() {
    // tiles of 16 x 8 with a 4 px gutter: pitch 20 x 12; the tile pixels are the 16 (8) in the middle of each pitch
    let fx = tiled(json!({"tile_width": 50, "tile_height": 50, "gutter_size": 4, "gutter_color": "#ffffff"}));
    let px = render(card_film(fx, "layer"));
    assert_pixels(&px, 1, |x, y| {
        let (u, v) = (wrap(x - 6, 20), wrap(y - 2, 12));
        if (2..18).contains(&u) && (2..10).contains(&v) {
            card_rgb(2 * (u - 2), 2 * (v - 2))
        } else {
            [255, 255, 255]
        }
    });
}

#[test]
fn motion_tile_renders_the_same_twice() {
    let fx = tiled(json!({"mirror_edges": "on", "phase": 33, "horizontal_phase_shift": "on", "gutter_size": 3, "gutter_color": "#ff00ff80", "tile_center": [0.43, 0.61]}));
    for space in ["screen", "layer"] {
        assert_eq!(render(card_film(fx.clone(), space)), render(card_film(fx.clone(), space)), "{space}");
    }
}
