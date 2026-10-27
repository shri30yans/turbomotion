//! The box as drawn: scanned out of rendered frames and held against what `comp inspect` measures. The fill is white on
//! black and so is the text, so the red channel of a pixel is how much of it the box covers, whatever the glyphs do.

use serde_json::{json, Value};

use crate::common::*;

const W: usize = 1400;
const H: usize = 500;
const POS: [f64; 2] = [40.0, 200.0];
const PAD: [f64; 2] = [24.0, 12.0];

fn boxed(text: &str, font: &str, extra: Value, b: Value) -> Value {
    let mut layer = text_layer(json!(text), font, extra);
    layer["box"] = json!({"fit": "text", "fill": "#ffffff"});
    layer["box"].as_object_mut().expect("an object").extend(b.as_object().cloned().unwrap_or_default());
    film(W as u32, H as u32, json!([layer]))
}

/// The layer's box, and where the layer puts the top-left of its text block on screen.
fn measured_box(film: &Value, t: f64) -> ([f64; 4], [f64; 2]) {
    let c = content_at(film.clone(), t);
    let anchor = [c["anchor"][0].as_f64().expect("an anchor"), c["anchor"][1].as_f64().expect("an anchor")];
    (rects(&c)[0], [POS[0] - anchor[0], POS[1] - anchor[1]])
}

#[test]
fn the_drawn_box_is_the_measured_box_within_half_a_pixel_on_every_side() {
    for font in FONTS {
        for text in ["AVATAR", "To Wave", "Hamburgefonstiv", "Wave\nAVATAR to"] {
            let film = boxed(text, font, json!({}), json!({"padding": PAD}));
            let (r, origin) = measured_box(&film, 0.0);
            let px = render(film, 0.0);
            // along a row and a column through the padding, where there is no ink
            let (row, col) = ((origin[1] + r[1] + PAD[1] / 2.0) as usize, (origin[0] + r[0] + PAD[0] / 2.0) as usize);
            let [left, right] = run(W, |x| cover(&px, W, x, row));
            let [top, bottom] = run(H, |y| cover(&px, W, col, y));
            let drawn = [left, top, right, bottom];
            let expected = [origin[0] + r[0], origin[1] + r[1], origin[0] + r[2], origin[1] + r[3]];
            for (d, e) in drawn.iter().zip(expected) {
                assert!((d - e).abs() <= TOLERANCE, "{font} {text:?}: drawn {drawn:?}, measured {expected:?}");
            }
        }
    }
}

#[test]
fn a_radius_takes_four_minus_pi_radius_squared_off_the_area_and_is_held_to_a_pill() {
    let at = |radius: f64| area(&render(boxed("Wave", FONTS[0], json!({}), json!({"padding": PAD, "radius": radius})), 0.0), W, H);
    let (square, round, pill, huge) = (at(0.0), at(24.0), at(1.0e6), at(46.0));
    let corners = (4.0 - std::f64::consts::PI) * 24.0 * 24.0;
    assert!((square - round - corners).abs() <= 6.0, "radius 24 removes {} px of area, 4 corners of a circle's gap are {corners}", square - round);
    let film = boxed("Wave", FONTS[0], json!({}), json!({"padding": PAD}));
    let (r, _) = measured_box(&film, 0.0);
    let (w, h) = (r[2] - r[0], r[3] - r[1]);
    let radius = h / 2.0;
    let capsule = std::f64::consts::PI * radius * radius + (w - 2.0 * radius) * h;
    assert!((pill - capsule).abs() <= 8.0, "a radius far over half the height is a pill: {pill} px against {capsule}");
    assert!(huge <= square && huge >= pill - 8.0, "{huge}");
}

#[test]
fn the_box_is_the_same_frame_twice() {
    let film = boxed("AVATAR", FONTS[1], json!({"preset": {"name": "typewriter", "each": 0.1}}), json!({"padding": PAD, "radius": 12}));
    assert!(render(film.clone(), 0.35) == render(film, 0.35));
}
