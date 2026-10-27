//! `comp check` reads a text layer's own box as what its type sits on.

use serde_json::{json, Value};
use turbomotion_comp::{api, check::Severity};

use crate::common::*;

fn findings(color: &str, fill: Option<&str>) -> Vec<String> {
    let mut layer = text_layer(json!("Hi there"), FONTS[0], json!({"color": color, "pos": [200, 300], "size": 120}));
    if let Some(fill) = fill {
        // typed from half a second in, so the first frame has no character showing and the box still counts
        layer["box"] = json!({"fit": "text", "padding": [24, 12], "fill": fill});
        layer["preset"] = json!({"name": "typewriter", "at": 0.5});
    }
    let film: Value = film(1920, 1080, json!([layer]));
    api::validate(film, &fonts_dir()).expect("validates").into_iter().filter(|f| f.severity == Severity::Error).map(|f| f.rule.to_string()).collect()
}

#[test]
fn dark_type_on_a_light_box_over_a_dark_ground_is_fine_and_without_the_box_it_is_not() {
    assert_eq!(findings("#10141f", Some("#e2c48a")), Vec::<String>::new());
    assert_eq!(findings("#10141f", None), ["low-contrast"]);
}

#[test]
fn type_that_matches_its_own_box_is_still_low_contrast() {
    assert_eq!(findings("#e2c48a", Some("#e2c48a")), ["low-contrast"]);
}
