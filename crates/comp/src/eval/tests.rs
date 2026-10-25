//! The 3D base layer: where a camera looks, where a layer stands, and what the camera can see.

use super::*;
use crate::plan::{Plan, ROOT};
use serde_json::{json, Value};
use std::path::Path;

fn frame(v: Value, t: f64) -> Frame {
    let plan = Plan::from_value(v, Path::new(".")).expect("the plan");
    Evaluator::new(&plan).frame(ROOT, t).expect("the frame")
}

/// The direction the camera looks, in the world.
fn forward(cam: &CameraState) -> [f64; 3] {
    [cam.view.0[8], cam.view.0[9], cam.view.0[10]]
}

/// The layer's own depth axis in the world: the way its face points away from whoever reads it.
fn depth_axis(layer: &LayerState) -> [f64; 3] {
    [layer.world.0[2], layer.world.0[6], layer.world.0[10]]
}

#[test]
fn a_camera_with_an_orientation_looks_the_way_after_effects_does() {
    let f = frame(json!({"dur": 1, "layers": [{"type": "camera", "id": "cam", "pos": [0, 0, 0], "poi": [0, 0, 1000], "orientation": [30, 40, 0]}]}), 0.0);
    let cam = f.camera.expect("the camera");
    // After Effects 2026: a camera with an Orientation of (30, 40, 0) looks along [0.643, -0.383, 0.663].
    let d = forward(&cam);
    assert!((d[0] - 0.643).abs() < 1e-3 && (d[1] + 0.383).abs() < 1e-3 && (d[2] - 0.663).abs() < 1e-3, "{d:?}");
}

#[test]
fn a_camera_rotation_composes_like_an_orientation() {
    // source: Skottie and lottie-web (Rx · Ry · Rz for both); not measured in AE for Rotation
    let f = frame(json!({"dur": 1, "layers": [{"type": "camera", "id": "cam", "pos": [0, 0, 0], "poi": [0, 0, 1000], "rotation": [30, 40, 0]}]}), 0.0);
    let d = forward(&f.camera.expect("the camera"));
    assert!((d[0] - 0.643).abs() < 1e-3 && (d[1] + 0.383).abs() < 1e-3 && (d[2] - 0.663).abs() < 1e-3, "{d:?}");
}

#[test]
fn a_camera_turns_about_its_own_position_and_leaves_the_point_of_interest_where_it_is() {
    let f = frame(json!({"dur": 1, "layers": [{"type": "camera", "id": "cam", "pos": [0, 0, -1000], "poi": [0, 0, 0], "orientation": [0, 90, 0]}]}), 0.0);
    let cam = f.camera.expect("the camera");
    assert_eq!(cam.pos, [0.0, 0.0, -1000.0], "the camera stands where it was put");
    assert_eq!(cam.poi, [0.0, 0.0, 0.0], "the point of interest stays in the world");
    // a quarter turn about the y axis looks to the right, not into the screen
    let d = forward(&cam);
    assert!((d[0] - 1.0).abs() < 1e-9 && d[1].abs() < 1e-9 && d[2].abs() < 1e-9, "{d:?}");
}

#[test]
fn a_layer_that_aims_at_a_point_faces_it() {
    let f = frame(json!({"dur": 1, "layers": [{"type": "solid", "id": "s", "color": "#fff", "pos": [0, 0, 0], "aim": [1000, 0, 0]}]}), 0.0);
    // the readable side is the local -z, so the layer's depth axis points away from the point it faces
    let d = depth_axis(&f.layers[0]);
    assert!((d[0] + 1.0).abs() < 1e-9 && d[1].abs() < 1e-9 && d[2].abs() < 1e-9, "{d:?}");
}

#[test]
fn a_layer_stands_in_the_shared_3d_space_when_it_or_its_parent_is_3d() {
    let f = frame(json!({"dur": 1, "layers": [
        {"type": "solid", "id": "flat", "color": "#fff"},
        {"type": "solid", "id": "cube", "color": "#fff", "three_d": true},
        {"type": "solid", "id": "child", "color": "#fff", "parent": "cube"}
    ]}), 0.0);
    assert!(!f.layers[0].in_3d, "a flat layer stays out of the scene");
    assert!(f.layers[1].in_3d);
    assert!(f.layers[2].in_3d, "a child of a 3D layer stands in the same space");
}

#[test]
fn a_culled_layer_is_hidden_from_the_camera_that_cannot_see_its_side() {
    let camera = |cull: Option<&str>| {
        let mut cam = json!({"type": "camera", "id": "cam", "pos": [0, 0, -1000], "poi": [0, 0, 0]});
        if let Some(c) = cull {
            cam["cull"] = json!(c);
        }
        frame(json!({"dur": 1, "layers": [cam, {"type": "solid", "id": "s", "color": "#fff", "three_d": true, "cull": cull, "pos": [0, 0, 0]}]}), 0.0)
    };
    let both = camera(None);
    assert!(both.layers[1].visible, "a layer shows both sides by default");
    // the camera is in front of the layer, so it reads the front: culling the front hides the layer
    let front = camera(Some("front"));
    assert!(!front.layers[1].visible, "the front is culled away");
    let back = camera(Some("back"));
    assert!(back.layers[1].visible, "the back is culled, the front still shows");
}

fn rot_block(m: &Mat4) -> Mat4 {
    Mat4([m.0[0], m.0[1], m.0[2], 0.0, m.0[4], m.0[5], m.0[6], 0.0, m.0[8], m.0[9], m.0[10], 0.0, 0.0, 0.0, 0.0, 1.0])
}

