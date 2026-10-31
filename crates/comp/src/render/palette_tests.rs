//! Palette cycles measured on rendered frames: the pixel colour of each copy against the colour the cycle should give it,
//! written out here from the palette, in a film of encoded numbers and in linear light, in both effect spaces.

use serde_json::{json, Value};

use super::testkit::{bgra, render, render_at};

const W: usize = 320;
const H: usize = 60;
/// The palette of the tests: five colours that differ in every channel.
const PALETTE: [&str; 5] = ["#e63946", "#f1fa8c", "#2a9d8f", "#264653", "#9b5de5"];

fn levels(hex: &str) -> [f64; 3] {
    let n = u32::from_str_radix(&hex[1..], 16).expect("a hex colour");
    [f64::from(n >> 16 & 255), f64::from(n >> 8 & 255), f64::from(n & 255)]
}

/// sRGB to light and back, the IEC 61966-2-1 formulas.
fn to_light(level: f64) -> f64 {
    let v = level / 255.0;
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

fn to_level(light: f64) -> f64 {
    255.0 * if light <= 0.003_130_8 { light * 12.92 } else { 1.055 * light.powf(1.0 / 2.4) - 0.055 }
}

/// The films every test runs: encoded numbers or linear light, effects on the picture or on the layer.
fn films(layers: Value) -> Vec<(String, Value)> {
    let mut out = vec![];
    for light in [false, true] {
        for space in ["screen", "layer"] {
            let mut film = json!({"width": W, "height": H, "fps": 10, "dur": 1, "bg": "#000000", "effect_space": space, "layers": layers});
            if light {
                film["color"] = json!({"linear_light": true});
            }
            if space == "layer" {
                // a mask over the whole frame sends the layer through the layer-space path
                film["layers"][0]["masks"] = json!([{"path": {"rect": [0, 0, W, H]}}]);
            }
            out.push((format!("{} / {space}", if light { "linear light" } else { "encoded" }), film));
        }
    }
    out
}

/// The centre of copy `i` of a row of 40 px squares 50 px apart.
fn centre(i: usize) -> (usize, usize) {
    (30 + 50 * i, 30)
}

fn pixel(px: &[u8], (x, y): (usize, usize)) -> [f64; 3] {
    let p = bgra(px, W, x, y);
    [f64::from(p[2]), f64::from(p[1]), f64::from(p[0])]
}

fn assert_level(what: &str, got: [f64; 3], want: [f64; 3]) {
    assert!(got.iter().zip(want).all(|(g, w)| (g - w).abs() <= 1.0), "{what}: pixel {got:?}, expected {want:?}");
}

/// A row of `copies` squares through the shape repeater, tinted from `palette`.
fn shape_row(copies: usize, palette: &[&str], cycle: Value, extra: Value) -> Value {
    let mut repeat = json!({"copies": copies, "offset": [50, 0], "palette": palette, "cycle": cycle});
    repeat.as_object_mut().expect("an object").extend(extra.as_object().cloned().unwrap_or_default());
    json!([{"type": "shape", "items": [{"rect": [10, 10, 40, 40], "fill": "#ffffff", "repeat": repeat}]}])
}

/// The palette index written out from the definition: `floor(i + offset) mod n`, counted from the end when reversed.
fn step_index(i: usize, offset: f64, n: usize, reverse: bool) -> usize {
    let index = (i as f64 + offset).floor().rem_euclid(n as f64) as usize;
    if reverse { n - 1 - index } else { index }
}

#[test]
fn step_copy_i_shows_palette_i_plus_offset_mod_n_in_every_film() {
    // (copies, colours in the palette, offset, reverse): negative, fractional and reversed cycles among them
    let cases = [(5, 3, 0.0, false), (6, 4, 1.0, false), (6, 4, -1.0, false), (5, 5, 2.5, false), (4, 2, -0.5, true), (6, 5, 7.0, true), (6, 3, -7.25, false)];
    for (copies, n, offset, reverse) in cases {
        let palette = &PALETTE[..n];
        for (name, film) in films(shape_row(copies, palette, json!({"offset": offset, "reverse": reverse}), json!({}))) {
            let px = render(film);
            for i in 0..copies {
                assert_level(&format!("{name}, {copies} copies of {n} colours, offset {offset}, reverse {reverse}, copy {i}"), pixel(&px, centre(i)), levels(palette[step_index(i, offset, n, reverse)]));
            }
        }
    }
}

#[test]
fn ping_pong_shows_the_exact_index_sequence() {
    // four colours: 0 1 2 3 2 1 0 1 2 3 ..., the ends not doubled
    let want = [0, 1, 2, 3, 2, 1];
    for (name, film) in films(shape_row(6, &PALETTE[..4], json!({"mode": "ping_pong"}), json!({}))) {
        let px = render(film);
        for (i, index) in want.iter().enumerate() {
            assert_level(&format!("{name}, copy {i}"), pixel(&px, centre(i)), levels(PALETTE[*index]));
        }
    }
}

/// Copy `i` of a gradient cycle written out: `place = i + offset`, blended between the colours either side of
/// `place mod n`, in encoded numbers or in light.
fn gradient(i: usize, offset: f64, n: usize, light: bool) -> [f64; 3] {
    let u = (i as f64 + offset).rem_euclid(n as f64);
    let (from, to, by) = (u.floor() as usize % n, (u.floor() as usize + 1) % n, u - u.floor());
    let (a, b) = (levels(PALETTE[from]), levels(PALETTE[to]));
    std::array::from_fn(|c| if light { to_level(to_light(a[c]) + (to_light(b[c]) - to_light(a[c])) * by) } else { a[c] + (b[c] - a[c]) * by })
}

#[test]
fn a_gradient_is_the_closed_form_mix_in_the_working_space_of_each_film() {
    for offset in [0.0, 0.25, -0.5, 1.75] {
        for (name, film) in films(shape_row(6, &PALETTE[..3], json!({"mode": "gradient", "offset": offset}), json!({}))) {
            let px = render(film);
            for i in 0..6 {
                assert_level(&format!("{name}, offset {offset}, copy {i}"), pixel(&px, centre(i)), gradient(i, offset, 3, name.starts_with("linear")));
            }
        }
    }
}

#[test]
fn a_gradient_in_linear_light_matches_a_skia_gradient_drawn_in_the_same_film() {
    // black to white across 200 px: where the shape's gradient is at 0.4975 (pixel 99), a blend of light is level 187.5 / 0.4975's light
    let layers = json!([{"type": "shape", "items": [{"rect": [0, 0, 200, 20], "fill": {"gradient": {"stops": [[0, "#000000"], [1, "#ffffff"]], "from": [0, 0], "to": [200, 0]}}}]}]);
    let mut film = json!({"width": W, "height": H, "fps": 10, "dur": 1, "bg": "#000000", "color": {"linear_light": true}, "layers": layers});
    let px = render(film.clone());
    let by = 99.5 / 200.0;
    let skia = pixel(&px, (99, 10))[0];
    film["layers"] = shape_row(2, &["#000000", "#ffffff"], json!({"mode": "gradient", "offset": by}), json!({}));
    let ours = pixel(&render(film), centre(0))[0];
    assert!((skia - ours).abs() <= 1.0, "Skia's gradient shows {skia}, the palette cycle {ours}");
    assert!((ours - to_level(to_light(0.0) + (to_light(255.0) - to_light(0.0)) * by)).abs() <= 1.0);
}

#[test]
fn a_keyed_offset_changes_the_assignment_as_time_passes() {
    let film = shape_row(5, &PALETTE[..3], json!({"offset": [[0, 0], [1, 4, "none"]]}), json!({}));
    let layers = film;
    for (name, mut film) in films(layers) {
        film["dur"] = json!(2);
        // offset 4 * t: copy i shows floor(i + 4 t) mod 3
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let px = render_at(film.clone(), std::path::Path::new("."), t);
            for i in 0..5 {
                assert_level(&format!("{name}, t {t}, copy {i}"), pixel(&px, centre(i)), levels(PALETTE[step_index(i, 4.0 * t, 3, false)]));
            }
        }
    }
}

