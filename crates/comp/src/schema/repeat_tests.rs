//! What the repeat pre-pass makes of a layer: the wrappers, the copies and the palette swatches.

use super::*;

fn run(layer: Value) -> Result<Vec<Value>> {
    let mut v = json!({"layers": [layer]});
    expand(&mut v)?;
    Ok(v["layers"].as_array().unwrap().clone())
}

fn card() -> Value {
    json!({"type": "solid", "id": "card", "color": "#fff", "pos": [100, 100]})
}

fn with(spec: Value) -> Value {
    let mut l = card();
    l["repeat"] = spec;
    l
}

#[test]
fn copies_sit_under_nulls_that_step_away_from_the_original() {
    let out = run(with(json!({"count": 3, "offset": [40, 10], "rotation": 5, "scale": 0.5, "origin": [960, 540]}))).unwrap();
    let ids: Vec<&str> = out.iter().map(|l| l["id"].as_str().unwrap()).collect();
    // later copies first (they sit behind), the original last and on top
    assert_eq!(ids, ["card~r2", "card~c2", "card~r1", "card~c1", "card"]);
    let w = &out[2];
    assert_eq!(w["type"], "null");
    assert_eq!(w["pos"], json!([1000.0, 550.0]));
    assert_eq!(w["anchor"], json!([960.0, 540.0]));
    assert_eq!(w["rotation"], json!(5.0));
    assert_eq!(w["scale"], json!(0.5));
    assert_eq!(out[3]["parent"], "card~r1");
    assert_eq!(out[3]["pos"], json!([100, 100]));
    assert!(out[4].get("repeat").is_none() && out[4]["id"] == "card");
}

#[test]
fn order_above_puts_the_copies_over_the_original_and_opacity_falls_off() {
    let out = run(with(json!({"count": 3, "offset": [10, 0], "order": "above", "opacity": 0.5}))).unwrap();
    assert_eq!(out[0]["id"], "card");
    assert_eq!(out[2]["opacity"], json!(0.5));
    assert_eq!(out[4]["opacity"], json!(0.25));
}

#[test]
fn a_step_may_be_an_expression_and_a_copy_keeps_the_layers_parent_and_opacity_expression() {
    let mut l = with(json!({"count": 2, "rotation": "time * 10", "opacity": 0.5}));
    l["parent"] = json!("rig");
    l["opacity"] = json!("smooth(0, 1, time)");
    let out = run(l).unwrap();
    assert_eq!(out[0]["rotation"], json!("0 + 1 * (time * 10)"));
    assert_eq!(out[0]["parent"], "rig");
    assert_eq!(out[1]["parent"], "card~r1");
    assert_eq!(out[1]["opacity"], json!("(smooth(0, 1, time)) * 0.5"));
}

#[test]
fn three_d_layers_get_three_d_nulls() {
    let mut l = with(json!({"count": 2, "offset": [0, 0, -50], "rotation": [0, 15, 0]}));
    l["three_d"] = json!(true);
    let out = run(l).unwrap();
    assert_eq!(out[0]["three_d"], true);
    assert_eq!(out[0]["pos"], json!([0.0, 0.0, -50.0]));
    assert_eq!(out[0]["rotation"], json!([0.0, 15.0, 0.0]));
}

#[test]
fn a_palette_stamps_a_swatch_on_the_original_and_every_copy_and_is_checked_where_it_is_written() {
    let palette = json!({"count": 3, "palette": ["#f00", "#0f0"], "cycle": {"mode": "gradient", "offset": 0.5}, "paint": "both"});
    let out = run(with(palette)).unwrap();
    let swatch = |id: &str| out.iter().find(|l| l["id"] == id).map(|l| l["swatch"].clone()).unwrap_or_default();
    for (id, index) in [("card", 0), ("card~c1", 1), ("card~c2", 2)] {
        assert_eq!(swatch(id), json!({"palette": ["#f00", "#0f0"], "cycle": {"mode": "gradient", "offset": 0.5}, "paint": "both", "index": index}), "{id}");
    }
    assert!(out.iter().filter(|l| l["type"] == "null").all(|w| w.get("swatch").is_none()), "the nulls are not coloured");
    let refused = |spec: Value| format!("{:#}", run(with(spec)).unwrap_err());
    assert!(refused(json!({"count": 2, "palette": []})).contains("palette has no colours"));
    assert!(refused(json!({"count": 2, "palette": ["#f00", 3]})).contains("palette[1]"));
    assert!(refused(json!({"count": 2, "cycle": {"mode": "step"}})).contains("add a `palette`"));
    assert!(refused(json!({"count": 2, "palete": ["#f00"]})).contains("Did you mean \"palette\"?"));
    let mut own = with(json!({"count": 2, "palette": ["#f00"]}));
    own["swatch"] = json!({"palette": ["#fff"], "index": 0});
    assert!(format!("{:#}", run(own).unwrap_err()).contains("swatch` of its own"));
}

#[test]
fn mistakes_are_named() {
    let refused = |spec: Value| format!("{:#}", run(with(spec)).unwrap_err());
    assert!(refused(json!({"offset": [1, 0]})).contains("count"));
    assert!(refused(json!({"count": 1})).contains("2 to"));
    assert!(refused(json!({"count": 3, "ofset": [1, 0]})).contains("ofset"));
    assert!(refused(json!({"count": 3, "scale": 0})).contains("above 0"));
    assert!(refused(json!({"count": 3, "order": "front"})).contains("order"));
    assert!(refused(json!({"count": 3, "offset": [1, 2, 3]})).contains("flat"));
    let mut l = card();
    l.as_object_mut().unwrap().remove("id");
    l["repeat"] = json!({"count": 2});
    assert!(format!("{:#}", run(l).unwrap_err()).contains("needs an \"id\""));
}
