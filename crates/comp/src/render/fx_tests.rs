//! Effects that read other pictures: layer inputs (displacement, turbulence, mattes), multi-pass plugins and the plan-time
//! checks that keep them well formed. Each rendering test asserts where known pixels land.

use super::testkit::{render, render_at};
use crate::plan::Plan;
use serde_json::{json, Value};
use std::path::Path;

const W: usize = 64;
const H: usize = 32;

fn comp(layers: Vec<Value>) -> Value {
    json!({"width": W, "height": H, "dur": 1, "layers": layers})
}

/// How bright a pixel is (the frame is black, so the blue channel of a white shape is its coverage).
fn level(px: &[u8], x: usize, y: usize) -> u8 {
    px[(y * W + x) * 4]
}

/// The columns, and the rows, where any pixel is bright.
fn opaque_extent(px: &[u8]) -> ([usize; 2], [usize; 2]) {
    let hit = |x: usize, y: usize| level(px, x, y) > 128;
    let cols: Vec<usize> = (0..W).filter(|&x| (0..H).any(|y| hit(x, y))).collect();
    let rows: Vec<usize> = (0..H).filter(|&y| (0..W).any(|x| hit(x, y))).collect();
    ([cols[0], cols[cols.len() - 1] + 1], [rows[0], rows[rows.len() - 1] + 1])
}

fn square(x: f64, fx: Value) -> Value {
    json!({"type": "solid", "color": "#ffffff", "size": [4, 4], "pos": [x, 16], "fx": [fx]})
}

fn map(color: &str) -> Value {
    json!({"type": "solid", "id": "map", "color": color, "hidden": true})
}

#[test]
fn a_channel_of_the_map_moves_a_square_by_the_stated_pixels() {
    let still = render(comp(vec![square(20.0, json!({"effect": "displacement_map", "map": "map", "horizontal": "off", "vertical": "off"})), map("#ff0000")]));
    assert_eq!(opaque_extent(&still), ([18, 22], [14, 18]), "with both channels off nothing moves");
    let moved = render(comp(vec![square(20.0, json!({"effect": "displacement_map", "map": "map", "horizontal": "red", "vertical": "green", "max_x": 10, "max_y": 6})), map("#ff0000")]));
    assert_eq!(opaque_extent(&moved), ([28, 32], [8, 12]), "red 1 moves right 10; green 0 moves up 6");
    let other = render(comp(vec![square(20.0, json!({"effect": "displacement_map", "map": "map", "horizontal": "red", "vertical": "green", "max_x": 10, "max_y": 6})), map("#00ff00")]));
    assert_eq!(opaque_extent(&other), ([8, 12], [20, 24]), "red 0 moves left 10; green 1 moves down 6");
}

#[test]
fn outside_the_frame_either_pins_the_picture_or_wraps_it() {
    let half = |edge: &str| {
        let fx = json!({"effect": "displacement_map", "map": "map", "horizontal": "red", "vertical": "off", "max_x": 10, "outside": edge});
        render(comp(vec![json!({"type": "solid", "color": "#ffffff", "size": [32, 32], "pos": [16, 16], "fx": [fx]}), map("#ff0000")]))
    };
    let (pinned, wrapped) = (half("pin"), half("wrap"));
    assert!(level(&pinned, 2, 16) > 250, "pin: the edge pixel is stretched into the gap the move opens");
    assert!(level(&wrapped, 2, 16) < 5, "wrap: the gap shows the far side of the picture, which is empty");
    assert!(level(&wrapped, 42, 16) < 5 && level(&pinned, 42, 16) < 5, "the right edge of the half moved to x=42");
    assert!(level(&pinned, 41, 16) > 250, "the half now ends at x=42");
}

#[test]
fn a_square_pushed_past_the_frame_returns_on_the_other_side_when_wrapping() {
    let fx = json!({"effect": "displacement_map", "map": "map", "horizontal": "red", "vertical": "off", "max_x": 40, "outside": "wrap"});
    let px = render(comp(vec![square(60.0, fx), map("#ff0000")]));
    assert_eq!(opaque_extent(&px).0, [34, 38], "58..62 plus 40 is 98..102, which is 34..38 in a 64 px frame");
}

