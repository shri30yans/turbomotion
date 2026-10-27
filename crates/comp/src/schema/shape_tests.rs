//! The rules between the keys of a shape item that carry a taper or a palette: what is accepted and what each mistake says.

use serde_json::{json, Value};

use super::*;

fn item(extra: Value) -> Value {
    let mut v = json!({"rect": [0, 0, 100, 40], "stroke": {"color": "#fff", "width": 10}});
    v.as_object_mut().expect("an object").extend(extra.as_object().cloned().unwrap_or_default());
    v
}

fn refused(v: Value) -> String {
    match ShapeItem::parse(&v) {
        Ok(_) => panic!("accepted {v}"),
        Err(e) => format!("{e:#}"),
    }
}

#[test]
fn a_taper_and_a_palette_are_read_from_the_item() {
    let ok = ShapeItem::parse(&item(json!({"fill": "#fff", "taper": {"start": {"length": 30, "width": 0}}, "repeat": {"copies": 3, "palette": ["#f00", "#00f"], "paint": "both", "cycle": {"mode": "gradient"}}}))).expect("an item");
    assert!(ok.taper.is_some() && ok.repeat.expect("a repeat").palette.is_some());
}

#[test]
fn a_taper_needs_a_stroke_to_thin() {
    let m = refused(json!({"rect": [0, 0, 10, 10], "fill": "#fff", "taper": {"end": {"length": 5}}}));
    assert!(m.contains("taper thins a stroke") && m.contains("add a `stroke`"), "{m}");
}

#[test]
fn a_taper_with_a_bad_number_a_misspelt_key_or_a_dash_says_what_to_change() {
    assert!(refused(item(json!({"taper": {"start": {"length": -4}}}))).contains("taper start length is 0 or more, not -4"));
    assert!(refused(item(json!({"taper": {"end": {"length": 4, "width": -1}}}))).contains("taper end width"));
    assert!(refused(item(json!({"taper": {"strat": {"length": 4}}}))).contains("Did you mean \"start\"?"));
    assert!(refused(item(json!({"taper": {}}))).contains("`start` or an `end`"));
    assert!(refused(item(json!({"taper": {"start": {"length": 4}}, "stroke": {"color": "#fff", "width": 4, "dash": [4, 4]}}))).contains("cannot also be dashed"));
    assert!(refused(item(json!({"taper": {"start": {"length": 4}}, "stroke": {"color": "#fff", "width": 4, "cap": "pointy"}}))).contains("a stroke cap is"));
}

#[test]
fn a_palette_on_a_repeat_must_be_colours_and_must_have_something_to_colour() {
    let repeat = |extra: Value| {
        let mut r = json!({"copies": 3});
        r.as_object_mut().expect("an object").extend(extra.as_object().cloned().unwrap_or_default());
        refused(item(json!({"repeat": r})))
    };
    assert!(repeat(json!({"palette": []})).contains("palette has no colours"));
    assert!(repeat(json!({"palette": ["#fff", 7]})).contains("palette[1]"));
    assert!(repeat(json!({"palette": ["#fff"], "cycle": {"mode": "wave"}})).contains("cycle:"));
    assert!(repeat(json!({"palete": ["#fff"]})).contains("Did you mean \"palette\"?"));
    assert!(repeat(json!({"cycle": {"offset": 1}})).contains("add a `palette`"));
    // a fill that is not one colour cannot take one
    let gradient = refused(json!({"rect": [0, 0, 10, 10], "fill": {"gradient": {"stops": [[0, "#000"], [1, "#fff"]]}}, "repeat": {"copies": 2, "palette": ["#f00"]}}));
    assert!(gradient.contains("flat-colour fill"), "{gradient}");
    let no_stroke = refused(json!({"rect": [0, 0, 10, 10], "fill": "#fff", "repeat": {"copies": 2, "palette": ["#f00"], "paint": "stroke"}}));
    assert!(no_stroke.contains("has no stroke"), "{no_stroke}");
}
