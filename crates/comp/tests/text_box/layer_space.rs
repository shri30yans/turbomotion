//! The box is part of the layer: it takes the layer's scale and rotation exactly, and in layer space the buffer the
//! effects run on grows to hold it.

use serde_json::{json, Value};

use crate::common::*;

const W: usize = 1400;
const H: usize = 900;
const POS: [f64; 2] = [300.0, 300.0];
const PAD: [f64; 2] = [24.0, 12.0];

fn film(space: &str, scale: f64, rotation: f64) -> Value {
    let mut layer = text_layer(json!("Wave AV"), FONTS[1], json!({"pos": POS, "scale": scale, "rotation": rotation, "fx": [{"effect": "blur", "radius": 3}]}));
    layer["box"] = json!({"fit": "text", "padding": PAD, "fill": "#ffffff", "radius": 20});
    let mut film = crate::common::film(W as u32, H as u32, json!([layer]));
    film["effect_space"] = json!(space);
    film
}

/// The box in layer pixels from the block's top-left, and the layer's anchor in the same pixels.
fn box_and_anchor(film: &Value) -> ([f64; 4], [f64; 2]) {
    let c = content_at(film.clone(), 0.0);
    (rects(&c)[0], [c["anchor"][0].as_f64().expect("an anchor"), c["anchor"][1].as_f64().expect("an anchor")])
}

#[test]
fn the_buffer_of_a_layer_space_layer_holds_the_box_not_only_the_text() {
    let film = film("layer", 1.0, 0.0);
    let (b, _) = box_and_anchor(&film);
    let local = layer_at(film, 0.0)["local"]["bounds_local"].clone();
    let got: Vec<f64> = local.as_array().expect("bounds_local").iter().map(|v| v.as_f64().expect("a number")).collect();
    assert!(got.iter().zip(b).all(|(g, e)| (g - e).abs() <= 1.0), "the layer's pixels are {got:?} and the box {b:?}");
    assert!(got[0] < 0.0 && got[1] < 0.0, "the padding reaches above and to the left of the text block");
}

#[test]
fn at_scale_two_the_drawn_box_is_exactly_twice_the_measured_one_in_both_effect_spaces() {
    for space in ["layer", "screen"] {
        let film = film(space, 2.0, 0.0);
        let (b, anchor) = box_and_anchor(&film);
        let px = render(film, 0.0);
        let (row, col) = (POS[1] as usize, (POS[0] + 2.0 * ((b[0] + b[2]) / 2.0 - anchor[0])) as usize);
        let [left, right] = half_crossings(W, |x| cover(&px, W, x, row));
        let [top, bottom] = half_crossings(H, |y| cover(&px, W, col, y));
        let expected = [POS[0] + 2.0 * (b[0] - anchor[0]), POS[1] + 2.0 * (b[1] - anchor[1]), POS[0] + 2.0 * (b[2] - anchor[0]), POS[1] + 2.0 * (b[3] - anchor[1])];
        for (d, e) in [left, top, right, bottom].iter().zip(expected) {
            assert!((d - e).abs() <= TOLERANCE, "{space}: drawn {:?}, expected {expected:?}", [left, top, right, bottom]);
        }
        assert!(((right - left) - 2.0 * (b[2] - b[0])).abs() <= TOLERANCE, "{space}: {} px wide for a {} px box at scale 2", right - left, b[2] - b[0]);
    }
}

#[test]
fn turned_a_quarter_the_box_turns_about_the_anchor_and_keeps_its_size() {
    for space in ["layer", "screen"] {
        let film = film(space, 2.0, 90.0);
        let (b, a) = box_and_anchor(&film);
        let px = render(film, 0.0);
        // clockwise 90 degrees takes (dx, dy) to (-dy, dx)
        let expected = [POS[0] - 2.0 * (b[3] - a[1]), POS[1] + 2.0 * (b[0] - a[0]), POS[0] - 2.0 * (b[1] - a[1]), POS[1] + 2.0 * (b[2] - a[0])];
        let (row, col) = (((expected[1] + expected[3]) / 2.0) as usize, ((expected[0] + expected[2]) / 2.0) as usize);
        let [left, right] = half_crossings(W, |x| cover(&px, W, x, row));
        let [top, bottom] = half_crossings(H, |y| cover(&px, W, col, y));
        for (d, e) in [left, top, right, bottom].iter().zip(expected) {
            assert!((d - e).abs() <= TOLERANCE, "{space}: drawn {:?}, expected {expected:?}", [left, top, right, bottom]);
        }
    }
}

#[test]
fn turned_thirty_degrees_at_scale_two_the_box_keeps_four_times_its_area_and_its_centre() {
    for space in ["layer", "screen"] {
        let film = film(space, 2.0, 30.0);
        let (b, a) = box_and_anchor(&film);
        let px = render(film, 0.0);
        let r = 20.0_f64.min((b[3] - b[1]) / 2.0);
        let area_in_layer = (b[2] - b[0]) * (b[3] - b[1]) - (4.0 - std::f64::consts::PI) * r * r;
        let got = area(&px, W, H);
        assert!((got - 4.0 * area_in_layer).abs() <= 8.0, "{space}: covers {got} px, a {area_in_layer} px box at scale 2 covers {}", 4.0 * area_in_layer);
        let (cx, cy) = ((b[0] + b[2]) / 2.0 - a[0], (b[1] + b[3]) / 2.0 - a[1]);
        let turn = 30.0_f64.to_radians();
        let centre = [POS[0] + 2.0 * (cx * turn.cos() - cy * turn.sin()), POS[1] + 2.0 * (cx * turn.sin() + cy * turn.cos())];
        let found = centroid(&px, W, H);
        assert!((found[0] - centre[0]).abs() <= TOLERANCE && (found[1] - centre[1]).abs() <= TOLERANCE, "{space}: centre {found:?}, expected {centre:?}");
    }
}