#[test]
fn the_palette_works_with_the_opacity_ramp_of_the_repeater() {
    // each copy is 0.8 of the one before: over black, copy i shows its colour at 0.8^i
    for (name, film) in films(shape_row(5, &PALETTE[..4], json!({"offset": 1}), json!({"opacity": 0.8}))) {
        let px = render(film);
        for i in 0..5 {
            let alpha = 0.8f64.powi(i as i32);
            let colour = levels(PALETTE[step_index(i, 1.0, 4, false)]);
            let want: [f64; 3] = std::array::from_fn(|c| if name.starts_with("linear") { to_level(to_light(colour[c]) * alpha) } else { colour[c] * alpha });
            assert_level(&format!("{name}, copy {i}"), pixel(&px, centre(i)), want);
        }
    }
}

#[test]
fn a_stroke_only_palette_leaves_the_fill_and_a_both_palette_takes_both() {
    let item = |paint: &str| json!([{"type": "shape", "items": [{"rect": [10, 10, 40, 40], "fill": "#ffffff", "stroke": {"color": "#000000", "width": 8},
        "repeat": {"copies": 3, "offset": [100, 0], "palette": PALETTE[..3], "paint": paint}}]}]);
    for (paint, fill_takes, stroke_takes) in [("stroke", false, true), ("fill", true, false), ("both", true, true)] {
        for (name, film) in films(item(paint)) {
            let px = render(film);
            for (i, swatch) in PALETTE[..3].iter().copied().enumerate() {
                let colour = levels(swatch);
                // the middle of the square is fill; the middle of the left edge (x = 10, y = 30) is stroke
                assert_level(&format!("{name}, paint {paint}, fill of copy {i}"), pixel(&px, (30 + 100 * i, 30)), if fill_takes { colour } else { [255.0; 3] });
                assert_level(&format!("{name}, paint {paint}, stroke of copy {i}"), pixel(&px, (10 + 100 * i, 30)), if stroke_takes { colour } else { [0.0; 3] });
            }
        }
    }
}

