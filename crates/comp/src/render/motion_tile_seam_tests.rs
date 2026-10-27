//! `motion_tile`: what happens at the seams (sub-pixel centres, taps that must not leave their tile, a window that reaches
//! past the layer), animation, both effect spaces, a scaled and turned layer, the colour encoding, and bad parameters.
//! The card, its frame and the closed forms are explained in `fx_motion_tile_tests.rs`.

use std::path::Path;

use serde_json::{json, Value};

use super::fx_motion_tile_tests::{assert_pixels, card_film, card_rgb, rgb, tiled, CARD, FRAME, ORIGIN};
use super::testkit::{render, render_at};
use crate::plan::Plan;

fn wrap(x: i32, period: i32) -> i32 {
    x.rem_euclid(period)
}

#[test]
fn a_centre_off_by_a_third_of_a_pixel_blends_two_texels_and_keeps_the_seam_columns_exact() {
    // tile = card, centre 0.3 px right: tiles start 0.3 px past whole pixels, so the first pixel of each tile reads the
    // card at 0.2 px, where a bilinear tap would reach texel -1, the empty space beside the card. Each tap is held inside
    // the card, so that pixel is the card's first texel column exactly and the last pixel of the tile before is its last.
    let px = render(card_film(tiled(json!({"tile_center": [0.5 + 0.3 / 32.0, 0.5]})), "layer"));
    let at = |lx: i32, ly: i32| rgb(&px, (ORIGIN[0] + lx) as usize, (ORIGIN[1] + ly) as usize);
    for tile in -1..=1 {
        for ly in [0, 3, 8, 12] {
            let row = wrap(ly, CARD[1]);
            assert_eq!(at(tile * CARD[0], ly), card_rgb(0, row), "the first pixel of tile {tile} is the card's first texel column, opaque");
            if tile > -1 {
                assert_eq!(at(tile * CARD[0] - 1, ly), card_rgb(31, row), "the pixel before it is the card's last texel column");
            }
            assert_eq!(render_alpha(&px, (ORIGIN[0] + tile * CARD[0]) as usize, (ORIGIN[1] + ly) as usize), 255);
        }
    }
    // the red/green boundary of the card, between texels 15 and 16: pixel 16 reads the card at 16.2, 0.3 red + 0.7 green
    let want = [0.3 * 224.0 + 0.7 * 32.0, 0.3 * 32.0 + 0.7 * 192.0, 0.3 * 32.0 + 0.7 * 32.0];
    let got = at(16, 2);
    assert!(got.iter().zip(&want).all(|(g, w)| (f64::from(*g) - w).abs() <= 1.5), "pixel 16 read at 16.2: {got:?}, expected {want:?}");
    assert_eq!(at(15, 2), card_rgb(15, 2), "pixel 15 reads at 15.2, between two red texels");
}

#[test]
fn moving_the_centre_by_whole_pixels_scrolls_the_card_around_with_no_gap() {
    // tile = card, centre 4 px right: the card moves 4 px right and what leaves one tile's edge enters the next
    let px = render(card_film(tiled(json!({"tile_center": [0.5 + 4.0 / 32.0, 0.5]})), "layer"));
    assert_pixels(&px, 1, |x, y| card_rgb(wrap(x - 4, CARD[0]), wrap(y, CARD[1])));
    assert!((0..FRAME[0]).all(|x| render_alpha(&px, x, 24) == 255), "no transparent seam");
}

fn render_alpha(px: &[u8], x: usize, y: usize) -> u8 {
    super::testkit::bgra(px, FRAME[0], x, y)[3]
}

#[test]
fn the_centre_and_the_phase_animate_with_keys() {
    // at t = 0.5 the centre is 4 px right (halfway to 8) and the phase 90 degrees (halfway to 180)
    let fx = tiled(json!({"tile_center": {"keys": [[0, [0.5, 0.5]], [1, [0.5 + 8.0 / 32.0, 0.5], "linear"]]}}));
    let (start, half) = (render_at(card_film(fx.clone(), "layer"), Path::new("."), 0.0), render_at(card_film(fx, "layer"), Path::new("."), 0.5));
    assert_pixels(&start, 1, |x, y| card_rgb(wrap(x, CARD[0]), wrap(y, CARD[1])));
    assert_pixels(&half, 1, |x, y| card_rgb(wrap(x - 4, CARD[0]), wrap(y, CARD[1])));
    let fx = tiled(json!({"phase": [[0, 0], [1, 180, "linear"]], "horizontal_phase_shift": "on"}));
    let half = render_at(card_film(fx, "layer"), Path::new("."), 0.5);
    assert_pixels(&half, 1, |x, y| card_rgb(wrap(x + 8 * y.div_euclid(CARD[1]), CARD[0]), wrap(y, CARD[1])));
}

