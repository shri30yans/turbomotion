//! The exact blur on rendered frames: a hard edge is the closed-form error function in a display film and in a linear-light
//! one, the layer's blur grows with the layer in layer space, the legacy blur still does what it did, and a plugin's kernel
//! passes blur as the platform effect does.

use std::path::PathBuf;

use serde_json::{json, Value};

use super::testkit::{bgra, render, render_at};

const W: usize = 400;
const H: usize = 300;
const EDGE: f64 = 200.0;

/// The standard normal's distribution function (Abramowitz and Stegun 7.1.26, good to 1.5e-7).
fn phi(x: f64) -> f64 {
    let z = x / std::f64::consts::SQRT_2;
    let t = 1.0 / (1.0 + 0.327_591_1 * z.abs());
    let poly = t * (0.254_829_592 + t * (-0.284_496_736 + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429))));
    0.5 * (1.0 + (1.0 - poly * (-z * z).exp()).copysign(z))
}

/// The 8-bit sRGB level of `light`.
fn encode(light: f64) -> f64 {
    255.0 * if light <= 0.003_130_8 { 12.92 * light } else { 1.055 * light.powf(1.0 / 2.4) - 0.055 }
}

/// White on the right half of a black frame, blurred by `fx`, in a film with `extra` keys.
fn edge_film(fx: Value, extra: Value) -> Value {
    let mut film = json!({"width": W, "height": H, "fps": 10, "dur": 1, "bg": "#000000", "layers": [
        {"type": "solid", "color": "#ffffff", "size": [W / 2, H], "anchor": [0, 0], "pos": [EDGE, 0], "fx": [fx]}
    ]});
    film.as_object_mut().expect("an object").extend(extra.as_object().expect("an object").clone());
    film
}

fn row(px: &[u8], y: usize) -> Vec<f64> {
    (0..W).map(|x| f64::from(bgra(px, W, x, y)[2])).collect()
}

/// Pixels either side of the edge that are compared: far enough that the profile has reached black and white, near enough
/// that the picture's own edges, where a blur fades into transparency, are out of it.
const WINDOW: usize = 80;

/// The most levels any pixel of the row within `WINDOW` of the edge is from `want(d)`, `d` being the distance of the pixel's
/// middle from the edge.
fn worst(px: &[u8], want: impl Fn(f64) -> f64) -> f64 {
    let (from, to) = (EDGE as usize - WINDOW, EDGE as usize + WINDOW);
    row(px, H / 2).iter().enumerate().filter(|(x, _)| (from..to).contains(x)).map(|(x, v)| (v - want(x as f64 + 0.5 - EDGE)).abs()).fold(0.0, f64::max)
}

#[test]
fn a_hard_edge_is_the_error_function_within_a_level_in_a_display_film() {
    for sigma in [4.0, 8.0, 16.0, 24.0] {
        let px = render(edge_film(json!({"effect": "blur", "radius": sigma, "quality": "exact"}), json!({})));
        let miss = worst(&px, |d| 255.0 * phi(d / sigma));
        assert!(miss <= 1.0, "sigma {sigma}: {miss:.2} levels from the error function");
    }
}

#[test]
fn a_hard_edge_is_the_error_function_of_light_in_a_linear_light_film() {
    // the blur mixes light, so the middle of the edge shows the sRGB of one half, 188, and not 128
    for sigma in [4.0, 8.0, 16.0, 24.0] {
        let px = render(edge_film(json!({"effect": "blur", "radius": sigma, "quality": "exact"}), json!({"color": {"linear_light": true}})));
        let miss = worst(&px, |d| encode(phi(d / sigma)));
        assert!(miss <= 1.0, "sigma {sigma}: {miss:.2} levels from the encoded error function");
        let middle = row(&px, H / 2)[EDGE as usize];
        let want = encode(phi(0.5 / sigma));
        assert!((middle - want).abs() <= 1.0 && middle > 160.0, "the first white pixel shows the encoding of its light, {want:.1}, not of half of it (which is the display film's 128): {middle}");
    }
}

fn square(scale: f64, fx: Value, space: &str) -> Value {
    json!({"width": W, "height": H, "fps": 10, "dur": 1, "bg": "#000000", "effect_space": space, "layers": [
        {"type": "solid", "color": "#ffffff", "size": [40, 40], "anchor": [20, 20], "pos": [200, 150], "scale": scale, "fx": [fx]}
    ]})
}

#[test]
fn in_layer_space_the_exact_blur_doubles_with_a_layer_scaled_to_two_and_in_screen_space_it_does_not() {
    let blur = json!({"effect": "blur", "radius": 6, "quality": "exact"});
    let tails = |space: &str, scale: f64| {
        let px = render(square(scale, blur.clone(), space));
        let row: Vec<u8> = (0..W).map(|x| bgra(&px, W, x, 150)[2]).collect();
        let right = 200 + (20.0 * scale) as usize;
        (right..W).take_while(|&x| row[x] >= 3).last().map_or(0, |last| last - right)
    };
    let (one, two) = (tails("layer", 1.0), tails("layer", 2.0));
    assert!(two.abs_diff(2 * one) <= 3, "layer space: {one} px of tail at scale 1, {two} at scale 2");
    let (one, two) = (tails("screen", 1.0), tails("screen", 2.0));
    assert!(two.abs_diff(one) <= 3, "screen space: {one} px at scale 1, {two} at scale 2");
}

