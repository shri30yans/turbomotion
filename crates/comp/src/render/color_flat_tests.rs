//! Every way a flat dark colour reaches a picture in linear light, one film each: the level the colour shows must be the
//! level it shows when pictures hold encoded numbers, which store a display level exactly. A source of flat colour that
//! holds it in fewer bits than the light needs shows a coarser level in the darks and fails here.

use serde_json::{json, Value};

use super::testkit::render;

const SIDE: usize = 96;
/// The darks that collapsed: 8 and 16 showed the same level, and 48 a different one.
const DARKS: [usize; 3] = [8, 16, 48];

fn hex(level: usize) -> String {
    format!("#{level:02x}{level:02x}{level:02x}")
}

fn film(color: Value, bg: &str, layers: Vec<Value>) -> Value {
    let mut film = json!({"width": SIDE, "height": SIDE, "fps": 10, "dur": 1, "bg": bg, "layers": layers});
    if !color.is_null() {
        film["color"] = color;
    }
    film
}

/// The brightest red value anywhere in the frame: where a shape, a glyph or a mesh is solid, its own colour.
fn brightest(px: &[u8]) -> usize {
    px.chunks_exact(4).map(|p| usize::from(p[2])).max().unwrap_or(0)
}

/// `layers` over `bg` in light and on encoded numbers: the two brightest levels.
fn both(bg: &str, layers: &[Value]) -> (usize, usize) {
    let in_light = brightest(&render(film(json!({"linear_light": true}), bg, layers.to_vec())));
    let encoded = brightest(&render(film(Value::Null, bg, layers.to_vec())));
    (in_light, encoded)
}

/// Whether `make(level)` shows the level on encoded numbers (as it must) and in light, for each of the darks.
fn assert_the_same_in_light(what: &str, make: impl Fn(usize) -> (String, Vec<Value>)) {
    for level in DARKS {
        let (bg, layers) = make(level);
        let (in_light, encoded) = both(&bg, &layers);
        assert!(encoded.abs_diff(level) <= 1, "{what}: on encoded numbers level {level} shows {encoded}");
        assert!(in_light.abs_diff(encoded) <= 1, "{what}: level {level} shows {in_light} in light and {encoded} on encoded numbers");
    }
}

#[test]
fn the_stroke_of_a_shape() {
    assert_the_same_in_light("stroke", |l| ("#000000".into(), vec![json!({"type": "shape", "items": [{"rect": [30, 30, 36, 36], "stroke": {"color": hex(l), "width": 8}}]})]));
}

#[test]
fn the_letters_of_a_text_layer() {
    assert_the_same_in_light("text", |l| ("#000000".into(), vec![json!({"type": "text", "text": "HHH", "size": 70, "weight": 900, "color": hex(l)})]));
}

#[test]
fn the_sprites_of_a_particle_layer() {
    assert_the_same_in_light("particle", |l| {
        let layer = json!({"type": "particles", "pos": [48, 48], "rate": 0, "burst": [[0.0, 1]], "life": [2, 2], "speed": [0, 0], "size": [60, 60], "spin": [0, 0], "colors": [[0, hex(l)]], "shape": "circle"});
        ("#000000".into(), vec![layer])
    });
}

#[test]
fn the_background_of_the_composition() {
    assert_the_same_in_light("background", |l| (hex(l), vec![]));
}

#[test]
fn the_faces_of_a_mesh() {
    assert_the_same_in_light("mesh", |l| {
        let camera = json!({"type": "camera", "zoom": 200});
        let light = json!({"type": "light", "kind": "ambient", "intensity": 1.0});
        let mesh = json!({"type": "mesh", "three_d": true, "geometry": {"kind": "box", "size": 40}, "color": hex(l), "side": hex(l), "pos": [48, 48, 0]});
        ("#000000".into(), vec![camera, light, mesh])
    });
}
