//! The paint order: a run of layers in the shared 3D space sorts far to near, and a flat layer between two of
//! them is a wall that stops them sorting against each other.

use super::*;
use crate::plan::{Plan, ROOT};
use serde_json::{json, Value};
use std::path::Path;

fn order(v: Value) -> Vec<usize> {
    let plan = Plan::from_value(v, Path::new(".")).expect("the plan");
    let frame = crate::eval::Evaluator::new(&plan).frame(ROOT, 0.0).expect("the frame");
    draw_order(&frame)
}

const CAMERA: fn() -> serde_json::Value = || json!({"type": "camera", "id": "cam", "pos": [0, 0, -1000], "poi": [0, 0, 0]});

#[test]
fn a_run_of_layers_in_the_3d_space_sorts_far_to_near() {
    let order = order(json!({"dur": 1, "layers": [
        CAMERA(),
        {"type": "solid", "id": "near", "color": "#fff", "three_d": true, "pos": [0, 0, 100]},
        {"type": "solid", "id": "far", "color": "#fff", "three_d": true, "pos": [0, 0, 1000]}
    ]}));
    // the far layer is drawn first, so the near one covers it
    assert_eq!(order, vec![0, 2, 1]);
}

#[test]
fn a_flat_layer_between_two_of_them_is_a_wall() {
    let order = order(json!({"dur": 1, "layers": [
        CAMERA(),
        {"type": "solid", "id": "far", "color": "#fff", "three_d": true, "pos": [0, 0, 1000]},
        {"type": "solid", "id": "wall", "color": "#fff"},
        {"type": "solid", "id": "near", "color": "#fff", "three_d": true, "pos": [0, 0, 100]}
    ]}));
    // the two layers in the 3D space cannot sort against each other: the flat one between them is a wall
    assert_eq!(order, vec![0, 1, 2, 3]);
}

#[test]
fn a_light_has_no_place_in_the_scene_and_does_not_break_a_run() {
    let order = order(json!({"dur": 1, "layers": [
        CAMERA(),
        {"type": "solid", "id": "near", "color": "#fff", "three_d": true, "pos": [0, 0, 100]},
        {"type": "light", "id": "l", "kind": "ambient", "pos": [0, 0, 0]},
        {"type": "solid", "id": "far", "color": "#fff", "three_d": true, "pos": [0, 0, 1000]}
    ]}));
    // the two layers in the 3D space still sort against each other; the light keeps the slot the timeline gave it
    assert_eq!(order, vec![0, 3, 2, 1]);
}
