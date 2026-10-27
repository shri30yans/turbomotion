//! `padding`, `min_width`, `align` and `radius`: what each does to the rectangles, in numbers.

use serde_json::{json, Value};

use crate::common::*;

const FONT: &str = FONTS[0];

fn at(text: &str, extra: Value, b: Value) -> Vec<[f64; 4]> {
    let mut b = b;
    b.as_object_mut().expect("an object").extend(extra.as_object().cloned().unwrap_or_default());
    rects(&content_at(film(1800, 600, json!([text_layer(json!(text), FONT, json!({"box": b}))])), 0.0))
}

fn plain(padding: Value) -> Value {
    red_box(padding)
}

#[test]
fn min_width_widens_a_short_box_to_exactly_that_and_leaves_a_longer_one_alone() {
    let natural = at("Hi", json!({}), plain(json!([20, 10])))[0];
    let wide = at("Hi", json!({"min_width": 400}), plain(json!([20, 10])))[0];
    assert!((width(wide) - 400.0).abs() <= TOLERANCE, "{} px wide", width(wide));
    assert_eq!((wide[1], wide[3]), (natural[1], natural[3]), "the height does not change");
    let kept = at("Hi", json!({"min_width": 10}), plain(json!([20, 10])))[0];
    assert_eq!(kept, natural, "a minimum below the natural width does nothing");
}

#[test]
fn align_names_the_side_of_a_widened_box_that_keeps_its_place() {
    let natural = at("Hi", json!({}), plain(json!([20, 10])))[0];
    let grown = |align: &str| at("Hi", json!({"min_width": 400, "align": align}), plain(json!([20, 10])))[0];
    let (left, right, centre) = (grown("left"), grown("right"), grown("center"));
    let extra = 400.0 - width(natural);
    assert!((left[0] - natural[0]).abs() <= TOLERANCE && (left[2] - (natural[2] + extra)).abs() <= TOLERANCE, "left keeps the left edge: {left:?} from {natural:?}");
    assert!((right[2] - natural[2]).abs() <= TOLERANCE && (right[0] - (natural[0] - extra)).abs() <= TOLERANCE, "right keeps the right edge: {right:?}");
    assert!((centre[0] - (natural[0] - extra / 2.0)).abs() <= TOLERANCE && (centre[2] - (natural[2] + extra / 2.0)).abs() <= TOLERANCE, "centre grows both ways: {centre:?}");
}

#[test]
fn without_an_align_a_box_widens_toward_the_side_the_text_is_aligned_to() {
    let widen = |text_align: &str| {
        let layer = text_layer(json!("Hi"), FONT, json!({"align": text_align, "box": {"fit": "text", "padding": [20, 10], "fill": "#fff", "min_width": 400}}));
        rects(&content_at(film(1800, 600, json!([layer])), 0.0))[0]
    };
    let natural = at("Hi", json!({}), plain(json!([20, 10])))[0];
    let extra = 400.0 - width(natural);
    let centred = widen("center");
    assert!((width(centred) - 400.0).abs() <= TOLERANCE);
    let (grow_left, grow_right) = (natural[0] - centred[0], centred[2] - natural[2]);
    assert!(grow_left > 0.0 && (grow_left - grow_right).abs() <= TOLERANCE + 1.0, "a centred text widens its box on both sides: left {grow_left}, right {grow_right}, of {extra}");
    let right = widen("right");
    assert!(right[2] - natural[2] <= TOLERANCE + 1.0 && natural[0] - right[0] > extra - 1.0, "a right-aligned text keeps the right edge: {right:?}");
}

#[test]
fn a_keyed_padding_gives_the_expected_box_at_every_sampled_time() {
    let film_with = |pad: Value| film(1800, 600, json!([text_layer(json!("Wave"), FONT, json!({"box": plain(pad)}))]));
    let fixed = rects(&content_at(film_with(json!([0, 0])), 0.0))[0];
    let keyed = film_with(json!([{"from": 4, "to": 44, "at": 0, "dur": 2, "ease": "none"}, {"from": 0, "to": 20, "at": 1, "dur": 1, "ease": "none"}]));
    for (t, x, y) in [(0.0, 4.0, 0.0), (0.5, 14.0, 0.0), (1.0, 24.0, 0.0), (1.5, 34.0, 10.0), (2.0, 44.0, 20.0), (3.0, 44.0, 20.0)] {
        let r = rects(&content_at(keyed.clone(), t))[0];
        let expected = [fixed[0] - x, fixed[1] - y, fixed[2] + x, fixed[3] + y];
        assert!(r.iter().zip(expected).all(|(a, b)| (a - b).abs() <= TOLERANCE), "t={t}: padding ({x}, {y}) gives {expected:?}, drawn {r:?}");
    }
}

#[test]
fn a_padding_that_dips_below_zero_at_run_time_is_an_error_naming_the_time() {
    let overshoot = film(800, 300, json!([text_layer(json!("Hi"), FONT, json!({"box": plain(json!([{"from": 0, "to": 20, "at": 0, "dur": 1, "ease": "back.in(3)"}, 0]))}))]));
    let message = inspect_error(overshoot, 0.3);
    assert!(message.contains("box padding x is -") && message.contains("0.300 s into the layer"), "{message}");
}

#[test]
fn a_text_with_no_characters_showing_has_no_box() {
    let c = content_at(film(800, 300, json!([text_layer(json!("   "), FONT, json!({"box": plain(json!([20, 10]))}))])), 0.0);
    assert!(rects(&c).is_empty(), "{c}");
    assert!(c["bounds"].is_null());
}
