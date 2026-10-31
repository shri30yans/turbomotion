//! Tapered strokes measured on rendered frames: the width of the stroke across the path at points along it against the
//! width the taper asks for, written out here from its numbers; in encoded and linear-light films, in both effect spaces.

use serde_json::{json, Value};
use skia_safe::{ContourMeasureIter, Path};

use super::testkit::{bgra, render};
use crate::{plan::geometry::PathGeo, schema::PathSpec, shape::stroke::coverage::Coverage};

const W: usize = 480;
const H: usize = 240;
/// How far the width of a stroke may differ from the one asked for, in pixels.
const TOLERANCE: f64 = 0.25;
/// An easing curve: the fraction of the way through it.
type Ease = fn(f64) -> f64;

fn to_light(level: f64) -> f64 {
    let v = level / 255.0;
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

/// The four films of every test: encoded or linear light, effects on the picture or on the layer.
fn films(items: Value) -> Vec<(String, Value)> {
    let mut out = vec![];
    for light in [false, true] {
        for space in ["screen", "layer"] {
            let mut film = json!({"width": W, "height": H, "fps": 10, "dur": 1, "bg": "#000000", "effect_space": space, "layers": [{"type": "shape", "items": items}]});
            if light {
                film["color"] = json!({"linear_light": true});
            }
            if space == "layer" {
                film["layers"][0]["masks"] = json!([{"path": {"rect": [0, 0, W, H]}}]);
            }
            out.push((format!("{} / {space}", if light { "linear light" } else { "encoded" }), film));
        }
    }
    out
}

/// White coverage of each pixel: the level in a film of encoded numbers; the light in linear light, where an edge pixel
/// is blended in light.
fn coverage(px: &[u8], light: bool) -> Coverage {
    Coverage::new(W as i32, H as i32, (0..W * H).map(|i| f64::from(px[i * 4 + 2])).map(|level| if light { to_light(level) } else { level / 255.0 }).collect())
}

/// The ruler: the path itself, as the engine builds it, to read points and normals off by distance.
fn ruler(spec: Value) -> Path {
    PathGeo::build(&serde_json::from_value::<PathSpec>(spec).expect("a path")).expect("geometry").at(0.0)
}

fn length(path: &Path) -> f64 {
    f64::from(ContourMeasureIter::new(path, false, None).next().expect("a contour").length())
}

/// Render `items` in every film and compare the width measured at each distance with `expected(distance)`.
fn check(items: Value, path: &Path, full: f64, distances: &[f64], expected: impl Fn(f64) -> f64) {
    let contour = ContourMeasureIter::new(path, false, None).next().expect("a contour");
    for (name, film) in films(items) {
        let cover = coverage(&render(film), name.starts_with("linear"));
        for &s in distances {
            let (p, t) = contour.pos_tan(s as f32).expect("a point on the path");
            let width = cover.width_across((f64::from(p.x), f64::from(p.y)), (-f64::from(t.y), f64::from(t.x)), full);
            assert!((width - expected(s)).abs() <= TOLERANCE, "{name}, {s:.0} px along: measured {width:.3}, expected {:.3}", expected(s));
        }
    }
}

fn line_item(taper: Value) -> Value {
    json!([{"d": "M 40 120 L 440 120", "stroke": {"color": "#ffffff", "width": 20}, "taper": taper}])
}

fn line() -> Path {
    ruler(json!({"d": "M 40 120 L 440 120"}))
}

#[test]
fn the_width_at_the_start_the_middle_and_the_end_is_the_one_asked_for_for_several_eases() {
    let eases: [(&str, Ease); 3] = [("none", |u| u), ("power2.out", |u| 1.0 - (1.0 - u).powi(3)), ("sine.inOut", |u| -((std::f64::consts::PI * u).cos() - 1.0) / 2.0)];
    for (name, ease) in eases {
        let taper = json!({"start": {"length": 100, "width": 0}, "end": {"length": 80, "width": 6}, "ease": name});
        // measured from each tip: the width rises from the tip's width to the full width
        let expected = |s: f64| if s < 100.0 { 20.0 * ease(s / 100.0) } else if s > 320.0 { 6.0 + 14.0 * ease((400.0 - s) / 80.0) } else { 20.0 };
        check(line_item(taper), &line(), 20.0, &[3.0, 25.0, 50.0, 99.0, 200.0, 321.0, 360.0, 397.0], expected);
    }
}

#[test]
fn a_curved_path_is_tapered_by_the_distance_along_the_curve() {
    let spec = json!({"d": "M 40 220 C 40 20 440 20 440 220"});
    let path = ruler(spec.clone());
    let len = length(&path);
    // GSAP's power2 is a cubic
    let ease = |u: f64| if u < 0.5 { 4.0 * u.powi(3) } else { 1.0 - (-2.0 * u + 2.0).powi(3) / 2.0 };
    let taper = json!({"start": {"length": {"fraction": 0.3}, "width": 0}, "end": {"length": {"fraction": 0.2}, "width": 4}, "ease": "power2.inOut"});
    let mut item = spec;
    item["stroke"] = json!({"color": "#ffffff", "width": 24});
    item["taper"] = taper;
    let expected = |s: f64| if s < 0.3 * len { 24.0 * ease(s / (0.3 * len)) } else if len - s < 0.2 * len { 4.0 + 20.0 * ease((len - s) / (0.2 * len)) } else { 24.0 };
    let at: Vec<f64> = [0.01, 0.1, 0.2, 0.3, 0.5, 0.7, 0.8, 0.9, 0.99].iter().map(|u| u * len).collect();
    check(json!([item]), &path, 24.0, &at, expected);
}

#[test]
fn a_tip_width_given_as_a_fraction_is_a_fraction_of_the_full_width() {
    // the start fades in over a quarter of the 400 px path from half the width; the width is linear in the distance
    let taper = json!({"start": {"length": {"fraction": 0.25}, "width": {"fraction": 0.5}}});
    check(line_item(taper), &line(), 20.0, &[2.0, 30.0, 70.0, 99.0, 150.0, 390.0], |s| if s < 100.0 { 10.0 + 10.0 * s / 100.0 } else { 20.0 });
}

#[test]
fn lengths_longer_than_half_the_path_are_held_to_half() {
    let taper = json!({"start": {"length": 5000, "width": 0}, "end": {"length": 5000, "width": 0}});
    check(line_item(taper), &line(), 20.0, &[20.0, 100.0, 200.0, 300.0, 380.0], |s| 20.0 * s.min(400.0 - s) / 200.0);
}

#[test]
fn a_closed_path_is_tapered_from_its_start_point_and_pinches_there_from_both_sides() {
    let spec = json!({"ellipse": [240, 120, 100, 100]});
    let path = ruler(spec.clone());
    let len = length(&path);
    let mut item = spec;
    item["stroke"] = json!({"color": "#ffffff", "width": 16});
    item["taper"] = json!({"start": {"length": 150, "width": 0}, "end": {"length": 100, "width": 0}});
    let expected = |s: f64| if s < 150.0 { 16.0 * s / 150.0 } else if len - s < 100.0 { 16.0 * (len - s) / 100.0 } else { 16.0 };
    check(json!([item]), &path, 16.0, &[5.0, 50.0, 100.0, 150.0, 300.0, len - 100.0, len - 50.0, len - 5.0], expected);
}

#[test]
fn a_tapered_stroke_follows_the_trimmed_outline() {
    // trimmed to the first half (200 px), the end taper is the last 50 px of that half
    let item = json!([{"d": "M 40 120 L 440 120", "stroke": {"color": "#ffffff", "width": 20}, "trim": {"end": 0.5}, "taper": {"end": {"length": 50, "width": 0}}}]);
    check(item, &line(), 20.0, &[10.0, 100.0, 150.0, 175.0, 195.0], |s| if s > 150.0 { 20.0 * (200.0 - s) / 50.0 } else { 20.0 });
}

#[test]
fn a_tapered_stroke_takes_the_colour_of_its_copy() {
    let palette = ["#e63946", "#2a9d8f", "#9b5de5"];
    let items = json!([{"d": "M 40 40 L 440 40", "stroke": {"color": "#ffffff", "width": 20}, "taper": {"start": {"length": 100}, "end": {"length": 100}},
        "repeat": {"copies": 3, "offset": [0, 70], "palette": palette, "paint": "stroke"}}]);
    for (name, film) in films(items) {
        let px = render(film);
        for (i, hex) in palette.iter().enumerate() {
            let n = u32::from_str_radix(&hex[1..], 16).expect("a hex colour");
            let want = [f64::from(n >> 16 & 255), f64::from(n >> 8 & 255), f64::from(n & 255)];
            // the middle of the stroke, where it is at its full width
            let p = bgra(&px, W, 240, 40 + 70 * i);
            let got = [f64::from(p[2]), f64::from(p[1]), f64::from(p[0])];
            assert!(got.iter().zip(want).all(|(g, w)| (g - w).abs() <= 1.0), "{name}, copy {i}: pixel {got:?}, expected {want:?}");
        }
    }
}
