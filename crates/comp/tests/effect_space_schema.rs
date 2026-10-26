//! The composition key `"effect_space"`: accepted, defaulted to screen, and mistyped names are answered with the
//! nearest valid key (or the valid values) rather than ignored.
use std::path::Path;

use serde_json::{json, Value};
use turbomotion_comp::{plan::Plan, schema::EffectSpace};

fn plan(extra: Value) -> anyhow::Result<Plan> {
    let mut film = json!({"width": 64, "height": 48, "fps": 30, "dur": 1, "layers": [{"type": "solid", "color": "#fff"}]});
    film.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    Plan::from_value(film, Path::new("."))
}

fn error(extra: Value) -> String {
    format!("{:#}", plan(extra).err().expect("rejected"))
}

#[test]
fn both_values_load_and_the_default_is_screen() {
    assert_eq!(plan(json!({})).unwrap().comps[turbomotion_comp::plan::ROOT].effect_space, EffectSpace::Screen);
    assert_eq!(plan(json!({"effect_space": "screen"})).unwrap().comps[turbomotion_comp::plan::ROOT].effect_space, EffectSpace::Screen);
    assert_eq!(plan(json!({"effect_space": "layer"})).unwrap().comps[turbomotion_comp::plan::ROOT].effect_space, EffectSpace::Layer);
}

#[test]
fn a_mistyped_key_names_the_nearest_valid_one() {
    let e = error(json!({"effect_spce": "layer"}));
    assert!(e.contains("unknown key \"effect_spce\"") && e.contains("Did you mean \"effect_space\"?"), "{e}");
    let e = error(json!({"effectspace": "layer"}));
    assert!(e.contains("Did you mean \"effect_space\"?"), "{e}");
}

#[test]
fn a_mistyped_value_lists_the_valid_ones() {
    let e = error(json!({"effect_space": "lyer"}));
    assert!(e.contains("effect_space") && e.contains("`screen` or `layer`"), "{e}");
}
