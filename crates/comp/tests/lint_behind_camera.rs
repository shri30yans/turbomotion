//! A wide 3D layer on a turning parent loses a corner behind the lens and is dropped whole; `check` says so.
use std::path::Path;

use serde_json::json;
use turbomotion_comp::{check::lint, plan::Plan};

fn film(repeat: u32) -> serde_json::Value {
    json!({"width": 1280, "height": 720, "fps": 24, "dur": 3, "bg": "#fff", "layers": [
        {"type": "null", "id": "root", "three_d": true, "pos": [640, 360, 0], "rotation": [-15, 40, 8]},
        {"type": "null", "id": "hinge", "three_d": true, "parent": "root", "rotation": [0, {"from": -90, "to": 0, "dur": 1.2}, 0]},
        {"type": "text", "id": "row", "text": "ECHOES OF PERSPECTIVES", "size": 120, "color": "#000", "three_d": true, "parent": "hinge", "pos": [210, 0, -1],
         "marquee": {"speed": 120, "gap": 40, "repeat": repeat}},
        {"type": "camera", "pos": [640, 360, -1100], "poi": [640, 360, 0], "zoom": 1100}
    ]})
}

fn rules(repeat: u32) -> Vec<&'static str> {
    let plan = Plan::from_value(film(repeat), Path::new(".")).unwrap();
    lint::run(&plan).unwrap().iter().map(|f| f.rule).collect()
}

#[test]
fn a_layer_whose_corner_passes_behind_the_lens_is_reported_with_the_fix() {
    let plan = Plan::from_value(film(3), Path::new(".")).unwrap();
    let findings = lint::run(&plan).unwrap();
    let f = findings.iter().find(|f| f.rule == "passes-behind-camera").expect("the row disappears when the hinge unfolds");
    assert!(f.layer.as_deref().is_some_and(|l| l.contains("row")) && f.t.is_some() && f.fix.contains("repeat"), "{f}");
}

#[test]
fn a_narrow_row_in_the_same_move_is_not_reported() {
    assert!(!rules(1).contains(&"passes-behind-camera"), "{:?}", rules(1));
}
