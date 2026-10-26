//! Dark levels in linear light. A film that keeps its pictures in light must show a grey it draws as that grey, the same
//! whatever the size of the solid or shape that draws it: the darks are where light is densest on the display's curve, so
//! storing them coarsely collapses neighbouring levels. Every expected value is the input level itself (an opaque grey over
//! black with the `normal` blend is the identity) or is computed from the sRGB curve's definition.

use serde_json::{json, Value};

use super::testkit::{bgra, render};

/// Swatches are `CELL` pixels apart in a `GRID` x `GRID` sheet, and `INK` pixels wide: much smaller than the frame.
const GRID: usize = 16;
const CELL: usize = 8;
const INK: usize = 4;
const SIDE: usize = GRID * CELL;

fn linear() -> Value {
    json!({"linear_light": true})
}

fn hex(level: usize) -> String {
    format!("#{level:02x}{level:02x}{level:02x}")
}

fn centre(level: usize) -> [usize; 2] {
    [(level % GRID) * CELL + CELL / 2, (level / GRID) * CELL + CELL / 2]
}

fn film(color: Value, layers: Vec<Value>) -> Value {
    json!({"width": SIDE, "height": SIDE, "fps": 10, "dur": 1, "bg": "#000000", "layers": layers, "color": color})
}

fn solid(level: usize, alpha: f64) -> Value {
    json!({"type": "solid", "color": hex(level), "size": [INK, INK], "pos": centre(level), "opacity": alpha})
}

fn shape(level: usize, alpha: f64) -> Value {
    let [x, y] = centre(level);
    let half = INK / 2;
    json!({"type": "shape", "opacity": alpha, "items": [{"rect": [x - half, y - half, INK, INK], "fill": {"color": hex(level)}}]})
}

/// The grey level (red, checked equal to green and blue) at the middle of every swatch of a sheet of 256 levels.
fn sheet(color: Value, make: fn(usize, f64) -> Value, alpha: f64) -> Vec<f64> {
    let px = render(film(color, (0..256).map(|l| make(l, alpha)).collect()));
    (0..256)
        .map(|l| {
            let [x, y] = centre(l);
            let [b, g, r, a] = bgra(&px, SIDE, x, y);
            assert!(b == g && g == r && a == 255, "level {l} came out as a colour: {b} {g} {r} {a}");
            f64::from(r)
        })
        .collect()
}

/// What each level that was drawn came out as, listed for an assertion message.
fn show(out: &[f64], levels: impl Iterator<Item = usize>) -> String {
    levels.map(|l| format!("{l}->{}", out[l])).collect::<Vec<_>>().join(" ")
}

/// Every level within one of itself, none of the darks (1..=32) off by more than that, and the steps in order and apart.
fn assert_identity(what: &str, out: &[f64]) {
    let off: Vec<usize> = (0..256).filter(|&l| (out[l] - l as f64).abs() > 1.0).collect();
    assert!(off.is_empty(), "{what}: {} levels are more than 1 off the level drawn: {}", off.len(), show(out, off.into_iter().take(40)));
    for (l, got) in out.iter().enumerate().take(33).skip(1) {
        assert!((got - l as f64).abs() <= 1.0, "{what}: dark level {l} came out as {got}");
    }
    let collapsed: Vec<usize> = (1..256).filter(|&l| out[l] <= out[l - 1]).collect();
    assert!(collapsed.is_empty(), "{what}: levels that show the same or less than the one below: {}", show(out, collapsed.iter().flat_map(|&l| [l - 1, l])));
}

#[test]
fn a_small_solid_shows_the_level_it_is_drawn_in_at_all_256_levels() {
    assert_identity("small solid", &sheet(linear(), solid, 1.0));
}

#[test]
fn a_small_shape_shows_the_level_it_is_drawn_in_at_all_256_levels() {
    assert_identity("small shape", &sheet(linear(), shape, 1.0));
}