/// The red channel of a picture at a fractional pixel position, interpolated between pixel centres like the GPU does.
fn red_at(px: &[u8], x: f64, y: f64) -> f64 {
    let at = |x: usize, y: usize| f64::from(px[(y.min(H - 1) * W + x.min(W - 1)) * 4 + 2]);
    let (x0, y0, fx, fy) = (x.floor() as usize, y.floor() as usize, x.fract(), y.fract());
    let top = at(x0, y0) * (1.0 - fx) + at(x0 + 1, y0) * fx;
    let bottom = at(x0, y0 + 1) * (1.0 - fx) + at(x0 + 1, y0 + 1) * fx;
    top * (1.0 - fy) + bottom * fy
}

fn luma_at(px: &[u8], x: usize, y: usize) -> f64 {
    let i = (y * W + x) * 4;
    (0.2126 * f64::from(px[i + 2]) + 0.7152 * f64::from(px[i + 1]) + 0.0722 * f64::from(px[i])) / 255.0
}

#[test]
fn noise_from_a_generator_layer_displaces_by_its_own_brightness() {
    const AMOUNT: f64 = 8.0;
    let noise_params = json!({"scale": 20});
    let ramp = json!({"type": "texture", "shader": "gradient", "params": {"from": "#000000", "to": "#ffffff", "angle": 45}});
    let effect = json!({"effect": "turbulent_displace", "noise": {"layer": "n", "fit": "screen"}, "amount": AMOUNT});
    let mut driven = comp(vec![json!({"type": "precomp", "comp": "src", "fx": [effect]}), json!({"type": "texture", "shader": "noise", "id": "n", "hidden": true, "params": noise_params})]);
    driven["comps"] = json!({"src": {"width": W, "height": H, "dur": 1, "layers": [ramp.clone()]}});
    let drawn = render(driven);
    let field = render(comp(vec![json!({"type": "texture", "shader": "noise", "params": noise_params})]));
    let source = render(comp(vec![ramp]));
    let mut moved = 0;
    for (x, y) in [(20, 8), (30, 16), (40, 20), (25, 25), (50, 10)] {
        // the second axis reads the field at the point mirrored through the frame's centre
        let dx = (luma_at(&field, x, y) - 0.5) * 2.0 * AMOUNT;
        let dy = (luma_at(&field, W - 1 - x, H - 1 - y) - 0.5) * 2.0 * AMOUNT;
        let expected = red_at(&source, x as f64 - dx, y as f64 - dy);
        let got = f64::from(drawn[(y * W + x) * 4 + 2]);
        assert!((got - expected).abs() < 6.0, "({x},{y}): the source at ({dx:.2},{dy:.2}) away is {expected:.1}, the effect drew {got:.1}");
        if dx.abs() + dy.abs() > 1.0 {
            moved += 1;
        }
    }
    assert!(moved >= 2, "the noise should really move several of the probed pixels");
}

#[test]
fn a_screen_fitted_map_stays_where_it_is_on_screen_and_a_centred_one_follows_the_layer() {
    // the map is white left of x=32 and empty right of it; the square sits at x=48, in the empty part
    let spot = json!({"type": "solid", "id": "spot", "color": "#ffffff", "size": [32, 32], "pos": [16, 16], "hidden": true});
    let square = |fit: &str| {
        let map = json!({"layer": "spot", "fit": fit});
        json!({"type": "solid", "color": "#ffffff", "size": [4, 4], "pos": [48, 16], "fx": [{"effect": "displacement_map", "map": map, "horizontal": "luma", "vertical": "off", "max_x": 8}]})
    };
    // an empty map pushes pixels one way (the picture lands 8 px left), a white one the other (8 px right)
    let (cols, _) = opaque_extent(&render(comp(vec![square("screen"), spot.clone()])));
    assert_eq!(cols, [38, 42], "on screen the map is empty at the square, so the square moves left");
    let (cols, _) = opaque_extent(&render(comp(vec![square("center"), spot])));
    assert_eq!(cols, [54, 58], "centred on the square's layer the map is white there, so the square moves right");
}