fn same(a: &Mat4, b: &Mat4) {
    for (x, y) in a.0.iter().zip(b.0) {
        assert!((x - y).abs() < 1e-9, "{a:?} vs {b:?}");
    }
}

#[test]
fn a_cameras_z_rotation_is_its_roll() {
    // derived: no separate roll; a camera Z Rotation of 30 tips the horizon anticlockwise on screen
    let f = frame(json!({"dur": 1, "layers": [{"type": "camera", "id": "cam", "pos": [0, 0, -1000], "poi": [0, 0, 0], "rotation": 30}]}), 0.0);
    let cam = f.camera.expect("the camera");
    let d = forward(&cam);
    assert!((d[2] - 1.0).abs() < 1e-12, "it still looks the same way: {d:?}");
    let p = cam.view.apply(100.0, 0.0, 0.0);
    assert!((p[0] - 86.6).abs() < 0.1 && (p[1] + 50.0).abs() < 0.1, "{p:?}");
}

#[test]
fn a_camera_and_a_layer_with_the_same_angles_share_one_pose() {
    // source: both go through eval::pose; the camera view's rotation is the transpose of the layer's world rotation
    let angles = json!({"orientation": [30, 40, 12], "rotation": [5, -20, 70]});
    let mut cam = json!({"type": "camera", "id": "cam", "pos": [0, 0, 0], "poi": [0, 0, 1000]});
    let mut sol = json!({"type": "solid", "id": "s", "color": "#fff", "pos": [0, 0, 0], "three_d": true});
    for (k, v) in angles.as_object().unwrap() {
        cam[k] = v.clone();
        sol[k] = v.clone();
    }
    let f = frame(json!({"dur": 1, "layers": [cam, sol]}), 0.0);
    same(&rot_block(&f.camera.unwrap().view), &rot_block(&f.layers[1].world).transpose_rotation());
}

#[test]
fn a_flat_layers_path_heading_turns_about_the_parents_z_axis_outside_its_orientation() {
    // source: py-aep (a 2D layer rotates Z by atan2(dy, dx)); the frame outside the Orientation is derived
    let f = frame(json!({"dur": 1, "layers": [{"type": "solid", "id": "s", "color": "#fff", "orientation": [0, 90, 0],
        "along": {"path": {"points": [[0, 0], [0, 100]]}, "t": 0.5, "orient": true}}]}), 0.0);
    let w = rot_block(&f.layers[0].world);
    same(&w, &Mat4::rot_z(90.0).mul(&Mat4::rot_y(90.0)));
}

#[test]
fn a_3d_layer_on_a_path_aims_its_depth_axis_along_the_travel() {
    // source: py-aep (a 3D layer aims its Z axis along the motion through a look-at)
    let f = frame(json!({"dur": 1, "layers": [{"type": "solid", "id": "s", "color": "#fff", "three_d": true,
        "along": {"path": {"points": [[0, 0], [0, 100]]}, "t": 0.5, "orient": true}}]}), 0.0);
    let d = depth_axis(&f.layers[0]);
    assert!(d[0].abs() < 1e-9 && (d[1] - 1.0).abs() < 1e-9 && d[2].abs() < 1e-9, "{d:?}");
}

#[test]
fn an_aim_and_a_path_heading_cannot_both_steer_a_layer() {
    let plan = Plan::from_value(json!({"dur": 1, "layers": [{"type": "solid", "id": "s", "color": "#fff", "three_d": true, "aim": [1, 0, 0],
        "along": {"path": {"points": [[0, 0], [0, 100]]}, "t": 0.5, "orient": true}}]}), Path::new("."));
    let e = format!("{:#}", plan.err().expect("rejected at plan time"));
    assert!(e.contains("aim") && e.contains("along.orient"), "{e}");
}

#[test]
fn an_aim_is_a_point_in_the_world_even_for_a_child() {
    // derived from After Effects' auto-orient toward the point of interest: the parent is turned a quarter and
    // moved, and the child still faces the same world point
    let f = frame(json!({"dur": 1, "layers": [
        {"type": "solid", "id": "p", "color": "#fff", "three_d": true, "anchor": [0, 0, 0], "pos": [500, 0, 0], "rotation": [0, 0, 90]},
        {"type": "solid", "id": "c", "color": "#fff", "three_d": true, "parent": "p", "pos": [0, 0, 0], "aim": [1500, 0, 0]}
    ]}), 0.0);
    // the readable side is local -z, so the depth axis points away from the aimed point: world -x
    let d = depth_axis(&f.layers[1]);
    assert!((d[0] + 1.0).abs() < 1e-9 && d[1].abs() < 1e-9 && d[2].abs() < 1e-9, "{d:?}");
}

#[test]
fn the_default_camera_is_after_effects_50mm_camera_and_still_draws_the_z0_plane_1_to_1() {
    // source: py-aep, measured in After Effects 2026: zoom = width / 0.72, so 2666.67 px for a 1920 wide comp
    let f = frame(json!({"width": 1920, "height": 1080, "dur": 1, "layers": [{"type": "camera", "id": "cam"}]}), 0.0);
    let cam = f.camera.expect("the camera");
    assert!((cam.zoom - 2666.666_666_666_7).abs() < 1e-6, "{}", cam.zoom);
    assert!((cam.pos[2] + cam.zoom).abs() < 1e-9, "centred at (w/2, h/2, -zoom): {:?}", cam.pos);
    let p = cam.view_proj.project(100.0, 200.0, 0.0).expect("in front of the camera");
    assert!((p[0] - 100.0).abs() < 1e-6 && (p[1] - 200.0).abs() < 1e-6, "{p:?}");
}
