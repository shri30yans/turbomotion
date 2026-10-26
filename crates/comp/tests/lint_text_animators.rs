//! Text animators that silently do nothing are reported: a positive opacity delta and a selector that reaches no character.
use std::path::Path;

use serde_json::{json, Value};
use turbomotion_comp::{check::lint, plan::Plan};

fn rules(animator: Value) -> Vec<&'static str> {
    let film = json!({"width": 640, "height": 360, "fps": 30, "dur": 2, "layers": [
        {"type": "text", "id": "t", "text": "SWEEP ME", "size": 60, "color": "#fff", "pos": [320, 180], "animators": [animator]}]});
    let plan = Plan::from_value(film, Path::new(".")).unwrap();
    lint::run(&plan).unwrap().iter().map(|f| f.rule).collect()
}

#[test]
fn a_selector_window_that_reaches_no_character_is_reported() {
    let r = rules(json!({"by": "chars", "props": {"opacity": -1}, "selector": {"start": 0.0, "end": 0.2, "offset": 5}}));
    assert!(r.contains(&"animator-never-applies"), "{r:?}");
}

#[test]
fn a_selector_window_that_sweeps_across_the_text_is_fine() {
    let r = rules(json!({"by": "chars", "props": {"opacity": -1}, "selector": {"start": 0.0, "end": 0.2, "offset": {"from": -0.2, "to": 1, "dur": 2, "ease": "none"}}}));
    assert!(!r.contains(&"animator-never-applies") && !r.contains(&"animator-opacity-delta"), "{r:?}");
}

#[test]
fn a_positive_opacity_delta_is_reported_because_opacity_is_already_full() {
    let r = rules(json!({"by": "chars", "props": {"opacity": 1}, "stagger": {"each": 0.05}, "at": 0.2, "dur": 0.4}));
    assert!(r.contains(&"animator-opacity-delta"), "{r:?}");
}
