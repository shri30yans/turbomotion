//! The size of a layer-space layer's picture, read from a real evaluated frame: its own size grown by its effects, never
//! the size of the frame, whatever the layer's scale.

use std::path::Path;

use serde_json::{json, Value};

use crate::eval::{buffer::LocalBuffer, Evaluator};
use crate::plan::{Plan, ROOT};

const FRAME: [u32; 2] = [1920, 1080];

fn local_of(space: Option<&str>, layer: Value) -> Option<LocalBuffer> {
    let mut film = json!({"width": FRAME[0], "height": FRAME[1], "dur": 1, "layers": [layer]});
    if let Some(s) = space {
        film["effect_space"] = json!(s);
    }
    let plan = Plan::from_value(film, Path::new(".")).expect("the plan");
    Evaluator::new(&plan).frame(ROOT, 0.0).expect("the frame").layers[0].local
}

fn solid(size: [f64; 2], scale: f64, fx: Value) -> Value {
    json!({"type": "solid", "color": "#fff", "size": size, "pos": [960, 540], "scale": scale, "fx": fx})
}

#[test]
fn the_picture_is_the_layer_grown_by_its_blur_and_not_the_frame() {
    // a blur of radius 10 reaches 3 sigma = 30 px on every side
    let b = local_of(Some("layer"), solid([40.0, 40.0], 1.0, json!([{"effect": "blur", "radius": 10}]))).expect("a layer-space picture");
    assert_eq!((b.origin, b.size), ([-30, -30], [100, 100]), "40 + 2 x 30");
    assert_eq!(b.bytes(), 100 * 100 * 8);
    assert!(b.bytes() * 100 < u64::from(FRAME[0]) * u64::from(FRAME[1]) * 8, "under a hundredth of a frame-sized picture");
}

#[test]
fn the_layer_scale_does_not_change_the_picture_it_is_drawn_into() {
    let fx = json!([{"effect": "blur", "radius": 10}]);
    let one = local_of(Some("layer"), solid([40.0, 40.0], 1.0, fx.clone())).unwrap();
    let four = local_of(Some("layer"), solid([40.0, 40.0], 4.0, fx)).unwrap();
    assert_eq!(one.size, four.size, "effects run on the layer's own pixels, so scale 4 draws into the same 100x100");
    assert_eq!(four.bounds_screen.map(|b| (b[2] - b[0]).round()), Some(400.0), "and lands 4x as wide on screen (100 x 4)");
}

#[test]
fn effects_add_up_along_the_chain_and_each_grows_the_box_the_one_before_left() {
    let b = local_of(Some("layer"), solid([40.0, 40.0], 1.0, json!([{"effect": "blur", "radius": 10}, {"effect": "glow", "radius": 5, "strength": 1}]))).unwrap();
    assert_eq!(b.size, [40 + 2 * (30 + 15), 40 + 2 * (30 + 15)], "30 px from the blur and 15 from the glow on each side");
}

#[test]
fn a_directional_blur_grows_the_picture_along_its_angle_only() {
    let b = local_of(Some("layer"), solid([40.0, 40.0], 1.0, json!([{"effect": "directional_blur", "distance": 40, "angle": 0}]))).unwrap();
    assert_eq!(b.size, [80, 40], "half the smear (20 px) each way along x, nothing along y");
}

#[test]
fn content_larger_than_the_frame_keeps_its_whole_picture() {
    let b = local_of(Some("layer"), solid([4000.0, 100.0], 1.0, json!([{"effect": "blur", "radius": 2}]))).unwrap();
    assert_eq!(b.size, [4012, 112], "4000 px wide in a 1920 px frame: nothing is cut to the frame");
}

#[test]
fn a_layer_without_effects_or_masks_and_every_screen_space_layer_has_no_picture() {
    assert!(local_of(Some("layer"), solid([40.0, 40.0], 1.0, json!([]))).is_none(), "nothing to run before the transform");
    assert!(local_of(None, solid([40.0, 40.0], 1.0, json!([{"effect": "blur", "radius": 10}]))).is_none(), "the default is the screen pipeline");
    assert!(local_of(Some("screen"), solid([40.0, 40.0], 1.0, json!([{"effect": "blur", "radius": 10}]))).is_none());
}

#[test]
fn inspect_shows_the_three_boxes_of_a_layer_space_layer_and_nothing_for_a_screen_layer() {
    let state = |space: &str| {
        let film = json!({"width": 240, "height": 160, "dur": 1, "effect_space": space, "layers": [solid([4.0, 4.0], 2.0, json!([{"effect": "blur", "radius": 10}]))]});
        let plan = Plan::from_value(film, Path::new(".")).expect("the plan");
        serde_json::to_value(&Evaluator::new(&plan).frame(ROOT, 0.0).expect("the frame").layers[0]).expect("serialises")
    };
    let layer = state("layer");
    assert_eq!(layer["local"]["bounds_local"], json!([0.0, 0.0, 4.0, 4.0]));
    assert_eq!(layer["local"]["bounds_post_fx"], json!([-30.0, -30.0, 34.0, 34.0]), "3 sigma = 30 px each side");
    assert_eq!(layer["local"]["size"], json!([64, 64]));
    assert!(layer["local"]["bounds_screen"].is_array(), "{}", layer["local"]);
    assert!(state("screen").get("local").is_none(), "an inspect of a screen-space film is unchanged");
}
