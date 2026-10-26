//! What reaches a shader plugin and what files bring in, in numbers: a plugin gets the encoding it declares with
//! `@space`, an image keeps the colours its profile gives it, and a film that leaves in Display P3 shows sRGB red where
//! the published matrix puts it.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use skia_safe::{png_encoder, surfaces, AlphaType, Color4f, ColorType, ISize, ImageInfo, Paint, Pixmap};
use turbomotion_core::color::{Primaries, Profile, Transfer};

use super::testkit::{bgra, render_at};

const W: usize = 32;

/// A throwaway project folder with plugin files in `effects/`.
fn project(tag: &str, plugins: &[(&str, String)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tm-color-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("effects")).expect("a plugin folder");
    for (name, src) in plugins {
        std::fs::write(dir.join("effects").join(name), src).expect("a plugin file");
    }
    dir
}

fn film(color: Value, layers: Value) -> Value {
    let mut film = json!({"width": W, "height": W, "fps": 10, "dur": 1, "bg": "#000000", "layers": layers});
    if !color.is_null() {
        film["color"] = color;
    }
    film
}

/// The pixel at the middle of the frame as `[r, g, b]`.
fn rgb(px: &[u8]) -> [u8; 3] {
    let [b, g, r, _] = bgra(px, W, W / 2, W / 2);
    [r, g, b]
}

/// Doubles what it sees: 128 of 255 doubled is 255 on display numbers and the light of two such greys in light.
fn doubled(space: &str) -> String {
    format!("// @effect doubled\n// @summary Doubles the picture.\n// @bounds none\n{space}half4 main(float2 xy) {{ half4 c = image.eval(xy); return half4(c.rgb * 2.0, c.a); }}\n")
}

/// Paints its colour parameter as it is given.
const PAINT: &str = "// @effect paint\n// @summary Paints a colour.\n// @bounds none\n// @param tone color #808080 -- the colour\n// @space linear\nhalf4 main(float2 xy) { return half4(half3(tone.rgb), 1.0); }\n";

fn run(tag: &str, color: Value, plugin: &str, fx: Value, layer: Value) -> [u8; 3] {
    let dir = project(tag, &[("plugin.sksl", plugin.to_string())]);
    let mut layer = layer;
    layer["fx"] = json!([fx]);
    let px = render_at(film(color, json!([layer])), &dir, 0.0);
    std::fs::remove_dir_all(dir).ok();
    rgb(&px)
}

fn grey() -> Value {
    json!({"type": "solid", "color": "#808080"})
}

#[test]
fn a_plugin_receives_the_encoding_it_declares() {
    let linear = json!({"linear_light": true});
    let fx = json!({"effect": "doubled"});
    let seen = |tag, color: Value, space: &str| run(tag, color, &doubled(space), fx.clone(), grey())[0];
    assert_eq!(seen("d-off", Value::Null, ""), 255, "display numbers doubled clip, with or without the key");
    assert_eq!(seen("l-off", Value::Null, "// @space linear\n"), 255, "without linear light a plugin that asks for light still sees what the film holds");
    assert_eq!(seen("d-on", linear.clone(), "// @space display\n"), 255, "a display plugin in a linear film sees display numbers");
    let in_light = seen("l-on", linear, "// @space linear\n");
    assert!((i32::from(in_light) - 176).abs() <= 1, "a light plugin sees 0.2159 and doubles it to 0.4317, which shows as 175.6: {in_light}");
}

#[test]
fn a_colour_parameter_arrives_in_the_encoding_the_plugin_declares() {
    let linear = json!({"linear_light": true});
    let got = run("tone", linear, PAINT, json!({"effect": "paint", "tone": "#808080"}), json!({"type": "solid", "color": "#000000"}));
    assert!(got.iter().all(|c| (i32::from(*c) - 128).abs() <= 1), "#808080 reaches the shader as light and is shown as #808080 again: {got:?}");
}

/// The level of `light` shown on a display, and the P3 coordinates of sRGB red from the sRGB to P3 matrix (CSS Color 4).
fn shown(light: f64) -> f64 {
    255.0 * if light <= 0.003_130_8 { light * 12.92 } else { 1.055 * light.powf(1.0 / 2.4) - 0.055 }
}

#[test]
fn a_film_that_leaves_in_display_p3_shows_srgb_red_where_the_matrix_puts_it() {
    let want = [0.822_462, 0.033_194, 0.017_082].map(shown);
    for color in [json!({"space": "display_p3"}), json!({"space": "display_p3", "linear_light": true})] {
        let px = render_at(film(color.clone(), json!([{"type": "solid", "color": "#ff0000"}])), Path::new("."), 0.0);
        let got = rgb(&px);
        for ((g, w), c) in got.iter().zip(want).zip("rgb".chars()) {
            assert!((f64::from(*g) - w).abs() <= 1.5, "{color}: {c} {g} vs {w:.1}");
        }
    }
}

/// A one-colour PNG whose embedded profile is `profile`.
fn png_in(dir: &Path, name: &str, profile: Profile) {
    let info = ImageInfo::new(ISize::new(4, 4), ColorType::RGBA8888, AlphaType::Premul, profile.space());
    let mut surface = surfaces::raster(&info, None, None).expect("a surface");
    // the channel values (1, 0, 0) of the profile's own space: its red primary
    surface.canvas().draw_paint(&Paint::new(Color4f::new(1.0, 0.0, 0.0, 1.0), &profile.space()));
    let mut bytes = vec![0u8; info.compute_min_byte_size()];
    assert!(surface.read_pixels(&info, &mut bytes, info.min_row_bytes(), (0, 0)));
    let mut out = Vec::new();
    assert!(png_encoder::encode(&Pixmap::new(&info, &mut bytes, info.min_row_bytes()).expect("pixels"), &mut out, &png_encoder::Options::default()));
    std::fs::write(dir.join(name), out).expect("a png");
}

#[test]
fn an_image_keeps_the_colours_its_profile_gives_it_in_a_managed_film_and_is_drawn_as_it_is_otherwise() {
    let dir = project("images", &[]);
    let p3 = Profile { primaries: Primaries::DisplayP3, transfer: Transfer::Srgb };
    png_in(&dir, "p3.png", p3);
    png_in(&dir, "srgb.png", Profile::SRGB);
    let image = |src: &str| json!([{"type": "image", "src": src, "pos": [16, 16], "size": [32, 32]}]);
    let at = |color: Value, src: &str| rgb(&render_at(film(color, image(src)), &dir, 0.0));
    let managed = json!({"working_profile": "display_p3", "space": "display_p3"});
    assert_eq!(at(managed.clone(), "p3.png"), [255, 0, 0], "P3 red in a P3 film is P3 red");
    let want = [0.822_462, 0.033_194, 0.017_082].map(shown);
    let got = at(managed, "srgb.png");
    for (g, w) in got.iter().zip(want) {
        assert!((f64::from(*g) - w).abs() <= 1.5, "sRGB red in a P3 film: {got:?} vs {want:?}");
    }
    assert_eq!(at(Value::Null, "p3.png"), [255, 0, 0], "without the key the numbers are drawn as they are");
    assert_eq!(at(Value::Null, "srgb.png"), [255, 0, 0]);
    std::fs::remove_dir_all(dir).ok();
}
