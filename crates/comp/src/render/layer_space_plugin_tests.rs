//! What a shader plugin sees in each effect space, and what `@bounds` changes: the step-1 reproduction of the clipping
//! symptom as tests (the numbers are written up in `parity/specs/effect_space.md`).

use std::path::PathBuf;

use serde_json::{json, Value};

use super::testkit::{bgra, render_at};

const W: usize = 200;
const H: usize = 100;

fn plugins(tag: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tm-layerspace-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("effects")).expect("a plugin folder");
    for (name, src) in files {
        std::fs::write(dir.join("effects").join(name), src).expect("a plugin file");
    }
    dir
}

fn film(space: &str, fx: Value) -> Value {
    json!({"width": W, "height": H, "fps": 10, "dur": 1, "bg": "#000000", "effect_space": space, "layers": [
        {"type": "solid", "color": "#ffffff", "size": [20, 20], "anchor": [10, 10], "pos": [100, 50], "fx": [fx]}
    ]})
}

/// Reports what the shader is told, one value per channel, in levels of 255.
const REPORT: &str = "// @effect report\n// @summary Colours a pixel by the geometry the shader is given.\n// @bounds expand=10\nhalf4 main(float2 xy) { return half4(resolution.x / 255.0, buffer_size.x / 255.0, -layer_origin.x / 255.0, 1.0); }\n";

fn report(space: &str) -> [u8; 3] {
    let dir = plugins(&format!("report-{space}"), &[("report.sksl", REPORT)]);
    let px = render_at(film(space, json!({"effect": "report"})), &dir, 0.0);
    std::fs::remove_dir_all(dir).ok();
    let c = bgra(&px, W, 100, 50);
    [c[2], c[1], c[0]]
}

#[test]
fn in_screen_space_a_plugin_sees_the_whole_frame_as_its_resolution_and_its_picture() {
    assert_eq!(report("screen"), [200, 200, 0], "resolution = the frame (200 px), buffer = the frame, origin 0");
}

#[test]
fn in_layer_space_a_plugin_sees_the_layer_as_its_resolution_and_a_picture_grown_by_its_bounds() {
    let [resolution, buffer, origin] = report("layer");
    assert_eq!((resolution, buffer, origin), (20, 40, 10), "resolution = the 20 px layer, buffer = 20 + 2 x 10 of `@bounds expand=10`, its top-left 10 px up and left of the layer");
}

/// Moves the picture 30 px right.
fn shift(bounds: &str) -> String {
    format!("// @effect shift\n// @summary Moves the picture right.\n{bounds}half4 main(float2 xy) {{ return image.eval(xy - float2(30, 0)); }}\n")
}

fn lit_columns(space: &str, bounds: &str) -> (usize, usize) {
    let dir = plugins(&format!("shift-{space}-{}", bounds.len()), &[("shift.sksl", &shift(bounds))]);
    let px = render_at(film(space, json!({"effect": "shift"})), &dir, 0.0);
    std::fs::remove_dir_all(dir).ok();
    let lit: Vec<usize> = (0..W).filter(|&x| bgra(&px, W, x, 50)[2] > 128).collect();
    (lit.len(), lit.first().copied().unwrap_or(0))
}

#[test]
fn a_plugin_that_moves_pixels_outside_the_layer_is_clipped_in_layer_space_unless_it_declares_bounds() {
    assert_eq!(lit_columns("screen", ""), (20, 120), "screen space: the frame-sized picture has room, the square moves 30 px right to x = 120..140");
    assert_eq!(lit_columns("layer", ""), (20, 90), "layer space, no bounds: the picture is only the layer's 20 px and reads past its edge clamp to the edge, so the shift cannot show: the square stays at x = 90..110");
    assert_eq!(lit_columns("layer", "// @bounds expand_x=30\n"), (20, 120), "with `@bounds expand_x=30` it lands where the screen pipeline puts it");
}
