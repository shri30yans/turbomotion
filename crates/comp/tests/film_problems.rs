//! A film is validated whole: every mistake in every layer is reported together, each with its layer and key, a
//! "did you mean" where a name is near, and the forms an effect parameter accepts.
use std::path::Path;

use serde_json::{json, Value};
use turbomotion_comp::{plan::Plan, schema::Problems};

fn plan(film: Value) -> anyhow::Result<Plan> {
    Plan::from_value(film, Path::new("."))
}

fn problems(film: Value) -> Problems {
    let e = plan(film).err().expect("the film has mistakes");
    e.downcast_ref::<Problems>().unwrap_or_else(|| panic!("not a Problems error: {e:#}")).clone()
}

#[test]
fn every_layer_with_a_mistake_is_reported_in_one_pass() {
    let p = problems(json!({"width": 64, "height": 64, "fps": 30, "dur": 1, "layers": [
        {"type": "text", "id": "a", "text": "hi", "dure": 2, "fxx": []},
        {"type": "mesh", "id": "b", "three_d": true, "geometry": {"kind": "cylinder", "size": 5, "hight": 3}},
        {"type": "solid", "id": "c", "color": "#fff", "fx": [{"effect": "blur", "radis": 3}, {"effect": "tint", "color": [1, 2]}]},
        {"type": "text", "id": "d", "text": "ok"}
    ]}));
    let ids: Vec<_> = p.0.iter().map(|p| p.id.clone().unwrap()).collect();
    assert!(ids.contains(&"a".into()) && ids.contains(&"b".into()) && ids.contains(&"c".into()) && !ids.contains(&"d".into()), "{ids:?}");
    let all = p.to_string();
    // two unknown keys of layer a, each named, one with its nearest valid key
    assert!(all.contains("unknown key \"dure\". Did you mean \"dur\"?") && all.contains("unknown key \"fxx\". Did you mean \"fx\"?"), "{all}");
    // a nested unknown key names the nearest valid one too
    assert!(all.contains("hight") && all.contains("Did you mean \"height\"?"), "{all}");
    // both effect mistakes of layer c arrive in one message
    assert!(all.contains("radis") && all.contains("parameter \"color\"") && all.contains("[r, g, b]"), "{all}");
}

#[test]
fn a_group_cannot_carry_effects_because_it_draws_nothing() {
    let e = format!("{:#}", plan(json!({"dur": 1, "layers": [{"type": "null", "id": "g", "fx": [{"effect": "blur"}]}]})).err().unwrap());
    assert!(e.contains("null layers cannot carry \"fx\"") && e.contains("each child layer") && e.contains("precomp"), "{e}");
    assert!(plan(json!({"dur": 1, "layers": [{"type": "adjustment", "fx": [{"effect": "blur"}]}]})).is_ok());
}

#[test]
fn a_missing_font_is_a_plan_error_that_says_how_to_fix_it() {
    let e = format!("{:#}", plan(json!({"dur": 1, "layers": [{"type": "text", "id": "t", "text": "x", "font": "Jetbrains Mono Zzz"}]})).err().unwrap());
    assert!(e.contains("\"Jetbrains Mono Zzz\" is not an installed font family") && e.contains("comp({ fonts:") && e.contains("comp fonts"), "{e}");
}

#[test]
fn a_font_file_in_the_root_is_offered_by_name() {
    let dir = std::env::temp_dir().join(format!("tm-font-hint-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("fonts")).unwrap();
    std::fs::write(dir.join("fonts/QuuxMono-Regular.woff"), b"x").unwrap();
    let e = format!("{:#}", Plan::from_value(json!({"dur": 1, "layers": [{"type": "text", "text": "x", "font": "Quux Mono"}]}), &dir).err().unwrap());
    assert!(e.contains("fonts/QuuxMono-Regular.woff") && e.contains("\"Quux Mono\": \"fonts/QuuxMono-Regular.woff\""), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_text_object_that_is_not_a_counter_says_what_it_should_be() {
    let e = format!("{:#}", plan(json!({"dur": 1, "layers": [{"type": "text", "text": {"text": "hi"}}]})).err().unwrap());
    assert!(e.contains("counter") && e.contains("keys text") && !e.contains("expected one of `from`"), "{e}");
}

#[test]
fn top_level_and_shot_keys_are_checked_together_with_hints() {
    let e = format!("{:#}", plan(json!({"dur": 1, "layrs": [], "fpss": 30})).err().unwrap());
    assert!(e.contains("\"layrs\". Did you mean \"layers\"?") && e.contains("\"fpss\""), "{e}");
    let e = format!("{:#}", plan(json!({"width": 64, "height": 64, "fps": 30, "timeline": {"shots": [{"id": "a", "dur": 1, "durr": 2, "fxs": []}]}})).err().unwrap());
    assert!(e.contains("\"durr\". Did you mean \"dur\"?") && e.contains("\"fxs\""), "{e}");
}