/// The frame filled by a card as big as the frame (so screen space, which tiles the frame, and layer space, which tiles
/// the layer, are looking at the same picture).
fn frame_card_film(fx: Value, space: &str) -> Value {
    let (w, h) = (FRAME[0] as i32, FRAME[1] as i32);
    let quadrant = |hex: &str, i: i32| json!({"type": "solid", "color": hex, "size": [w / 2, h / 2], "pos": [w / 4 + w / 2 * (i % 2), h / 4 + h / 2 * (i / 2)]});
    json!({"width": w, "height": h, "fps": 10, "dur": 1, "bg": "#000000", "effect_space": space,
        "comps": {"card": {"width": w, "height": h, "dur": 1, "layers": [quadrant("#e02020", 0), quadrant("#20c020", 1), quadrant("#2040e0", 2), quadrant("#e0e020", 3)]}},
        "layers": [{"type": "precomp", "comp": "card", "pos": [w / 2, h / 2], "fx": [fx]}]})
}

#[test]
fn layer_space_and_screen_space_draw_the_same_tiles_over_the_frame() {
    let uses = [
        tiled(json!({"tile_width": 50, "tile_height": 50})),
        tiled(json!({"tile_width": 50, "tile_height": 25, "mirror_edges": "on", "tile_center": [0.4, 0.55]})),
        tiled(json!({"tile_width": 40, "tile_height": 40, "phase": 90, "horizontal_phase_shift": "on", "gutter_size": 3, "gutter_color": "#ffffff"})),
        json!({"effect": "motion_tile", "output_width": 50, "output_height": 60}),
    ];
    for fx in uses {
        let (layer, screen) = (render(frame_card_film(fx.clone(), "layer")), render(frame_card_film(fx.clone(), "screen")));
        let worst = layer.chunks(4).zip(screen.chunks(4)).flat_map(|(a, b)| a.iter().zip(b).map(|(x, y)| x.abs_diff(*y))).max().unwrap_or(0);
        assert!(worst <= 1, "{fx}: the two spaces differ by {worst} levels");
    }
}

#[test]
fn a_layer_scaled_by_two_and_turned_a_quarter_keeps_its_pattern_in_layer_space() {
    // the card layer (32 x 16) with 300 % output is a 96 x 48 picture; scale 2 and a 90 degree turn about the frame's
    // middle put layer offset (a, b) from the middle at frame offset 2 * (-b, a)
    let mut film = card_film(tiled(json!({})), "layer");
    film["width"] = json!(256);
    film["height"] = json!(256);
    film["layers"][0]["pos"] = json!([128, 128]);
    film["layers"][0]["scale"] = json!(2);
    film["layers"][0]["rotation"] = json!(90);
    let px = render(film);
    for (tx, ty, qx, qy) in (-1..=1).flat_map(|j| (-1..=1).flat_map(move |i| (0..4).map(move |q| (i, j, q % 2, q / 2)))) {
        // the centre of quadrant (qx, qy) of the card in tile (tx, ty), as an offset from the card's middle
        let (a, b) = (f64::from(tx * CARD[0] + (2 * qx - 1) * CARD[0] / 4), f64::from(ty * CARD[1] + (2 * qy - 1) * CARD[1] / 4));
        let (x, y) = ((128.0 - 2.0 * b).round() as usize, (128.0 + 2.0 * a).round() as usize);
        let got = {
            let [bb, g, r, _] = super::testkit::bgra(&px, 256, x, y);
            [r, g, bb]
        };
        let want = card_rgb(qx * CARD[0] / 2, qy * CARD[1] / 2);
        assert!(got.iter().zip(&want).all(|(a, b)| a.abs_diff(*b) <= 1), "quadrant ({qx},{qy}) of tile ({tx},{ty}) at frame ({x},{y}): {got:?}, expected {want:?}");
    }
}

