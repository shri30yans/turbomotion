//! Cameras, lights and text size as the evaluator reports them.

use super::*;
use crate::plan::ROOT;
use serde_json::json;

#[test]
fn cameras_and_lights_follow_their_animated_position_and_lights_reach_3d_layers() {
    let plan = Plan::from_value(
        json!({"dur": 4, "layers": [
            {"type": "camera", "pos": {"from": [0, 0, -1000], "to": [400, 0, -1000], "dur": 4, "ease": "none"}, "dof": {"focus": 1000, "blur": 10}},
            {"type": "light", "kind": "point", "pos": [100, 200, -300]},
            {"type": "solid", "color": "#fff", "id": "far", "three_d": true, "pos": [640, 360, 1000]},
            {"type": "solid", "color": "#fff", "id": "flat", "three_d": true, "lit": false, "pos": [640, 360, 0]}
        ]}),
        std::path::Path::new("."),
    )
    .unwrap();
    let f = Evaluator::new(&plan).frame(ROOT, 2.0).unwrap();
    assert_eq!(f.camera.as_ref().unwrap().pos, [200.0, 0.0, -1000.0]);
    assert_eq!(f.lights[0].pos, [100.0, 200.0, -300.0]);
    let (far, flat) = (&f.layers[2], &f.layers[3]);
    assert!(far.lit.is_some() && flat.lit.is_none());
    // the layer far behind the focal distance is much softer than the one near it
    assert!(far.extra_blur > 5.0 && flat.extra_blur < 2.0, "{} {}", far.extra_blur, flat.extra_blur);
}

#[test]
fn a_text_layer_is_as_wide_as_its_glyphs() {
    let plan = Plan::from_value(
        json!({"dur": 2, "layers": [
            {"type": "text", "id": "hi", "text": "Hi", "size": 80, "color": "#fff", "pos": [960, 540], "align": "center"}
        ]}),
        std::path::Path::new("."),
    )
    .unwrap();
    let ev = Evaluator::new(&plan);
    let size = ev.layer_size(plan.root(), &plan.root().layers[0]);
    assert!(size[0] < 400.0, "text used the full frame: {size:?}");
    assert!(size[1] < 200.0, "text used the full frame: {size:?}");
}
