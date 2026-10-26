//! The colour pipeline on the GPU, in numbers: what a half-white layer, an additive blend, a blur across a hard edge and
//! a gradient show with `color: {linear_light: true}` and without it. The expected values are computed from the sRGB
//! curve's definition, not copied from a render.

use serde_json::{json, Value};

use super::testkit::{bgra, render};

const W: usize = 64;
const H: usize = 32;
const ROW: usize = 16;

/// The sRGB curve, from its definition (IEC 61966-2-1).
fn to_light(level: f64) -> f64 {
    let v = level / 255.0;
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

/// The level a light value is shown as (unrounded).
fn to_level(light: f64) -> f64 {
    255.0 * if light <= 0.003_130_8 { light * 12.92 } else { 1.055 * light.powf(1.0 / 2.4) - 0.055 }
}

fn film(color: Value, layers: Value) -> Value {
    let mut film = json!({"width": W, "height": H, "fps": 10, "dur": 1, "bg": "#000000", "layers": layers});
    if !color.is_null() {
        film["color"] = color;
    }
    film
}

fn linear() -> Value {
    json!({"linear_light": true})
}

/// The red channel at column `x` of the middle row.
fn level(px: &[u8], x: usize) -> f64 {
    f64::from(bgra(px, W, x, ROW)[2])
}

fn white_half() -> Value {
    json!([{"type": "solid", "color": "#ffffff", "opacity": 0.5}])
}

#[test]
fn half_white_over_black_shows_level_188_in_light_and_128_on_encoded_numbers() {
    let want = to_level(0.5);
    assert!((want - 187.5).abs() < 0.1, "half the light is level 187.5 on the display, not 127.5");
    let in_light = level(&render(film(linear(), white_half())), 8);
    assert!((in_light - want).abs() <= 1.0, "linear light: {in_light} vs {want}");
    let encoded = level(&render(film(Value::Null, white_half())), 8);
    assert!((encoded - 127.5).abs() <= 1.0, "encoded numbers: {encoded} vs 127.5");
}

#[test]
fn an_additive_blend_adds_light_not_encoded_numbers() {
    let layers = json!([{"type": "solid", "color": "#808080"}, {"type": "solid", "color": "#808080", "blend": "add"}]);
    let want = to_level(2.0 * to_light(128.0));
    let in_light = level(&render(film(linear(), layers.clone())), 8);
    assert!((in_light - want).abs() <= 1.0, "two level-128 greys added in light: {in_light} vs {want}");
    let encoded = level(&render(film(Value::Null, layers)), 8);
    assert!((encoded - 255.0).abs() <= 1.0, "added as encoded numbers they clip to 255: {encoded}");
}

/// A black left half and a white right half (the edge is at column 32) that the layer's blur then spreads.
fn edge_film(color: Value) -> Value {
    let mut f = film(color, json!([{"type": "precomp", "comp": "edge", "fx": [{"effect": "blur", "radius": 6}]}]));
    f["comps"] = json!({"edge": {"width": W, "height": H, "dur": 1, "layers": [
        {"type": "solid", "color": "#000000"},
        {"type": "solid", "color": "#ffffff", "size": [32, H], "pos": [48, 16]}
    ]}});
    f
}

#[test]
fn a_blur_across_a_hard_edge_averages_light_where_the_default_averages_encoded_numbers() {
    let in_light = render(edge_film(linear()));
    let encoded = render(edge_film(Value::Null));
    // the two pixels either side of the edge are symmetric about it, so their mean is the edge's midpoint
    let mid = |px: &[u8], f: fn(f64) -> f64| (f(level(px, 31)) + f(level(px, 32))) / 2.0;
    let light_mid = mid(&in_light, to_light);
    let encoded_mid = mid(&encoded, |l| l);
    assert!((light_mid - 0.5).abs() < 0.02, "in linear light the midpoint is half the light: {light_mid}");
    assert!((encoded_mid - 127.5).abs() < 2.0, "on encoded numbers the midpoint is 127.5: {encoded_mid}");
    // the same edge as the eye sees it: 255 * encode(0.5) = 187.5 against 127.5
    let shown = |px: &[u8]| (level(px, 31) + level(px, 32)) / 2.0;
    assert!(shown(&in_light) > shown(&encoded) + 20.0, "light {} against encoded {}", shown(&in_light), shown(&encoded));
}

fn ramp(color: Value) -> Vec<u8> {
    render(json!({"width": 256, "height": 8, "fps": 10, "dur": 1, "bg": "#000000", "layers": [
        {"type": "shape", "items": [{"rect": [0, 0, 256, 8], "fill": {"gradient": {"type": "linear", "from": [0, 0], "to": [256, 0], "stops": [[0, "#000000"], [1, "#ffffff"]]}}}]}
    ], "color": color}))
}

#[test]
fn a_gradient_in_light_follows_the_light_ramp_within_one_level_and_never_bands() {
    let px = ramp(linear());
    let at = |x: usize| f64::from(bgra(&px, 256, x, 4)[2]);
    let worst = (0..256).map(|x| (at(x) - to_level((x as f64 + 0.5) / 256.0)).abs()).fold(0.0, f64::max);
    assert!(worst <= 1.0, "the ramp is off the ideal light ramp by {worst} levels");
    let steps: Vec<f64> = (1..256).map(|x| at(x) - at(x - 1)).collect();
    assert!(steps.iter().all(|s| *s >= 0.0), "the ramp never goes back down");
    // the steepest step of the ideal ramp is where the curve is steepest, at the dark end; no step is longer than it
    let ideal = (1..256).map(|x| to_level((x as f64 + 0.5) / 256.0) - to_level((x as f64 - 0.5) / 256.0)).fold(0.0, f64::max);
    assert!(steps.iter().all(|s| *s <= ideal.ceil() + 1.0), "a step of {:?} is longer than the ideal {ideal}", steps.iter().copied().fold(0.0, f64::max));
}

#[test]
fn a_gradient_on_encoded_numbers_is_the_straight_ramp() {
    let px = ramp(Value::Null);
    let worst = (0..256).map(|x| (f64::from(bgra(&px, 256, x, 4)[2]) - 255.0 * (x as f64 + 0.5) / 256.0).abs()).fold(0.0, f64::max);
    assert!(worst <= 1.0, "off the straight ramp by {worst} levels");
}

#[test]
fn rendering_twice_gives_the_same_bytes_and_leaving_the_key_out_is_the_empty_block() {
    let layers = json!([{"type": "solid", "color": "#336699"}, {"type": "solid", "color": "#ffffff", "opacity": 0.4, "fx": [{"effect": "blur", "radius": 4}]}]);
    assert_eq!(render(film(linear(), layers.clone())), render(film(linear(), layers.clone())), "linear light renders the same twice");
    assert_eq!(render(film(Value::Null, layers.clone())), render(film(json!({}), layers)), "an empty block is the unmanaged film");
}

#[test]
fn an_effect_written_for_display_numbers_gives_the_same_result_in_light() {
    // contrast pivots on mid-grey as the eye sees it: level 96 at amount 2 is 255 * ((96/255 - 0.5) * 2 + 0.5)
    let layers = json!([{"type": "solid", "color": "#606060", "fx": [{"effect": "contrast", "amount": 2}]}]);
    let want = 255.0 * ((96.0 / 255.0 - 0.5) * 2.0 + 0.5);
    for (what, color) in [("encoded numbers", Value::Null), ("linear light", linear())] {
        let got = level(&render(film(color, layers.clone())), 8);
        assert!((got - want).abs() <= 1.0, "{what}: {got} vs {want}");
    }
}

#[test]
fn a_luma_matte_reads_the_brightness_the_eye_sees_in_light_too() {
    // a level-128 matte is 0.502 bright as displayed, so it lets 0.502 of a white layer through: shown in light that is
    // level 188 (and 128 when blending encoded numbers). Reading its brightness in light (0.216) would show 128 in both.
    let layers = json!([
        {"type": "solid", "id": "m", "color": "#808080", "hidden": true},
        {"type": "solid", "color": "#ffffff", "matte": {"source": "m", "mode": "luma"}}
    ]);
    let in_light = level(&render(film(linear(), layers.clone())), 8);
    let want = to_level(128.0 / 255.0);
    assert!((in_light - want).abs() <= 1.0, "light: {in_light} vs {want}");
    let encoded = level(&render(film(Value::Null, layers)), 8);
    assert!((encoded - 128.0).abs() <= 1.0, "encoded numbers: {encoded} vs 128");
}
