//! The picture a layer with `motion_tile` is drawn into in layer space: the layer grown by what the output size asks for
//! (`@bounds expand_x=box_w*(output_width-100)/200 ...`), read from a real evaluated layer.

use std::path::Path;

use serde_json::{json, Value};

use crate::eval::{buffer::LocalBuffer, Evaluator};
use crate::plan::{Plan, ROOT};

/// The picture of a 32 x 16 solid in a 1920 x 1080 frame with `fx`, at `t`.
fn picture(fx: Value, t: f64) -> LocalBuffer {
    let layer = json!({"type": "solid", "color": "#fff", "size": [32, 16], "pos": [960, 540], "fx": fx});
    let film = json!({"width": 1920, "height": 1080, "dur": 2, "effect_space": "layer", "layers": [layer]});
    let plan = Plan::from_value(film, Path::new(".")).expect("the plan");
    Evaluator::new(&plan).frame(ROOT, t).expect("the frame").layers[0].local.expect("a layer-space picture")
}

fn tile(params: Value) -> Value {
    let mut fx = json!({"effect": "motion_tile"});
    if let (Some(base), Some(more)) = (fx.as_object_mut(), params.as_object()) {
        base.extend(more.clone());
    }
    json!([fx])
}

#[test]
fn the_picture_grows_by_half_the_extra_output_on_each_side() {
    let b = picture(tile(json!({"output_width": 300, "output_height": 300})), 0.0);
    assert_eq!((b.origin, b.size), ([-32, -16], [96, 48]), "300 % of 32 x 16 is 96 x 48: 32 px left and right, 16 up and down");
    let b = picture(tile(json!({"output_width": 300})), 0.0);
    assert_eq!((b.origin, b.size), ([-32, 0], [96, 16]), "only the width asked for more");
    assert_eq!(b.bounds_post_fx, [-32.0, 0.0, 64.0, 16.0]);
}

#[test]
fn an_output_that_is_smaller_than_the_layer_does_not_shrink_the_picture() {
    let b = picture(tile(json!({"output_width": 50, "output_height": 50})), 0.0);
    assert_eq!((b.origin, b.size), ([0, 0], [32, 16]), "the crop is made inside the layer's own picture");
}

#[test]
fn the_default_output_adds_nothing() {
    assert_eq!(picture(tile(json!({})), 0.0).size, [32, 16]);
}

#[test]
fn a_fraction_of_a_pixel_is_rounded_outwards() {
    let b = picture(tile(json!({"output_width": 150})), 0.0);
    assert_eq!((b.origin[0], b.size[0]), (-8, 48), "150 % of 32 is 48: 8 px a side");
    let b = picture(tile(json!({"output_height": 117})), 0.0);
    assert_eq!((b.origin[1], b.size[1]), (-2, 20), "117 % of 16 is 18.72: 1.36 px a side, rounded out to 2");
}

#[test]
fn the_growth_follows_an_animated_output_size() {
    let fx = tile(json!({"output_width": [[0, 100], [1, 300, "linear"]]}));
    assert_eq!(picture(fx.clone(), 0.0).size[0], 32);
    assert_eq!(picture(fx.clone(), 0.5).size[0], 64, "200 % at the half");
    assert_eq!(picture(fx, 1.0).size[0], 96);
}

#[test]
fn an_output_past_its_range_is_held_at_one_thousand_percent() {
    assert_eq!(picture(tile(json!({"output_width": 5000})), 0.0).size[0], 320, "1000 % of 32");
}

#[test]
fn it_adds_to_the_effects_around_it() {
    // a blur of radius 2 (3 sigma = 6 px) and then a 300 % tile: the tile grows the layer's own box by 32 and 16, the blur
    // by 6 on every side, and the buffer holds both
    let fx = json!([{"effect": "blur", "radius": 2}, {"effect": "motion_tile", "output_width": 300, "output_height": 300}]);
    let b = picture(fx, 0.0);
    assert_eq!(b.size, [32 + 2 * (6 + 32), 16 + 2 * (6 + 16)]);
}

#[test]
fn an_output_that_would_make_the_picture_too_big_is_an_error_that_names_the_layer() {
    // 1000 % of a 1920 px wide layer is 19200 px: past the 16384 px a layer's picture may have
    let layer = json!({"type": "solid", "id": "wall", "color": "#fff", "size": [1920, 100], "pos": [960, 540], "fx": tile(json!({"output_width": 1000}))});
    let film = json!({"width": 1920, "height": 1080, "dur": 1, "effect_space": "layer", "layers": [layer]});
    let plan = Plan::from_value(film, Path::new(".")).expect("the plan");
    let e = Evaluator::new(&plan).frame(ROOT, 0.0).err().map(|e| format!("{e:#}")).unwrap_or_default();
    assert!(e.contains("wall") && e.contains("19200x100"), "{e}");
}
