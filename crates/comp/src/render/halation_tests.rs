//! `film_halation` on rendered frames: the same picture in both effect spaces, a halo that scales with the layer in layer
//! space and not in screen space, a threshold that is light itself in a linear-light film, the tint's hue in the tail,
//! and renders that repeat exactly. The point-source energy and falloff numbers are in `core`'s `halation_tests`.

use serde_json::{json, Value};

use super::testkit::{bgra, render};

const W: usize = 400;
const H: usize = 300;
const ROW: usize = 150;
/// A pixel counts as part of the halo while its red is at least this (of 255).
const TAIL_LEVEL: u8 = 3;

fn halation(extra: Value) -> Value {
    let mut fx = json!({"effect": "film_halation", "threshold": 0.5, "radius": 20, "strength": 0.6});
    fx.as_object_mut().expect("an object").extend(extra.as_object().expect("an object").clone());
    fx
}

/// A `size` square of grey `level` about the middle, scaled by `scale`, with `fx`, in a film with `extra` keys, on black.
fn film(level: u8, size: f64, scale: f64, fx: Value, extra: Value) -> Value {
    let mut film = json!({"width": W, "height": H, "fps": 10, "dur": 1, "bg": "#000000", "layers": [
        {"type": "solid", "color": format!("#{level:02x}{level:02x}{level:02x}"), "size": [size, size], "anchor": [size / 2.0, size / 2.0], "pos": [200, 150], "scale": scale, "fx": [fx]}
    ]});
    film.as_object_mut().expect("an object").extend(extra.as_object().expect("an object").clone());
    film
}

fn channel(px: &[u8], x: usize, c: usize) -> u8 {
    bgra(px, W, x, ROW)[2 - c]
}

/// How far past the square's right edge the halo reaches along the middle row.
fn tail(px: &[u8], edge_x: usize) -> usize {
    (edge_x..W).take_while(|&x| channel(px, x, 0) >= TAIL_LEVEL).last().map_or(0, |last| last - edge_x)
}

fn both_spaces(f: impl Fn(&str) -> Value) -> (Vec<u8>, Vec<u8>) {
    (render(f("layer")), render(f("screen")))
}

#[test]
fn both_effect_spaces_draw_the_same_halo() {
    let (layer, screen) = both_spaces(|space| film(255, 60.0, 1.0, halation(json!({})), json!({"effect_space": space})));
    let worst = layer.iter().zip(&screen).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0);
    assert!(worst <= 1, "layer space and screen space differ by at most {worst} levels");
    assert!(tail(&layer, 230) >= 30, "and the halo is there: {} px past the edge", tail(&layer, 230));
}

#[test]
fn the_halo_grows_with_the_layer_in_layer_space_and_keeps_its_size_in_screen_space() {
    let tails = |space: &str, scale: f64| tail(&render(film(255, 60.0, scale, halation(json!({})), json!({"effect_space": space}))), 200 + (30.0 * scale) as usize);
    let (layer_1, layer_2) = (tails("layer", 1.0), tails("layer", 2.0));
    let (screen_1, screen_2) = (tails("screen", 1.0), tails("screen", 2.0));
    assert!(layer_2.abs_diff(2 * layer_1) <= 3, "layer space: {layer_1} px at scale 1 and {layer_2} px at scale 2");
    assert!(screen_2.abs_diff(screen_1) <= 3, "screen space: {screen_1} px at scale 1 and {screen_2} px at scale 2");
}

#[test]
fn the_threshold_is_light_in_a_linear_light_film_and_encoded_numbers_otherwise() {
    // level 160 is 0.627 as a number and 0.352 as light: above a threshold of 0.5 in one, below it in the other
    let halo = |color: Value| channel(&render(film(160, 60.0, 1.0, halation(json!({"strength": 1})), color)), 240, 0);
    assert!(halo(json!({})) > 10, "encoded numbers: 0.627 is over 0.5 ({})", halo(json!({})));
    assert_eq!(halo(json!({"color": {"linear_light": true}})), 0, "light: 0.352 is under 0.5");
    let bright = channel(&render(film(200, 60.0, 1.0, halation(json!({"strength": 1})), json!({"color": {"linear_light": true}}))), 240, 0);
    assert!(bright > 10, "level 200 is 0.578 of light, over 0.5 ({bright})");
}

#[test]
fn the_tail_has_the_tints_hue() {
    let px = render(film(255, 60.0, 1.0, halation(json!({"strength": 1, "color": [255, 90, 40]})), json!({})));
    let (r, g, b) = (f64::from(channel(&px, 236, 0)), f64::from(channel(&px, 236, 1)), f64::from(channel(&px, 236, 2)));
    assert!((60.0..255.0).contains(&r), "red {r} is measurable and not clipped");
    assert!((g / r - 90.0 / 255.0).abs() < 0.04 && (b / r - 40.0 / 255.0).abs() < 0.04, "green/red {} blue/red {}", g / r, b / r);
}

#[test]
fn the_same_film_renders_the_same_bytes() {
    let f = film(255, 60.0, 1.0, halation(json!({})), json!({"effect_space": "layer"}));
    assert_eq!(render(f.clone()), render(f));
}