/// A row of five layers made by the layer repeater, 50 px apart.
fn layer_row(layer: Value, palette: &[&str], cycle: Value, extra: Value) -> Value {
    let mut repeat = json!({"count": 5, "offset": [50, 0], "palette": palette, "cycle": cycle});
    repeat.as_object_mut().expect("an object").extend(extra.as_object().cloned().unwrap_or_default());
    let mut layer = layer;
    layer["repeat"] = repeat;
    json!([layer])
}

#[test]
fn a_layer_repeater_colours_solids_shapes_and_text_by_their_place_in_the_row() {
    let solid = json!({"type": "solid", "id": "bar", "color": "#ffffff", "size": [40, 40], "pos": [30, 30]});
    let shape = json!({"type": "shape", "id": "bar", "items": [{"rect": [10, 10, 40, 40], "fill": "#ffffff"}]});
    for (what, layer) in [("solid", solid), ("shape", shape)] {
        for (name, film) in films(layer_row(layer, &PALETTE[..4], json!({"offset": -1}), json!({}))) {
            let px = render(film);
            for i in 0..5 {
                assert_level(&format!("{what}, {name}, layer {i}"), pixel(&px, centre(i)), levels(PALETTE[step_index(i, -1.0, 4, false)]));
            }
        }
    }
}

#[test]
fn a_layer_repeater_colours_text_and_its_opacity_ramp_still_applies() {
    let text = json!({"type": "text", "id": "t", "text": "H", "size": 36, "weight": 900, "color": "#ffffff", "pos": [30, 30]});
    for (name, film) in films(layer_row(text, &PALETTE[..3], json!({"mode": "step"}), json!({"opacity": 0.5}))) {
        let px = render(film);
        for i in 0..5 {
            // the brightest pixel of each channel inside the copy's own 50 px column is the colour at the copy's opacity
            let alpha = 0.5f64.powi(i as i32);
            let column = (50 * i..50 * i + 50).flat_map(|x| (0..H).map(move |y| (x, y)));
            let best: [f64; 3] = column.map(|p| pixel(&px, p)).fold([0.0; 3], |m, p| std::array::from_fn(|c| m[c].max(p[c])));
            let colour = levels(PALETTE[i % 3]);
            let want: [f64; 3] = std::array::from_fn(|c| if name.starts_with("linear") { to_level(to_light(colour[c]) * alpha) } else { colour[c] * alpha });
            // glyph coverage is not 1 anywhere on a thin stem of a small size, so allow the brightest pixel to be a little under
            assert!(best.iter().zip(want).all(|(b, w)| *b <= w + 1.0 && *b >= w * 0.9 - 1.0), "{name}, text {i}: brightest {best:?}, expected about {want:?}");
        }
    }
}
