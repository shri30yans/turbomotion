//! A box is strict: a mistake is refused with the key, the value and what is allowed, at plan time.

use std::path::Path;

use serde_json::{json, Value};
use turbomotion_comp::plan::Plan;

use crate::common::*;

fn plan_error(layer: Value) -> String {
    match Plan::from_value(film(400, 200, json!([layer])), &fonts_dir()) {
        Ok(_) => panic!("the plan should be refused"),
        Err(e) => format!("{e:#}"),
    }
}

fn with_box(b: Value) -> Value {
    text_layer(json!("Hi"), FONTS[0], json!({"box": b}))
}

fn assert_all(message: &str, parts: &[&str]) {
    for p in parts {
        assert!(message.contains(p), "{message:?} lacks {p:?}");
    }
}

#[test]
fn a_box_that_is_complete_is_planned() {
    let plan = Plan::from_value(film(400, 200, json!([with_box(json!({"fit": "text", "fill": "#fff", "padding": [8, 4], "radius": 6, "min_width": 100, "align": "center", "per_line": true}))])), Path::new(&fonts_dir()));
    assert!(plan.is_ok(), "{:#}", plan.err().unwrap());
}

#[test]
fn a_fit_other_than_text_names_the_one_choice() {
    assert_all(&plan_error(with_box(json!({"fit": "layer", "fill": "#fff"}))), &["fit", "unknown variant `layer`", "expected `text`"]);
}

#[test]
fn the_box_needs_its_fit_and_its_fill() {
    assert_all(&plan_error(with_box(json!({"fill": "#fff"}))), &["missing field `fit`"]);
    assert_all(&plan_error(with_box(json!({"fit": "text"}))), &["missing field `fill`"]);
}

#[test]
fn a_misspelt_key_names_the_nearest_valid_one() {
    assert_all(&plan_error(with_box(json!({"fit": "text", "fill": "#fff", "paddin": [4, 4]}))), &["paddin", "Did you mean \"padding\"?"]);
    assert_all(&plan_error(with_box(json!({"fit": "text", "fill": "#fff", "perline": true}))), &["perline", "Did you mean \"per_line\"?"]);
}

#[test]
fn negative_numbers_are_refused_with_the_number_and_the_unit() {
    let b = |key: &str, v: Value| {
        let mut b = json!({"fit": "text", "fill": "#fff"});
        b[key] = v;
        plan_error(with_box(b))
    };
    assert_all(&b("padding", json!([-4, 6])), &["box padding x is 0 or more pixels, not -4"]);
    assert_all(&b("padding", json!([4, -6])), &["box padding y is 0 or more pixels, not -6"]);
    assert_all(&b("padding", json!([4, {"from": 0, "to": -6, "dur": 1}])), &["box padding y is 0 or more pixels, not -6"]);
    assert_all(&b("radius", json!(-1)), &["box radius is 0 or more pixels, not -1"]);
    assert_all(&b("min_width", json!(-10)), &["box min_width is 0 or more pixels, not -10"]);
}

#[test]
fn padding_is_a_pair_and_per_line_is_a_boolean() {
    assert_all(&plan_error(with_box(json!({"fit": "text", "fill": "#fff", "padding": 12}))), &["padding", "array of length 2"]);
    assert_all(&plan_error(with_box(json!({"fit": "text", "fill": "#fff", "per_line": "yes"}))), &["per_line", "boolean"]);
    assert_all(&plan_error(with_box(json!({"fit": "text", "fill": "#fff", "align": "middle"}))), &["align", "unknown variant `middle`"]);
}

#[test]
fn a_box_on_a_layer_that_is_not_text_says_which_layers_take_it() {
    let solid = json!({"id": "s", "type": "solid", "color": "#fff", "box": {"fit": "text", "fill": "#fff"}});
    assert_all(&plan_error(solid), &["unknown key \"box\"", "`box` is a key of text layers"]);
}

#[test]
fn per_line_on_a_layer_that_is_not_text_is_an_unknown_key() {
    let solid = json!({"id": "s", "type": "solid", "color": "#fff", "per_line": true});
    assert_all(&plan_error(solid), &["unknown key \"per_line\"", "A solid layer takes"]);
}
