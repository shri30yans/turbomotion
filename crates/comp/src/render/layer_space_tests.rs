//! `effect_space: "layer"` measured on rendered frames: tails scale with the layer, content outside the frame feeds the
//! effect, a directional blur turns with the layer, the default is the old pipeline, and renders repeat exactly.

use serde_json::{json, Value};

use super::testkit::{bgra, render};

const W: usize = 400;
const H: usize = 300;
/// A pixel counts as part of a blur's tail while its red is at least this (of 255).
const TAIL_LEVEL: u8 = 3;
/// A tail measured at one scale may differ from its exact multiple by this many pixels (sampling at the edge).
const TOLERANCE: f64 = 2.0;

/// A white `size` square about the middle of the frame (or `pos`), scaled by `scale`, with `fx`, on black.
fn film(space: Option<&str>, size: f64, pos: [f64; 2], scale: f64, rotation: f64, fx: Value) -> Value {
    let mut film = json!({"width": W, "height": H, "fps": 10, "dur": 1, "bg": "#000000", "layers": [
        {"type": "solid", "color": "#ffffff", "size": [size, size], "anchor": [size / 2.0, size / 2.0], "pos": pos, "scale": scale, "rotation": rotation, "fx": fx}
    ]});
    if let Some(s) = space {
        film["effect_space"] = json!(s);
    }
    film
}

fn red(px: &[u8], x: usize, y: usize) -> u8 {
    bgra(px, W, x, y)[2]
}

/// How far past the square's right edge (`edge_x`) the blur still reaches along the row `y`.
fn tail(px: &[u8], edge_x: usize, y: usize) -> f64 {
    let last = (edge_x..W).take_while(|&x| red(px, x, y) >= TAIL_LEVEL).last().unwrap_or(edge_x);
    (last - edge_x) as f64
}

fn blurred_tail(space: &str, scale: f64) -> f64 {
    let px = render(film(Some(space), 60.0, [200.0, 150.0], scale, 0.0, json!([{"effect": "blur", "radius": 10}])));
    tail(&px, 200 + (30.0 * scale) as usize, 150)
}

#[test]
fn a_blur_tail_doubles_with_the_layer_scale_in_layer_space() {
    let (one, two) = (blurred_tail("layer", 1.0), blurred_tail("layer", 2.0));
    assert_eq!((one, two), (22.0, 45.0), "the measured tails, pinned");
    assert!(one >= 10.0, "a radius 10 blur reaches past the edge ({one})");
    assert!((two - 2.0 * one).abs() <= TOLERANCE, "scale 2 tail {two} px is twice the scale 1 tail {one} px (+-{TOLERANCE})");
}

#[test]
fn a_blur_tail_is_the_same_at_every_scale_in_screen_space_as_before() {
    let (one, two) = (blurred_tail("screen", 1.0), blurred_tail("screen", 2.0));
    assert_eq!((one, two), (22.0, 22.0), "the measured tails, pinned");
    assert!((two - one).abs() <= TOLERANCE, "screen tails stay {one} and {two} px whatever the scale");
}

/// A 100 px white square with `fx` centred on `pos`, at `scale`, in the given effect space.
fn square(space: &str, pos: [f64; 2], scale: f64, fx: Value) -> Vec<u8> {
    render(film(Some(space), 100.0, pos, scale, 0.0, fx))
}

/// The largest difference between `a`'s columns `0..cols` and `b`'s columns `shift..shift + cols` on row `y`.
fn column_gap(a: &[u8], b: &[u8], shift: usize, cols: usize, y: usize) -> u8 {
    (0..cols).map(|x| red(a, x, y).abs_diff(red(b, x + shift, y))).max().unwrap_or(0)
}

#[test]
fn a_blur_uses_content_outside_the_frame_in_layer_space() {
    let blur = json!([{"effect": "blur", "radius": 10}]);
    // the same square with half of it outside the frame, and wholly inside it 100 px further right
    let half_out = square("layer", [0.0, 150.0], 1.0, blur.clone());
    let inside = square("layer", [100.0, 150.0], 1.0, blur.clone());
    let gap = column_gap(&half_out, &inside, 100, 60, 150);
    assert!(gap <= 2, "columns 0..60 of the half-outside square match the whole square's by {gap} levels (tolerance 2)");
    assert!(red(&half_out, 0, 150) >= 250, "the frame's first column is inside the square's body: {}", red(&half_out, 0, 150));
    // the old pipeline blurs only what was inside the frame, so the first column fades
    let screen = square("screen", [0.0, 150.0], 1.0, blur);
    assert!(column_gap(&screen, &inside, 100, 60, 150) > 20, "screen space loses what was outside the frame");
}

