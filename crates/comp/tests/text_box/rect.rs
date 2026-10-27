//! `source_rect_at_time(t, include_extents)`: the typed measurement expressions will call, and what `comp inspect` shows.

use serde_json::json;
use turbomotion_comp::{
    eval::{text::bounds::source_rect_at_time, Evaluator},
    plan::{Plan, ROOT},
};

use crate::common::*;

fn ask(layers: serde_json::Value, index: usize, t: f64, extents: bool) -> anyhow::Result<Option<turbomotion_comp::text::bounds::SourceRect>> {
    let plan = Plan::from_value(film(1400, 500, layers), &fonts_dir()).expect("the plan");
    let comp = &plan.comps[ROOT];
    source_rect_at_time(&Evaluator::new(&plan), comp, &comp.layers[index], t, extents)
}

#[test]
fn the_rectangle_is_the_text_bounds_inspect_shows_and_the_measured_characters() {
    let layers = json!([text_layer(json!("AVATAR Wave"), FONTS[0], json!({}))]);
    let r = ask(layers.clone(), 0, 0.0, false).unwrap().expect("text is showing");
    let c = content_at(film(1400, 500, layers), 0.0);
    let shown = &c["bounds"]["block"];
    assert_eq!([r.left, r.top, r.width, r.height].map(|v| (v * 1e3).round()), ["left", "top", "width", "height"].map(|k| (shown[k].as_f64().unwrap() * 1e3).round()));
    let m = measured("AVATAR Wave", FONTS[0]);
    assert_eq!(serde_json::to_value(m.bounds.expect("bounds")).unwrap(), c["bounds"], "`measure()` and an evaluated layer with every character showing are one measurement");
    let extents = line_extents(&m.chars)[0];
    assert!((r.left - extents[0]).abs() <= TOLERANCE && (r.width - (extents[1] - extents[0])).abs() <= TOLERANCE, "{r:?} for characters from {} to {}", extents[0], extents[1]);
}

#[test]
fn include_extents_grows_the_rectangle_by_half_the_stroke_on_every_side() {
    let layers = json!([text_layer(json!("Wave"), FONTS[0], json!({"stroke": {"color": "#ff0000", "width": 10}}))]);
    let (plain, grown) = (ask(layers.clone(), 0, 0.0, false).unwrap().unwrap(), ask(layers, 0, 0.0, true).unwrap().unwrap());
    assert!((plain.left - grown.left - 5.0).abs() < 1e-9 && (plain.top - grown.top - 5.0).abs() < 1e-9, "{plain:?} {grown:?}");
    assert!((grown.width - plain.width - 10.0).abs() < 1e-9 && (grown.height - plain.height - 10.0).abs() < 1e-9, "{plain:?} {grown:?}");
}

#[test]
fn nothing_showing_is_none_and_a_layer_that_is_not_text_is_an_error_naming_its_type() {
    let layers = json!([text_layer(json!("Wave"), FONTS[0], json!({"preset": {"name": "typewriter", "at": 1.0}})), {"id": "s", "type": "solid", "color": "#fff"}]);
    assert_eq!(ask(layers.clone(), 0, 0.2, false).unwrap(), None);
    assert!(ask(layers.clone(), 0, 1.5, false).unwrap().is_some());
    let message = format!("{:#}", ask(layers, 1, 0.0, false).unwrap_err());
    assert!(message.contains("a source rectangle is measured for text layers, and this is a solid layer"), "{message}");
}
