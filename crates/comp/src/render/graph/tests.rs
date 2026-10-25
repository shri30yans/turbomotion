//! The graph's shape, without a GPU: what is emitted, in what order, and what is shared.

use std::path::Path;

use serde_json::{json, Value};

use super::*;
use crate::{eval::Evaluator, plan::{Plan, ROOT}};

fn plan_of(layers: Vec<Value>) -> Plan {
    Plan::from_value(json!({"width": 64, "height": 32, "dur": 1, "layers": layers}), Path::new(".")).expect("the plan")
}

/// The passes of the root frame at t = 0.
fn passes(plan: &Plan) -> Vec<Pass> {
    let frame = Evaluator::new(plan).frame(ROOT, 0.0).expect("the frame");
    comp_graph(plan.root(), &frame, &[]).passes
}

fn count(passes: &[Pass], is: impl Fn(&Stage) -> bool) -> usize {
    passes.iter().filter(|p| is(&p.stage)).count()
}

#[test]
fn every_pass_reads_only_images_an_earlier_pass_wrote() {
    let plan = plan_of(vec![
        json!({"type": "solid", "id": "m", "color": "#fff"}),
        json!({"type": "solid", "color": "#f00", "matte": {"source": "m", "mode": "luma"}, "fx": [{"effect": "blur", "radius": 2}]}),
        json!({"type": "adjustment", "fx": [{"effect": "vignette"}]}),
    ]);
    let mut written = std::collections::HashSet::new();
    for p in passes(&plan) {
        assert!(p.reads.iter().all(|r| written.contains(r)), "a pass reads an image nothing has written yet");
        if let Write::New(id) = p.write {
            assert!(written.insert(id), "an image is written twice");
        }
    }
}

#[test]
fn what_is_not_drawn_is_not_in_the_graph() {
    let plan = plan_of(vec![json!({"type": "solid", "id": "m", "color": "#fff"}), json!({"type": "solid", "color": "#f00", "hidden": true}), json!({"type": "solid", "color": "#00f", "opacity": 0})]);
    assert_eq!(count(&passes(&plan), |s| matches!(s, Stage::Paint { .. })), 1, "the hidden and the transparent layers are never painted");
}

#[test]
fn a_layer_that_several_others_read_is_drawn_once() {
    let user = |matte: Option<&str>| {
        let mut l = json!({"type": "solid", "color": "#f00", "fx": [{"effect": "displacement_map", "map": "src"}]});
        if let Some(m) = matte {
            l["matte"] = json!({"source": m, "mode": "alpha"});
        }
        l
    };
    let plan = plan_of(vec![json!({"type": "solid", "id": "src", "color": "#fff", "hidden": true}), user(Some("src")), user(None)]);
    let ps = passes(&plan);
    assert_eq!(count(&ps, |s| matches!(s, Stage::Paint { layer: 0 })), 1, "the map and matte source is painted once for both readers");
    let effects: Vec<&Pass> = ps.iter().filter(|p| matches!(&p.stage, Stage::Effects { pictures, .. } if pictures == &[0])).collect();
    assert_eq!(effects.len(), 2, "each reader's effects read the picture of layer 0");
    assert_eq!(effects[0].reads[1], effects[1].reads[1], "and it is the same image");
}

#[test]
fn images_are_dropped_after_their_last_reader() {
    let layers: Vec<Value> = (0..12).map(|_| json!({"type": "solid", "color": "#fff", "fx": [{"effect": "blur", "radius": 1}]})).collect();
    let plan = plan_of(layers);
    let frame = Evaluator::new(&plan).frame(ROOT, 0.0).expect("the frame");
    let graph = comp_graph(plan.root(), &frame, &[]);
    let images = graph.passes.iter().filter(|p| matches!(p.write, Write::New(_))).count();
    assert!(images >= 12 * 2, "twelve layers make at least two images each ({images})");
    assert!(graph.peak_images() <= 3, "but never more than three are alive together, not {}", graph.peak_images());
}

#[test]
fn the_description_lists_every_pass_and_goes_into_nested_compositions() {
    let value = json!({"width": 64, "height": 32, "dur": 1, "comps": {"inner": {"width": 64, "height": 32, "dur": 1, "layers": [{"type": "solid", "color": "#fff"}]}},
        "layers": [{"type": "precomp", "comp": "inner"}]});
    let plan = Plan::from_value(value, Path::new(".")).expect("the plan");
    let text = describe(&plan, &Evaluator::new(&plan), 0.0).expect("the description");
    assert!(text.contains("comp root") && text.contains("comp inner"), "{text}");
    assert!(text.contains("backdrop") && text.contains("composite normal"), "{text}");
}

#[test]
fn the_inputs_of_an_effect_are_the_layers_its_shader_samples_not_its_composite_mask() {
    let plan = plan_of(vec![
        json!({"type": "solid", "id": "map", "color": "#fff", "hidden": true}),
        json!({"type": "solid", "id": "mask", "color": "#fff", "hidden": true}),
        json!({"type": "solid", "color": "#f00", "fx": [{"effect": "displacement_map", "map": "map", "composite": {"mask": {"kind": "layer", "layer": "mask"}}}]}),
    ]);
    let frame = Evaluator::new(&plan).frame(ROOT, 0.0).expect("the frame");
    let (graph, read) = effect_inputs_graph(plan.root(), &frame, 2, 0);
    assert_eq!(read, vec![0], "only the map");
    assert_eq!(count(&graph.passes, |s| matches!(s, Stage::Paint { layer: 1 })), 0, "the mask layer is not drawn for a history frame");
    assert_eq!(graph.outputs.len(), 2, "the effect's own input picture and the map");
}