#[test]
fn a_glow_and_a_blur_of_a_square_wholly_outside_the_frame_bleed_into_it_in_layer_space() {
    // the square's right edge is 10 px left of the frame; its halo (3 sigma = 30 px) reaches 20 px in
    for fx in [json!([{"effect": "glow", "radius": 10, "strength": 1}]), json!([{"effect": "blur", "radius": 10}])] {
        let out = square("layer", [-60.0, 150.0], 1.0, fx.clone());
        let inside = square("layer", [90.0, 150.0], 1.0, fx.clone());
        let gap = column_gap(&out, &inside, 150, 30, 150);
        assert!(gap <= 2, "{fx}: the halo in columns 0..30 matches the same square drawn in frame by {gap} levels (tolerance 2)");
        assert!(red(&out, 2, 150) >= 8, "{fx}: the halo is really there: {} at x = 2", red(&out, 2, 150));
        let screen = square("screen", [-60.0, 150.0], 1.0, fx.clone());
        assert_eq!(red(&screen, 2, 150), 0, "{fx}: screen space has nothing of the off-frame square to blur");
    }
}

#[test]
fn a_layer_scaled_into_the_frame_is_blurred_as_a_whole() {
    let blur = json!([{"effect": "blur", "radius": 10}]);
    let half_out = square("layer", [0.0, 150.0], 2.0, blur.clone());
    let inside = square("layer", [200.0, 150.0], 2.0, blur);
    let gap = column_gap(&half_out, &inside, 200, 100, 150);
    assert!(gap <= 2, "at scale 2 the half-outside square matches the whole one by {gap} levels (tolerance 2)");
    let screen = square("screen", [0.0, 150.0], 2.0, json!([{"effect": "blur", "radius": 10}]));
    assert!(column_gap(&screen, &inside, 200, 100, 150) > 20, "the comparison can fail: screen space differs");
}

/// How many pixels along row `y` (or column `x` when `vertical`) through the middle are lit.
fn span(px: &[u8], vertical: bool) -> usize {
    if vertical {
        (0..H).filter(|&y| red(px, 200, y) >= TAIL_LEVEL).count()
    } else {
        (0..W).filter(|&x| red(px, x, 150) >= TAIL_LEVEL).count()
    }
}

fn smeared(space: &str, rotation: f64) -> (usize, usize) {
    let px = render(film(Some(space), 20.0, [200.0, 150.0], 1.0, rotation, json!([{"effect": "directional_blur", "distance": 40, "angle": 0}])));
    (span(&px, false), span(&px, true))
}

#[test]
fn a_directional_blur_turns_with_the_layer_in_layer_space() {
    let (across, up) = smeared("layer", 0.0);
    assert!(across >= 55 && up <= 25, "unrotated: smeared across {across} px, {up} px tall (20 px square + 40 px smear)");
    let (across, up) = smeared("layer", 90.0);
    assert!(up >= 55 && across <= 25, "rotated 90 degrees the smear runs up: {up} px tall, {across} px across");
    let (across, up) = smeared("screen", 90.0);
    assert!(across >= 55 && up <= 25, "screen space keeps the smear along the frame's x: {across} px across, {up} px tall");
}

#[test]
fn the_default_is_the_screen_pipeline_and_a_render_repeats_exactly() {
    let fx = json!([{"effect": "blur", "radius": 10}, {"effect": "glow", "radius": 6, "strength": 1}]);
    let default = render(film(None, 60.0, [150.0, 120.0], 1.5, 20.0, fx.clone()));
    let screen = render(film(Some("screen"), 60.0, [150.0, 120.0], 1.5, 20.0, fx.clone()));
    assert!(default == screen, "leaving effect_space out is the same as \"screen\", byte for byte");
    assert!(default == render(film(None, 60.0, [150.0, 120.0], 1.5, 20.0, fx.clone())), "two renders of the default are identical");
    let layer = film(Some("layer"), 60.0, [150.0, 120.0], 1.5, 20.0, fx);
    assert!(render(layer.clone()) == render(layer), "two renders in layer space are identical");
}

#[test]
fn a_mask_is_drawn_on_the_layers_pixels_so_it_scales_with_the_layer() {
    // a 60 px square scaled 2 about the frame's middle; the mask keeps its left half (layer x 0..30)
    let mut film = film(Some("layer"), 60.0, [200.0, 150.0], 2.0, 0.0, json!([{"effect": "blur", "radius": 4}]));
    film["layers"][0]["masks"] = json!([{"path": {"rect": [0, 0, 30, 60]}}]);
    let px = render(film);
    assert!(red(&px, 170, 150) > 240, "left of the mask edge is the square: {}", red(&px, 170, 150));
    assert_eq!(red(&px, 205, 150), 0, "right of the mask edge (layer x 30 is screen x 200) nothing is left, not even the blur's halo");
    assert_eq!(red(&px, 260, 150), 0);
}
