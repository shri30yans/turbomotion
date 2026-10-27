//! What colour each copy of a palette cycle takes: the arithmetic of the three modes against closed forms written out
//! here, in both transfers, and the cycle read from a frame, with a keyed offset and a layer repeater.

use std::path::Path;

use serde_json::{json, Value};
use turbomotion_core::color::{Primaries, Profile, Transfer};

use super::*;
use crate::{
    eval::{Evaluator, LayerState},
    plan::{Plan, ROOT},
};

const LIGHT: Profile = Profile { primaries: Primaries::Srgb, transfer: Transfer::Linear };

const RED: Rgba = [255.0, 0.0, 0.0, 1.0];
const GREEN: Rgba = [0.0, 255.0, 0.0, 1.0];
const BLUE: Rgba = [0.0, 0.0, 255.0, 1.0];
const YELLOW: Rgba = [255.0, 255.0, 0.0, 1.0];

fn cycled(colours: &[Rgba], mode: CycleMode, offset: f64, reverse: bool, working: Profile) -> Cycled {
    let list = NonEmpty::new(colours.to_vec()).expect("a palette");
    Cycled { colours: if reverse { list.reversed() } else { list }, mode, offset, working }
}

/// sRGB to light and back, the IEC 61966-2-1 formulas.
fn to_light(level: f32) -> f64 {
    let v = f64::from(level) / 255.0;
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

fn to_level(light: f64) -> f64 {
    255.0 * if light <= 0.003_130_8 { light * 12.92 } else { 1.055 * light.powf(1.0 / 2.4) - 0.055 }
}

#[test]
fn step_copy_i_takes_colour_i_plus_offset_mod_n_for_whole_negative_and_fractional_offsets() {
    let palette = [RED, GREEN, BLUE, YELLOW];
    for offset in [0.0, 1.0, -1.0, 2.0, 5.0, -7.0, 0.5, 1.75, -0.5, -2.25] {
        for reverse in [false, true] {
            let c = cycled(&palette, CycleMode::Step, offset, reverse, Profile::SRGB);
            for copy in 0..12 {
                // written out: the palette index is floor(copy + offset) mod 4, counted from the last colour when reversed
                let index = (copy as f64 + offset).floor().rem_euclid(4.0) as usize;
                let expected = if reverse { palette[3 - index] } else { palette[index] };
                assert_eq!(c.colour(copy), expected, "copy {copy}, offset {offset}, reverse {reverse}");
            }
        }
    }
}

#[test]
fn ping_pong_runs_the_palette_forwards_then_backwards_without_doubling_the_ends() {
    let c = cycled(&[RED, GREEN, BLUE], CycleMode::PingPong, 0.0, false, Profile::SRGB);
    let names = |c: &Cycled, n: usize| -> Vec<usize> { (0..n).map(|i| [RED, GREEN, BLUE].iter().position(|p| *p == c.colour(i)).expect("a palette colour")).collect() };
    assert_eq!(names(&c, 9), [0, 1, 2, 1, 0, 1, 2, 1, 0]);
    let shifted = cycled(&[RED, GREEN, BLUE], CycleMode::PingPong, -1.0, false, Profile::SRGB);
    assert_eq!(names(&shifted, 6), [1, 0, 1, 2, 1, 0]);
    let single = cycled(&[RED], CycleMode::PingPong, 3.0, false, Profile::SRGB);
    assert_eq!((0..4).map(|i| single.colour(i)).collect::<Vec<_>>(), [RED; 4]);
}

#[test]
fn a_display_gradient_is_the_straight_mix_of_the_encoded_levels_and_wraps_to_the_first_colour() {
    let (a, b) = ([200.0, 40.0, 0.0, 1.0], [0.0, 120.0, 250.0, 1.0]);
    let c = cycled(&[a, b], CycleMode::Gradient, 0.25, false, Profile::SRGB);
    for copy in 0..6 {
        let u = (copy as f64 + 0.25).rem_euclid(2.0);
        let (from, to, by) = if u < 1.0 { (a, b, u) } else { (b, a, u - 1.0) };
        let got = c.colour(copy);
        for ch in 0..3 {
            let want = f64::from(from[ch]) + (f64::from(to[ch]) - f64::from(from[ch])) * by;
            assert!((f64::from(got[ch]) - want).abs() < 1e-3, "copy {copy} channel {ch}: {} against {want}", got[ch]);
        }
    }
}

#[test]
fn a_linear_light_gradient_mixes_light_and_the_midpoint_of_black_and_white_is_level_188() {
    let black_white = cycled(&[[0.0, 0.0, 0.0, 1.0], [255.0, 255.0, 255.0, 1.0]], CycleMode::Gradient, 0.5, false, LIGHT);
    assert!((black_white.colour(0)[0] - 187.516).abs() < 0.1, "{:?}", black_white.colour(0));
    let (a, b) = ([255.0, 64.0, 8.0, 1.0], [10.0, 200.0, 160.0, 1.0]);
    let c = cycled(&[a, b], CycleMode::Gradient, 0.0, false, LIGHT);
    for by in [0.0, 0.25, 0.5, 0.75] {
        let at = cycled(&[a, b], CycleMode::Gradient, by, false, LIGHT).colour(0);
        for ch in 0..3 {
            let want = to_level(to_light(a[ch]) + (to_light(b[ch]) - to_light(a[ch])) * by);
            assert!((f64::from(at[ch]) - want).abs() < 0.1, "by {by} channel {ch}: {} against {want}", at[ch]);
        }
    }
    assert!(c.colour(0).iter().zip(a).all(|(got, want)| (got - want).abs() < 0.1), "a gradient starts on its first colour: {:?}", c.colour(0));
}

#[test]
fn a_gradient_blends_alpha_straight_in_either_transfer() {
    for working in [Profile::SRGB, LIGHT] {
        let c = cycled(&[[10.0, 10.0, 10.0, 0.2], [10.0, 10.0, 10.0, 1.0]], CycleMode::Gradient, 0.5, false, working);
        assert!((c.colour(0)[3] - 0.6).abs() < 1e-6);
    }
}

fn frame(film: Value, t: f64) -> crate::eval::Frame {
    let plan = Plan::from_value(film, Path::new(".")).expect("the plan");
    Evaluator::new(&plan).frame(ROOT, t).expect("the frame")
}

fn tints(frame: &crate::eval::Frame, copies: usize) -> Vec<Rgba> {
    let Content::Shape { items } = &frame.layers[0].content else { panic!("a shape layer") };
    let tint = items[0].repeat.as_ref().and_then(|r| r.tint.as_ref()).expect("a tinted repeat");
    (0..copies).map(|i| tint.colours.colour(i)).collect()
}

fn shape_film(cycle: Value) -> Value {
    json!({"width": 200, "height": 100, "dur": 2, "layers": [{"type": "shape", "items": [{"rect": [0, 0, 20, 20], "fill": "#fff",
        "repeat": {"copies": 4, "offset": [30, 0], "palette": ["#ff0000", "#00ff00", "#0000ff", "#ffff00"], "cycle": cycle}}]}]})
}

#[test]
fn a_keyed_offset_moves_the_assignment_as_time_passes() {
    let film = shape_film(json!({"offset": [[0, 0], [1, 3, "none"]]}));
    let at = |t: f64| tints(&frame(film.clone(), t), 4);
    assert_eq!(at(0.0), [RED, GREEN, BLUE, YELLOW]);
    // offset 3 at t = 1: copy i takes colour (i + 3) mod 4
    assert_eq!(at(1.0), [YELLOW, RED, GREEN, BLUE]);
    // offset 1.5 at t = 0.5 holds the colour until it reaches the next whole number: floor(i + 1.5)
    assert_eq!(at(0.5), [GREEN, BLUE, YELLOW, RED]);
}

#[test]
fn an_offset_that_is_not_a_number_is_named_when_it_is_read() {
    let plan = Plan::from_value(shape_film(json!({"offset": "pow(0 - 1, 0.5)"})), Path::new(".")).expect("the plan");
    let error = Evaluator::new(&plan).frame(ROOT, 0.0).err().map(|e| format!("{e:#}")).unwrap_or_default();
    assert!(error.contains("offset") || error.contains("finite") || error.contains("zero"), "{error}");
}

fn layer<'f>(frame: &'f crate::eval::Frame, id: &str) -> &'f LayerState {
    frame.layers.iter().find(|l| l.id.as_deref() == Some(id)).unwrap_or_else(|| panic!("no layer {id}"))
}

