//! Every shape-item property the SDK can tween (`sdk/item-tweens.ts`) must also be a static field of the item, and mean the same.

#[allow(dead_code)] // each test crate uses a different part of the helpers
mod schema_common;

use std::path::Path;

use schema_common::{error_of, load, shape_items_at};
use serde_json::{json, Value};

/// `(name, path)` of each entry of the SDK's `ITEM` table, read from the source so this test fails when the SDK gains a property.
fn sdk_item_properties() -> Vec<(String, Vec<String>)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../script/sdk/item-tweens.ts");
    let text = std::fs::read_to_string(&src).unwrap_or_else(|e| panic!("{}: {e}", src.display()));
    let found: Vec<(String, Vec<String>)> = text
        .lines()
        .filter_map(|l| {
            let (name, rest) = l.trim().split_once(": { path: [")?;
            let path = rest.split_once(']')?.0.split(',').map(|s| s.trim().trim_matches('"').to_string()).collect();
            Some((name.to_string(), path))
        })
        .collect();
    assert!(found.len() >= 10, "found only {} item properties in item-tweens.ts: the table's layout changed", found.len());
    found
}

/// A value of the right kind for the property at the end of `path`.
fn sample(path: &[String]) -> Value {
    match path.last().map(String::as_str) {
        Some("color") => json!("#33aaff"),
        Some("offset") if path.len() == 1 => json!([4, 6]),
        _ => json!(0.5),
    }
}

/// The item written with `name` flat, and the same item with the value nested at `path`.
fn both_spellings(name: &str, path: &[String]) -> (Value, Value) {
    let value = sample(path);
    // a stroke needs its other required fields; leave out the one under test
    let stroke_rest: Vec<(&str, Value)> = [("color", json!("#ffffff")), ("width", json!(2))].into_iter().filter(|(k, _)| path.last().map(String::as_str) != Some(k)).collect();
    let mut flat = json!({"rect": [0, 0, 10, 10], name: value.clone()});
    let mut nested = json!({"rect": [0, 0, 10, 10]});
    match path {
        [one] => nested[one] = value,
        [group, field] => nested[group] = json!({field: value}),
        _ => panic!("{name}: unexpected path {path:?}"),
    }
    if path[0] == "stroke" {
        for (k, v) in stroke_rest {
            flat["stroke"][k] = v.clone();
            nested["stroke"][k] = v;
        }
    }
    (flat, nested)
}

#[test]
fn every_property_the_sdk_tweens_is_a_static_item_field() {
    for (name, path) in sdk_item_properties() {
        let (flat, nested) = both_spellings(&name, &path);
        let shape = |item: Value| json!({"type": "shape", "items": [item]});
        let flat_plan = load(shape(flat.clone())).unwrap_or_else(|e| panic!("{name} is tweenable but not a static field: {e:#}\n{flat}"));
        let nested_plan = load(shape(nested)).unwrap_or_else(|e| panic!("{name}: the nested spelling fails: {e:#}"));
        assert_eq!(shape_items_at(&flat_plan, 1.0), shape_items_at(&nested_plan, 1.0), "{name}: flat and nested spellings evaluate differently");
    }
}

#[test]
fn a_flat_field_can_be_an_expression_or_a_tween() {
    let plan = load(json!({"type": "shape", "items": [{"rect": [0, 0, 10, 10], "fill": "#fff", "fill_opacity": {"from": 0, "to": 1, "dur": 2, "ease": "none"}, "trim_end": "time / 4"}]})).unwrap();
    let at = |t: f64| shape_items_at(&plan, t);
    assert!((at(1.0)[0]["fill"]["opacity"].as_f64().unwrap() - 0.5).abs() < 1e-6);
    assert!((at(2.0)[0]["trim"][1].as_f64().unwrap() - 0.5).abs() < 1e-6);
}

#[test]
fn a_bare_fill_colour_combines_with_a_flat_opacity() {
    let plan = load(json!({"type": "shape", "items": [{"rect": [0, 0, 10, 10], "fill": "#ff0000", "fill_opacity": 0.25}]})).unwrap();
    assert!((shape_items_at(&plan, 0.0)[0]["fill"]["color"][0].as_f64().unwrap() - 255.0).abs() < 1e-6);
}

#[test]
fn setting_a_property_both_ways_is_an_error_that_names_both() {
    let err = error_of(json!({"type": "shape", "items": [{"rect": [0, 0, 10, 10], "fill": {"color": "#fff", "opacity": 1}, "fill_opacity": 0.5}]}));
    assert!(err.contains("fill_opacity") && err.contains("fill.opacity"), "{err}");
}

#[test]
fn an_unknown_field_lists_the_flat_spellings() {
    let err = error_of(json!({"type": "shape", "items": [{"rect": [0, 0, 10, 10], "fill_alpha": 1}]}));
    assert!(err.contains("fill_alpha") && err.contains("fill_opacity") && err.contains("trim_end"), "{err}");
}