/// How far, past the right side of the square of `square`, the middle row is lit (3 levels of 255 or more).
fn reach_of(px: &[u8]) -> usize {
    (220..W).take_while(|&x| bgra(px, W, x, 150)[2] >= 3).last().map_or(0, |last| last - 220)
}

#[test]
fn an_animated_radius_follows_the_clock_of_the_frame() {
    let grow = json!({"effect": "blur", "radius": [[0, 2], [1, 12]], "quality": "exact"});
    let (early, late) = (reach_of(&render_at(square(1.0, grow.clone(), "screen"), &PathBuf::from("."), 0.0)), reach_of(&render_at(square(1.0, grow, "screen"), &PathBuf::from("."), 0.9)));
    assert!(late > 3 * early, "the radius is read at the time of the frame: {early} px, then {late}");
}

#[test]
fn a_film_that_does_not_ask_for_the_exact_blur_renders_the_legacy_one_and_the_same_film_twice_is_the_same_pixels() {
    let film = edge_film(json!({"effect": "blur", "radius": 12}), json!({}));
    let named = edge_film(json!({"effect": "blur", "radius": 12, "quality": "legacy"}), json!({}));
    assert_eq!(render(film.clone()), render(named), "`legacy` is what no `quality` means");
    assert_eq!(render(film.clone()), render(film));
    let exact = edge_film(json!({"effect": "blur", "radius": 12, "quality": "exact"}), json!({}));
    assert_eq!(render(exact.clone()), render(exact), "the exact blur is as deterministic");
}

fn plugin_dir(tag: &str, file: &str, source: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tm-blur-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("effects")).expect("a plugin folder");
    std::fs::write(dir.join("effects").join(file), source).expect("a plugin file");
    dir
}

/// A plugin that is one kernel pass and one pass that shows it.
const SOFT: &str = "// @effect soft\n// @summary Blurs with a kernel pass.\n// @bounds expand=sigma*4\n// @param sigma float 6 0 40 -- sigma\n// @pass halo kernel=gaussian radius=sigma from=image\n// @pass main\nhalf4 main(float2 xy) { return halo.eval(xy); }\n";

#[test]
fn a_kernel_pass_in_a_plugin_blurs_as_the_platform_effect_does() {
    let dir = plugin_dir("soft", "soft.sksl", SOFT);
    let sigma = 6.0;
    let plugin = render_at(edge_film(json!({"effect": "soft", "sigma": sigma}), json!({"effect_space": "layer"})), &dir, 0.0);
    let platform = render(edge_film(json!({"effect": "blur", "radius": sigma, "quality": "exact"}), json!({"effect_space": "layer"})));
    std::fs::remove_dir_all(dir).ok();
    let miss = worst(&plugin, |d| 255.0 * phi(d / sigma));
    assert!(miss <= 1.0, "the plugin's hard edge is {miss:.2} levels from the error function");
    assert!(plugin.iter().zip(&platform).all(|(a, b)| a == b), "and is the platform blur's pixels, level for level");
}

/// A frame-high white layer from x = 100 to 300, whose left edge `tilt_shift` blurs by how far each row is from its band.
fn film_of(fx: Value) -> Value {
    json!({"width": W, "height": 300, "fps": 10, "dur": 1, "bg": "#000000", "effect_space": "layer", "layers": [
        {"type": "solid", "color": "#ffffff", "size": [200, 300], "anchor": [0, 0], "pos": [100, 0], "fx": [fx]}
    ]})
}

fn tilted(quality: Option<&str>) -> Value {
    let mut fx = json!({"effect": "tilt_shift", "focus": 0.5, "band": 0.1, "blur": 24});
    if let Some(q) = quality {
        fx["quality"] = json!(q);
    }
    film_of(fx)
}

fn edge_row(px: &[u8], y: usize) -> Vec<f64> {
    (40..160).map(|x| f64::from(bgra(px, W, x, y)[2])).collect()
}

#[test]
fn exact_tilt_shift_keeps_its_band_sharp_blurs_the_far_rows_by_a_gaussian_and_the_rows_between_by_less() {
    let px = render(tilted(Some("exact")));
    let sharp = edge_row(&px, 150);
    assert_eq!((sharp[59], sharp[60]), (0.0, 255.0), "in the band the edge is a step: {:?}", &sharp[56..64]);
    // far from the band it is the blur of sigma blur/2 = 12, level for level
    let reference = render(film_of(json!({"effect": "blur", "radius": 12, "quality": "exact"})));
    let (far, want) = (edge_row(&px, 5), edge_row(&reference, 5));
    let miss = far.iter().zip(&want).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
    assert!(miss <= 1.0, "the far rows are {miss:.2} levels from the blur of sigma 12");
    // between them the edge is softer than a step and sharper than the far rows, and more so the further it is from the band
    let width = |y: usize| edge_row(&px, y).iter().filter(|v| **v > 8.0 && **v < 247.0).count();
    let (near, middle, far_width) = (width(150 - 60), width(150 - 100), width(5));
    assert!(0 < near && near < middle && middle < far_width, "the soft part of the edge widens with the distance from the band: {near}, {middle}, {far_width} px");
}

#[test]
fn tilt_shift_without_a_quality_is_the_legacy_tilt_shift_and_the_two_are_different_pictures() {
    let (named, left_out, exact) = (render(tilted(Some("legacy"))), render(tilted(None)), render(tilted(Some("exact"))));
    assert_eq!(named, left_out);
    assert_ne!(named, exact);
}