fn plugin_dir(tag: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("tm-fxtests-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("effects")).expect("a plugin folder");
    for (name, src) in files {
        std::fs::write(dir.join("effects").join(name), src).expect("a plugin file");
    }
    dir
}

const CROSS_BLUR: &str = "// @effect cross_blur\n// @summary Three-tap blur across, then down: two passes.\n// @pass across -- horizontal taps\nhalf4 main(float2 xy) {\n    return (image.eval(xy - float2(1, 0)) + image.eval(xy) + image.eval(xy + float2(1, 0))) / 3.0;\n}\n// @pass down -- vertical taps of the first pass\nhalf4 main(float2 xy) {\n    return (across.eval(xy - float2(0, 1)) + across.eval(xy) + across.eval(xy + float2(0, 1))) / 3.0;\n}\n";

#[test]
fn passes_run_in_order_and_each_sees_the_one_before() {
    let dir = plugin_dir("passes", &[("cross_blur.sksl", CROSS_BLUR)]);
    let dot = json!({"type": "solid", "color": "#ffffff", "size": [1, 1], "pos": [8.5, 8.5], "fx": [{"effect": "cross_blur"}]});
    let px = render_at(comp(vec![dot]), &dir, 0.0);
    for (x, y) in [(7, 7), (8, 8), (9, 9), (7, 9)] {
        assert!((i32::from(level(&px, x, y)) - 28).abs() <= 2, "({x},{y}) should hold a ninth of the dot, 28 of 255, not {}", level(&px, x, y));
    }
    assert_eq!((level(&px, 6, 8), level(&px, 8, 10)), (0, 0), "the blur reaches one pixel each way and no further");
    std::fs::remove_dir_all(dir).ok();
}

const HALF_COPY: &str = "// @effect half_copy\n// @summary Copies the picture through a half-size pass.\n// @pass small scale=0.5\nhalf4 main(float2 xy) { return image.eval(xy); }\n// @pass big\nhalf4 main(float2 xy) { return small.eval(xy); }\n";

#[test]
fn a_reduced_pass_is_sampled_back_at_full_size_in_the_right_place() {
    let dir = plugin_dir("scale", &[("half_copy.sksl", HALF_COPY)]);
    let left = json!({"type": "solid", "color": "#ffffff", "size": [32, 32], "pos": [16, 16], "fx": [{"effect": "half_copy"}]});
    let px = render_at(comp(vec![left]), &dir, 0.0);
    assert!(level(&px, 4, 16) > 250 && level(&px, 28, 16) > 250, "the left half survives the round trip");
    assert!(level(&px, 36, 16) < 5 && level(&px, 60, 16) < 5, "the right half stays empty");
    std::fs::remove_dir_all(dir).ok();
}

fn plan_error(layers: Vec<Value>) -> String {
    format!("{:#}", Plan::from_value(comp(layers), Path::new(".")).err().expect("the plan should be rejected"))
}

fn displaced_by(map: Value) -> Value {
    json!({"type": "solid", "id": "a", "color": "#fff", "fx": [{"effect": "displacement_map", "map": map}]})
}

#[test]
fn a_map_that_cannot_work_is_rejected_when_the_plan_is_made() {
    let e = plan_error(vec![displaced_by(json!("nothing")), map("#f00")]);
    assert!(e.contains("does not exist") && e.contains("a, map"), "{e}");
    let e = plan_error(vec![displaced_by(json!("a"))]);
    assert!(e.contains("reads the layer it is on"), "{e}");
    let e = plan_error(vec![json!({"type": "solid", "color": "#fff", "fx": [{"effect": "displacement_map"}]})]);
    assert!(e.contains("needs \"map\""), "{e}");
    let e = plan_error(vec![displaced_by(json!({"layer": "map", "fit": "tile"})), map("#f00")]);
    assert!(e.contains("fit \"tile\"") && e.contains("\"center\"") && e.contains("\"screen\""), "{e}");
    let e = plan_error(vec![displaced_by(json!("cam")), json!({"type": "camera", "id": "cam"})]);
    assert!(e.contains("has no picture"), "{e}");
}

#[test]
fn layers_that_read_each_other_form_a_loop_error() {
    let a = json!({"type": "solid", "id": "a", "color": "#fff", "fx": [{"effect": "displacement_map", "map": "b"}]});
    let b = json!({"type": "solid", "id": "b", "color": "#fff", "fx": [{"effect": "displacement_map", "map": "a"}]});
    let e = plan_error(vec![a, b]);
    assert!(e.contains("loop"), "{e}");
}