#[test]
fn a_small_solid_is_the_same_on_encoded_numbers_as_it_was() {
    assert_identity("small solid, default", &sheet(Value::Null, solid, 1.0));
}

#[test]
fn a_full_frame_solid_shows_the_level_it_is_drawn_in() {
    // one render per level, so the dense sample: every dark, then every 15th level and the top of the range
    let levels = (0..=32).chain((33..255).step_by(15)).chain([254, 255]);
    let out: Vec<(usize, f64)> = levels
        .map(|l| {
            let px = render(film(linear(), vec![json!({"type": "solid", "color": hex(l)})]));
            (l, f64::from(bgra(&px, SIDE, 8, 8)[2]))
        })
        .collect();
    let off: Vec<String> = out.iter().filter(|(l, v)| (v - *l as f64).abs() > 1.0).map(|(l, v)| format!("{l}->{v}")).collect();
    assert!(off.is_empty(), "full-frame solids more than 1 off: {}", off.join(" "));
}

/// The sRGB curve's definition (IEC 61966-2-1): the level a quantity of light is shown as.
fn to_level(light: f64) -> f64 {
    255.0 * if light <= 0.003_130_8 { light * 12.92 } else { 1.055 * light.powf(1.0 / 2.4) - 0.055 }
}

/// The light a level shows (the curve's other direction).
fn to_light(level: f64) -> f64 {
    let v = level / 255.0;
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

#[test]
fn half_covered_white_is_half_the_light_whatever_the_size_of_what_covers_it() {
    let want = to_level(0.5);
    assert!((want - 187.5).abs() < 0.1, "half the light of white is level 187.5");
    for (what, make) in [("solid", solid as fn(usize, f64) -> Value), ("shape", shape)] {
        let px = render(film(linear(), vec![make(255, 0.5)]));
        let [x, y] = centre(255);
        let got = f64::from(bgra(&px, SIDE, x, y)[2]);
        assert!((got - want).abs() <= 1.0, "50% white small {what}: {got} against {want}");
    }
}

#[test]
fn half_covered_dark_greys_add_their_light_from_black_in_proportion() {
    // alpha is coverage, never encoded: a grey at half alpha over black is half the light of that grey
    let out = sheet(linear(), solid, 0.5);
    let off: Vec<String> = (8..=64).filter(|&l| (out[l] - to_level(to_light(l as f64) / 2.0)).abs() > 1.0).map(|l| format!("{l}->{}", out[l])).collect();
    assert!(off.is_empty(), "half-alpha darks are not half the light: {}", off.join(" "));
}

#[test]
fn a_gradient_across_a_small_layer_follows_the_light_ramp_without_banding() {
    const W: usize = 320;
    let px = render(json!({"width": W, "height": 8, "fps": 10, "dur": 1, "bg": "#000000", "color": linear(), "layers": [
        {"type": "shape", "items": [{"rect": [32, 0, 256, 8], "fill": {"gradient": {"type": "linear", "from": [32, 0], "to": [288, 0], "stops": [[0, "#000000"], [1, "#ffffff"]]}}}]}
    ]}));
    let at = |x: usize| f64::from(bgra(&px, W, 32 + x, 4)[2]);
    let worst = (0..256).map(|x| (at(x) - to_level((x as f64 + 0.5) / 256.0)).abs()).fold(0.0, f64::max);
    assert!(worst <= 1.0, "the ramp is off the ideal light ramp by {worst} levels");
    let steps: Vec<f64> = (1..256).map(|x| at(x) - at(x - 1)).collect();
    assert!(steps.iter().all(|s| *s >= 0.0), "the ramp goes back down");
    let ideal = (1..256).map(|x| to_level((x as f64 + 0.5) / 256.0) - to_level((x as f64 - 0.5) / 256.0)).fold(0.0, f64::max);
    let widest = steps.iter().copied().fold(0.0, f64::max);
    assert!(widest <= ideal.ceil() + 1.0, "a step of {widest} levels is longer than the ideal {ideal}");
}
