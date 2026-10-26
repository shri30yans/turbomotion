//! `comp catalogue layer <type>` lists exactly the keys the parser accepts, with forms and docs, and its example runs.
use std::path::Path;

use serde_json::{json, Value};
use turbomotion_comp::{catalogue::layer_keys, fxlib::Library, plan::Plan, schema::Kind};

fn keys(v: &Value, list: &str) -> Vec<String> {
    v[list].as_array().unwrap().iter().map(|k| k["key"].as_str().unwrap().to_string()).collect()
}

/// A form that is still a bare Rust type name (`Fill`, `Blend`) tells an author nothing.
fn unresolved(rows: &Value) -> Vec<String> {
    rows.as_array().unwrap().iter().filter(|r| r["form"].as_str().unwrap().chars().all(|c| c.is_ascii_alphanumeric()) && r["form"].as_str().unwrap().starts_with(char::is_uppercase)).map(|r| format!("{} -> {}", r["key"], r["form"])).collect()
}

#[test]
fn every_layer_type_lists_its_keys_with_readable_forms() {
    for ty in Kind::NAMES {
        let v = layer_keys::layer(ty).unwrap_or_else(|e| panic!("{ty}: {e:#}"));
        assert_eq!(v["type"], *ty);
        assert!(unresolved(&v["keys"]).is_empty(), "{ty}: {:?}", unresolved(&v["keys"]));
        assert!(unresolved(&v["common_keys"]).is_empty(), "{ty}: {:?}", unresolved(&v["common_keys"]));
    }
}

#[test]
fn the_example_of_a_layer_that_needs_no_file_is_a_layer_the_planner_accepts() {
    for ty in Kind::NAMES.iter().filter(|t| !["pack", "image", "video", "audio", "precomp"].contains(t)) {
        let v = layer_keys::layer(ty).unwrap();
        let film = json!({"width": 640, "height": 360, "fps": 30, "dur": 1, "layers": [v["example"].clone()]});
        Plan::from_value(film, Path::new(".")).unwrap_or_else(|e| panic!("{ty}: {e:#}"));
    }
}

#[test]
fn the_text_keys_come_with_their_forms_and_docs() {
    let v = layer_keys::layer("text").unwrap();
    let find = |key: &str| v["keys"].as_array().unwrap().iter().find(|k| k["key"] == key).unwrap().clone();
    assert!(keys(&v, "keys").contains(&"animators".to_string()));
    assert!(find("size")["form"].as_str().unwrap().contains("expression string") && find("size")["required"] == false);
    assert_eq!(find("text")["required"], true);
    assert!(find("text")["form"].as_str().unwrap().contains("counter"));
    assert!(find("marquee")["form"].as_str().unwrap().contains("speed") && find("marquee")["doc"].as_str().unwrap().contains("scrolling"));
}

#[test]
fn the_common_keys_are_the_parsers_with_forms_and_are_part_of_the_catalogue() {
    let all = turbomotion_comp::catalogue::all();
    let common = keys(&all, "layer_common_keys");
    for k in ["id", "pos", "scale", "rotation", "opacity", "start", "dur", "blend", "fx", "masks", "matte", "clip_to", "parent", "three_d"] {
        assert!(common.contains(&k.to_string()), "{k} missing from {common:?}");
    }
    let rows = all["layer_common_keys"].as_array().unwrap();
    assert!(rows.iter().all(|r| !r["form"].as_str().unwrap().is_empty()), "every common key has a form: {rows:?}");
    let pos = rows.iter().find(|r| r["key"] == "pos").unwrap();
    assert!(pos["form"].as_str().unwrap().contains("[x, y]"), "{pos}");
}

#[test]
fn a_wrong_layer_or_effect_name_suggests_the_nearest() {
    let e = layer_keys::layer("tex").err().unwrap().to_string();
    assert!(e.contains("Did you mean \"text\"?"), "{e}");
    let lib = Library::builtin().unwrap();
    let e = layer_keys::effect(&lib, "blurr").err().unwrap().to_string();
    assert!(e.contains("Did you mean \"blur\"?"), "{e}");
    assert_eq!(layer_keys::effect(&lib, "echo").unwrap()["name"], "echo");
}

#[test]
fn shape_lists_its_item_keys() {
    let v = layer_keys::layer("shape").unwrap();
    let items = keys(&v, "item_keys");
    assert!(items.contains(&"ellipse".to_string()) && items.contains(&"stroke".to_string()), "{items:?}");
}