#[test]
fn a_layer_repeater_with_a_palette_stamps_every_layer_of_the_row_with_its_place() {
    let film = json!({"width": 200, "height": 100, "dur": 1, "layers": [
        {"type": "solid", "id": "bar", "color": "#ffffff", "pos": [0, 0], "repeat": {"count": 4, "offset": [20, 0], "palette": ["#ff0000", "#00ff00", "#0000ff"], "cycle": {"offset": 1}}}]});
    let f = frame(film, 0.0);
    let colour = |id: &str| match &layer(&f, id).content {
        Content::Solid { color, .. } => *color,
        _ => panic!("a solid"),
    };
    // the original is copy 0; the copies are 1 to 3; offset 1 shifts the palette by one colour
    assert_eq!([colour("bar"), colour("bar~c1"), colour("bar~c2"), colour("bar~c3")], [GREEN, BLUE, RED, GREEN]);
}

fn planned(layer: Value) -> String {
    match Plan::from_value(json!({"width": 100, "height": 100, "dur": 1, "layers": [layer]}), Path::new(".")) {
        Ok(_) => String::new(),
        Err(e) => format!("{e:#}"),
    }
}

#[test]
fn a_swatch_on_a_layer_with_nothing_to_colour_is_refused_when_the_film_is_planned() {
    let swatch = |paint: &str| json!({"palette": ["#f00"], "index": 0, "paint": paint});
    assert!(planned(json!({"type": "null", "swatch": swatch("fill")})).contains("a swatch colours a text, solid or shape layer, not null"));
    assert!(planned(json!({"type": "solid", "color": "#fff", "swatch": swatch("stroke")})).contains("a solid has a colour and no stroke"));
    assert!(planned(json!({"type": "text", "text": "x", "swatch": swatch("stroke")})).contains("has no `stroke`"));
    assert!(planned(json!({"type": "shape", "items": [{"rect": [0, 0, 9, 9], "fill": {"gradient": {"stops": [[0, "#000"], [1, "#fff"]]}}}], "swatch": swatch("fill")})).contains("no item of this shape layer has a paint"));
    assert_eq!(planned(json!({"type": "text", "text": "x", "stroke": {"color": "#fff", "width": 2}, "swatch": swatch("both")})), "");
}
