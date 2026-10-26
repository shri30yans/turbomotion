//! The graph of a layer-space composition: which stages a layer gets, in what order, and what is alive at once.

use std::path::Path;

use serde_json::{json, Value};

use super::*;
use crate::{eval::Evaluator, plan::{Plan, ROOT}};

fn graph_at(space: &str, t: f64, layers: Vec<Value>) -> Vec<Stage> {
    let plan = Plan::from_value(json!({"width": 64, "height": 32, "dur": 1, "effect_space": space, "layers": layers}), Path::new(".")).expect("the plan");
    let frame = Evaluator::new(&plan).frame(ROOT, t).expect("the frame");
    comp_graph(plan.root(), &frame, &[]).passes.into_iter().map(|p| p.stage).collect()
}

fn graph_of(space: &str, layers: Vec<Value>) -> Vec<Stage> {
    graph_at(space, 0.0, layers)
}

fn names(stages: &[Stage]) -> Vec<&'static str> {
    stages
        .iter()
        .map(|s| match s {
            Stage::Backdrop => "backdrop",
            Stage::Paint { .. } => "paint",
            Stage::PaintLocal { .. } => "paint_local",
            Stage::Effects { .. } => "effects",
            Stage::MaskCoverage { .. } => "mask",
            Stage::MaskCoverageLocal { .. } => "mask_local",
            Stage::Place { .. } => "place",
            Stage::Finish { .. } => "finish",
            Stage::MatteCoverage { .. } => "matte",
            Stage::Multiply => "multiply",
            Stage::Snapshot => "snapshot",
            Stage::Composite { placed: Some(_), .. } => "composite_placed",
            Stage::Composite { .. } => "composite",
            _ => "other",
        })
        .collect()
}

fn blurred() -> Value {
    json!({"type": "solid", "color": "#f00", "size": [20, 20], "fx": [{"effect": "blur", "radius": 2}]})
}

#[test]
fn a_layer_with_effects_is_painted_in_its_own_pixels_and_placed_by_the_composite() {
    let stages = names(&graph_of("layer", vec![blurred()]));
    assert_eq!(stages, ["backdrop", "paint_local", "effects", "composite_placed", "snapshot"], "no frame-sized picture is made: the composite carries it through the layer's matrix");
}

#[test]
fn the_default_pipeline_still_paints_a_frame_sized_picture() {
    let stages = names(&graph_of("screen", vec![blurred()]));
    assert_eq!(stages, ["backdrop", "paint", "effects", "composite", "snapshot"]);
}

#[test]
fn masks_come_after_the_effects_and_before_the_transform() {
    let mut layer = blurred();
    layer["masks"] = json!([{"path": {"rect": [0, 0, 10, 10]}}]);
    let stages = names(&graph_of("layer", vec![layer]));
    assert_eq!(stages, ["backdrop", "paint_local", "effects", "mask_local", "multiply", "composite_placed", "snapshot"]);
}

#[test]
fn a_layer_that_is_matted_is_placed_before_the_matte_which_works_in_the_frame() {
    let matte = json!({"type": "solid", "id": "m", "color": "#fff", "size": [20, 20]});
    let mut layer = blurred();
    layer["matte"] = json!({"source": "m", "mode": "alpha"});
    let stages = names(&graph_of("layer", vec![matte, layer]));
    let (place, matte) = (stages.iter().position(|s| *s == "place").expect("placed"), stages.iter().position(|s| *s == "matte").expect("matted"));
    assert!(place < matte, "{stages:?}");
    assert!(!stages.contains(&"composite_placed"), "a matted layer is placed first, then composited from the frame: {stages:?}");
}

#[test]
fn the_blur_of_a_transition_runs_in_the_frame_after_the_layer_is_placed() {
    let mut layer = blurred();
    layer["enter"] = json!({"preset": "blur", "dur": 1, "blur": 60, "ease": "none"});
    let stages = names(&graph_at("layer", 0.3, vec![layer]));
    assert_eq!(stages, ["backdrop", "paint_local", "effects", "place", "finish", "composite", "snapshot"]);
}

#[test]
fn a_layer_without_effects_or_masks_is_painted_in_the_frame_even_in_layer_space() {
    let plain = json!({"type": "solid", "color": "#f00", "size": [20, 20]});
    let stages = names(&graph_of("layer", vec![plain]));
    assert_eq!(stages, ["backdrop", "paint", "composite", "snapshot"], "nothing to run before the transform");
}

#[test]
fn twelve_layer_space_layers_never_hold_more_than_three_pictures_at_once() {
    let plan = Plan::from_value(json!({"width": 64, "height": 32, "dur": 1, "effect_space": "layer", "layers": (0..12).map(|_| blurred()).collect::<Vec<_>>()}), Path::new(".")).expect("the plan");
    let frame = Evaluator::new(&plan).frame(ROOT, 0.0).expect("the frame");
    let graph = comp_graph(plan.root(), &frame, &[]);
    assert!(graph.peak_images() <= 3, "{}", graph.peak_images());
}

#[test]
fn the_description_names_the_largest_picture_a_layer_is_painted_into() {
    let hundred = |space: &str| {
        let layers: Vec<Value> = (0..100).map(|i| json!({"type": "solid", "color": "#f00", "size": [40, 40], "pos": [40 + (i % 10) * 180, 60 + (i / 10) * 95], "fx": [{"effect": "blur", "radius": 6}]})).collect();
        let plan = Plan::from_value(json!({"width": 1920, "height": 1080, "dur": 1, "effect_space": space, "layers": layers}), Path::new(".")).expect("the plan");
        describe(&plan, &Evaluator::new(&plan), 0.0).expect("the description")
    };
    let head = |text: String| text.lines().next().unwrap_or_default().to_string();
    let (screen, layer) = (head(hundred("screen")), head(hundred("layer")));
    assert!(screen.contains("largest layer picture 1920x1080 px"), "{screen}");
    assert!(layer.contains("largest layer picture 76x76 px"), "40 px + 2 x 18 px of a radius 6 blur: {layer}");
    assert!(layer.contains("at most 2 images alive at once"), "{layer}");
}