#[test]
fn a_half_pixel_slide_is_blended_where_the_film_is_in_light_and_where_it_is_not() {
    // pixel 16 reads the card at 16.0, between texels 15 (red, 224) and 16 (green, 32): half and half. As encoded numbers 128; as light, the mean of
    // the two lights, shown again as a number.
    let light = |level: f64| if level / 255.0 <= 0.04045 { level / 255.0 / 12.92 } else { ((level / 255.0 + 0.055) / 1.055).powf(2.4) };
    let shown = |l: f64| 255.0 * if l <= 0.003_130_8 { 12.92 * l } else { 1.055 * l.powf(1.0 / 2.4) - 0.055 };
    let fx = tiled(json!({"tile_center": [0.5 + 0.5 / 32.0, 0.5]}));
    let red_at_boundary = |film: Value| f64::from(rgb(&render(film), (ORIGIN[0] + 16) as usize, ORIGIN[1] as usize + 2)[0]);
    let encoded = red_at_boundary(card_film(fx.clone(), "layer"));
    let mut in_light = card_film(fx, "layer");
    in_light["color"] = json!({"linear_light": true});
    let lit = red_at_boundary(in_light);
    assert!((encoded - 128.0).abs() <= 1.0, "encoded: {encoded}");
    let want = shown((light(224.0) + light(32.0)) / 2.0);
    assert!((lit - want).abs() <= 1.5, "in light: {lit}, the mean of the two lights is {want:.1}");
}

/// A card of one pixel stripes: white at even columns, black at odd ones.
fn striped_film(fx: Value, color: Value) -> Value {
    let stripes: Vec<Value> = (0..CARD[0] / 2).map(|k| json!({"type": "solid", "color": "#ffffff", "size": [1, CARD[1]], "pos": [f64::from(2 * k) + 0.5, f64::from(CARD[1]) / 2.0]})).collect();
    let mut film = card_film(fx, "layer");
    film["comps"]["card"]["layers"] = json!(stripes);
    film["color"] = color;
    film
}

#[test]
fn a_reduced_tile_is_the_average_of_the_pixels_it_covers_in_the_encoding_the_film_is_in() {
    // tiles half as wide as the card cover two pixel stripes a pixel: black and white, so half the light. As encoded
    // numbers that is 128; the film in linear light shows the half light as 188.
    let fx = json!({"effect": "motion_tile", "tile_width": 50});
    let level = |film: Value| {
        let px = render(film);
        let all: Vec<u8> = (0..CARD[0]).map(|lx| rgb(&px, (ORIGIN[0] + lx) as usize, (ORIGIN[1] + 5) as usize)[0]).collect();
        (all.iter().copied().min().unwrap_or(0), all.iter().copied().max().unwrap_or(0))
    };
    let (lo, hi) = level(striped_film(fx.clone(), Value::Null));
    assert!(lo >= 127 && hi <= 128, "encoded: every pixel across the card is {lo}..{hi}");
    let (lo, hi) = level(striped_film(fx, json!({"linear_light": true})));
    assert!(lo >= 187 && hi <= 189, "in light: every pixel across the card is {lo}..{hi}");
}

fn plan_error(fx: Value) -> String {
    Plan::from_value(card_film(fx, "layer"), Path::new(".")).err().map(|e| format!("{e:#}")).unwrap_or_default()
}

#[test]
fn a_mistyped_parameter_or_option_names_the_valid_ones() {
    let e = plan_error(json!({"effect": "motion_tile", "tile_widht": 50}));
    assert!(e.contains("tile_widht") && e.contains("tile_width"), "{e}");
    let e = plan_error(json!({"effect": "motion_tile", "mirror_edges": "yes"}));
    assert!(e.contains("\"yes\" is not an option") && e.contains("off, on"), "{e}");
    let e = plan_error(json!({"effect": "motion_tile", "tile_center": 3}));
    assert!(e.contains("tile_center"), "{e}");
}

#[test]
fn a_tile_width_below_its_range_is_held_at_the_smallest_tile() {
    // tile width is at least 1 %, so 0 reads as 1 % (the other end of a range is pinned in `eval/buffer_motion_tile_tests.rs`)
    assert_eq!(render(card_film(tiled(json!({"tile_width": 0})), "layer")), render(card_film(tiled(json!({"tile_width": 1})), "layer")));
}
