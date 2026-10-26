//! The composition key `"color"`: accepted, left out means unmanaged, and mistakes are answered with the nearest valid
//! key or the valid values rather than ignored.
use std::path::Path;

use serde_json::{json, Value};
use turbomotion_comp::plan::{Plan, ROOT};
use turbomotion_core::color::{Pipeline, Primaries, Profile, Transfer};

fn plan(extra: Value) -> anyhow::Result<Plan> {
    let mut film = json!({"width": 64, "height": 48, "fps": 30, "dur": 1, "comps": {"inner": {"layers": []}}, "layers": [{"type": "precomp", "comp": "inner"}]});
    film.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    Plan::from_value(film, Path::new("."))
}

fn error(extra: Value) -> String {
    format!("{:#}", plan(extra).err().expect("rejected"))
}

#[test]
fn leaving_the_key_out_is_the_unmanaged_default() {
    assert_eq!(plan(json!({})).unwrap().comps[ROOT].color, Pipeline::default());
    assert!(!plan(json!({"color": {}})).unwrap().comps[ROOT].color.is_managed());
}

#[test]
fn linear_light_and_gamuts_load_and_nested_compositions_inherit_them() {
    let p = plan(json!({"color": {"linear_light": true, "working_profile": "display_p3", "space": "display_p3"}})).unwrap();
    let (root, inner) = (p.comps[ROOT].color, p.comps["inner"].color);
    assert_eq!(root.working, Profile { primaries: Primaries::DisplayP3, transfer: Transfer::Linear });
    assert_eq!(root.output, Profile { primaries: Primaries::DisplayP3, transfer: Transfer::Srgb });
    assert_eq!(inner, root, "a nested composition takes the film's colour setting");
}

#[test]
fn a_mistyped_key_names_the_nearest_valid_one() {
    let e = error(json!({"color": {"linar_light": true}}));
    assert!(e.contains("linar_light") && e.contains("linear_light"), "{e}");
    let e = error(json!({"colour": {"linear_light": true}}));
    assert!(e.contains("unknown key \"colour\"") && e.contains("Did you mean \"color\"?"), "{e}");
}

#[test]
fn a_mistyped_value_lists_the_valid_ones() {
    let e = error(json!({"color": {"space": "p3"}}));
    assert!(e.contains("srgb") && e.contains("display_p3"), "{e}");
    let e = error(json!({"color": {"depth": "full"}}));
    assert!(e.contains("half"), "{e}");
}

#[test]
fn a_nested_composition_cannot_choose_the_frames_output_space() {
    let e = error(json!({"comps": {"inner": {"layers": [], "color": {"space": "display_p3"}}}}));
    assert!(e.contains("inner") && e.contains("only the root"), "{e}");
}
