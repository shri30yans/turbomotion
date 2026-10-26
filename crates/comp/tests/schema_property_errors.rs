//! A property that cannot be read says which layer, which property, what forms it takes and what it found.

#[allow(dead_code)] // each test crate uses a different part of the helpers
mod schema_common;

use schema_common::{error_of, load};
use serde_json::json;

fn solid(extra: serde_json::Value) -> serde_json::Value {
    let mut layer = json!({"type": "solid", "color": "#fff", "id": "bg"});
    layer.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    layer
}

#[track_caller]
fn assert_names(err: &str, parts: &[&str]) {
    for p in parts {
        assert!(err.contains(p), "the error should mention {p:?}:\n{err}");
    }
}

#[test]
fn a_list_of_numbers_given_to_a_scalar_is_named_as_a_vector() {
    let err = error_of(solid(json!({"opacity": [0.78, 1]})));
    assert_names(&err, &["layer 0", "\"bg\"", "opacity", "scalar", "[0.78,1]", "vector", "expression"]);
}

#[test]
fn a_vector_tween_end_given_to_a_scalar_names_the_field() {
    let err = error_of(solid(json!({"opacity": {"from": [1], "to": 0}})));
    assert_names(&err, &["opacity", "\"from\" must be one number", "[1]", "a list is a vector"]);
}

#[test]
fn a_vector_tween_with_one_scalar_end_says_both_must_be_lists() {
    let err = error_of(solid(json!({"scale": {"from": [0.78, 1], "to": 1.25}})));
    assert_names(&err, &["scale", "\"from\" and \"to\" both as lists", "[0.78,1]", "1.25"]);
}

#[test]
fn a_scalar_given_to_a_vector_lists_the_vector_forms() {
    let err = error_of(solid(json!({"pos": 500})));
    assert_names(&err, &["pos", "500", "[x, y]", "\"keys\""]);
}

#[test]
fn a_key_with_a_text_value_names_the_key() {
    let err = error_of(solid(json!({"opacity": [[0, "x"]]})));
    assert_names(&err, &["opacity", "a key's value must be a number", "\"x\""]);
}

#[test]
fn the_right_forms_still_load() {
    for layer in [
        solid(json!({"opacity": 0.5})),
        solid(json!({"opacity": "wiggle(2, 0.1)"})),
        solid(json!({"opacity": [[0, 0], [1, 1]]})),
        solid(json!({"opacity": {"from": 0, "to": 1, "dur": 1}})),
        solid(json!({"scale": [0.78, 1]})),
        solid(json!({"scale": [{"from": 0.8, "to": 1.2}, "1 + time / 10"]})),
        solid(json!({"pos": {"from": [0, 0], "to": [10, 20], "dur": 1}})),
    ] {
        load(layer.clone()).unwrap_or_else(|e| panic!("{layer}: {e:#}"));
    }
}
