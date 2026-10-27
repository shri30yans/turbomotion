//! Planning the blur family: how far each reaches (what a layer is grown by), and what it refuses before a frame is drawn.

use serde_json::{json, Map, Value};

use super::Effect;
use crate::kernel::{taps, Blur, Edge};

const BOX: [f64; 4] = [0.0, 0.0, 400.0, 300.0];

fn params(v: &Value) -> Map<String, Value> {
    let mut o = v.as_object().expect("an object").clone();
    o.remove("effect");
    o
}

fn build(v: Value) -> anyhow::Result<Effect> {
    Effect::build(v["effect"].as_str().expect("a name"), &params(&v))
}

fn grows(v: Value) -> [f64; 4] {
    build(v.clone()).unwrap_or_else(|e| panic!("{v}: {e:#}")).grows(0.0, BOX)
}

#[test]
fn an_exact_blur_grows_the_layer_by_the_pixels_its_kernel_reads_and_a_legacy_one_by_three_sigma() {
    let reach = Blur::gaussian([10.0; 2], Edge::Transparent).reach();
    assert_eq!(reach, [f64::from(taps::gaussian_taps(10.0)); 2], "the kernel's own statement of how far it reads");
    assert_eq!(grows(json!({"effect": "blur", "radius": 10, "quality": "exact"})), [40.0; 4], "4 sigma, from the one constant the shader loops share");
    assert_eq!(grows(json!({"effect": "blur", "radius": 10})), [30.0; 4], "a film made before the exact blur grows as it did");
    assert_eq!(grows(json!({"effect": "blur", "radius": 10, "quality": "exact", "dimensions": "horizontal"})), [40.0, 0.0, 40.0, 0.0]);
    assert_eq!(grows(json!({"effect": "blur", "radius": 10, "quality": "exact", "dimensions": "vertical"})), [0.0, 40.0, 0.0, 40.0]);
    for quality in ["legacy", "exact"] {
        assert_eq!(grows(json!({"effect": "blur", "radius": 10, "quality": quality, "edge": "repeat"})), [0.0; 4], "a repeated edge adds nothing past the picture ({quality})");
    }
}

#[test]
fn the_glows_and_the_shadow_reach_as_far_as_the_blur_they_are_made_of() {
    assert_eq!(grows(json!({"effect": "glow", "radius": 8, "quality": "exact"})), [32.0; 4]);
    assert_eq!(grows(json!({"effect": "film_halation", "radius": 8, "quality": "exact"})), [32.0; 4]);
    assert_eq!(grows(json!({"effect": "shadow", "dx": 10, "dy": 0, "blur": 5, "quality": "exact"})), [10.0, 20.0, 30.0, 20.0], "4 sigma, further on the side it falls toward");
    assert_eq!(grows(json!({"effect": "shadow", "dx": 10, "dy": 0, "blur": 5})), [5.0, 15.0, 25.0, 15.0], "legacy: 3 sigma");
}

#[test]
fn a_box_and_a_lens_reach_the_pixels_they_read() {
    assert_eq!(grows(json!({"effect": "box_blur", "radius": 5.5, "iterations": 3})), [18.0; 4], "six pixels each way, three times over");
    assert_eq!(grows(json!({"effect": "box_blur", "radius": 5, "iterations": 2, "dimensions": "vertical"})), [0.0, 10.0, 0.0, 10.0]);
    assert_eq!(grows(json!({"effect": "lens_blur", "radius": 10.6, "blades": 6})), [12.0; 4], "the circle through the polygon's corners, and half a pixel more");
    assert_eq!(grows(json!({"effect": "lens_blur", "radius": 10, "edge": "repeat"})), [0.0; 4]);
}

#[test]
fn a_smear_reaches_half_its_path_each_way_and_the_pixel_its_bilinear_read_adds() {
    assert_eq!(grows(json!({"effect": "directional_blur", "distance": 40, "angle": 0, "quality": "exact"})), [21.0, 1.0, 21.0, 1.0]);
    assert_eq!(grows(json!({"effect": "directional_blur", "distance": 40, "angle": 0})), [20.0, 0.0, 20.0, 0.0], "legacy");
    assert_eq!(grows(json!({"effect": "directional_blur", "distance": 0, "quality": "exact"})), [0.0; 4], "a smear of no length reaches nowhere");
    let zoom = grows(json!({"effect": "radial_blur", "amount": 0.2, "x": 0.0, "y": 0.5, "quality": "exact"}));
    assert_eq!((zoom[0], zoom[2]), (0.0, 400.0 * 0.2 + 1.0), "the side on the centre stays, the far side moves a fifth of its distance: {zoom:?}");
    let spin = grows(json!({"effect": "radial_blur", "amount": 30, "x": 0.5, "y": 0.5, "mode": "spin", "quality": "exact"}));
    let far = 200f64.hypot(150.0);
    assert!((spin[0] - (far - 200.0 + 1.0)).abs() < 1e-9 && (spin[1] - (far - 150.0 + 1.0)).abs() < 1e-9, "a spin carries a corner round its circle, and the layer is grown to hold the circle: {spin:?}");
    assert_eq!(grows(json!({"effect": "radial_blur", "amount": 0, "mode": "spin", "quality": "exact"})), [0.0; 4]);
}

#[test]
fn a_blur_the_pass_cannot_run_is_refused_when_it_is_built_and_says_what_the_limit_is() {
    let e = format!("{:#}", build(json!({"effect": "blur", "radius": 600, "quality": "exact"})).unwrap_err());
    assert!(e.contains("2048") && e.contains("sigma 512"), "it names the number of taps and the sigma they allow: {e}");
    assert!(build(json!({"effect": "blur", "radius": 600})).is_ok(), "the legacy blur has no such limit and is not asked for one");
    let e = format!("{:#}", build(json!({"effect": "glow", "radius": [[0, 2], [1, 900]], "quality": "exact"})).unwrap_err());
    assert!(e.contains("taps each side"), "an animated radius is checked at its peak: {e}");
    let e = format!("{:#}", build(json!({"effect": "box_blur", "radius": 3000})).unwrap_err());
    assert!(e.contains("box blur") && e.contains("taps"), "{e}");
    let e = format!("{:#}", build(json!({"effect": "lens_blur", "blades": 2})).unwrap_err());
    assert!(e.contains("0 blades (a disc) or 3 to 16, not 2"), "{e}");
}

#[test]
fn a_spin_without_the_exact_blur_is_an_error_not_a_zoom() {
    let e = format!("{:#}", build(json!({"effect": "radial_blur", "mode": "spin"})).unwrap_err());
    assert!(e.contains("needs `quality: exact`"), "{e}");
    assert!(build(json!({"effect": "radial_blur", "mode": "spin", "quality": "exact"})).is_ok());
}

#[test]
fn a_misspelt_choice_says_what_was_meant_and_lists_the_options() {
    let e = format!("{:#}", build(json!({"effect": "blur", "quality": "exakt", "edge": "repat", "dimensions": "horizontl"})).unwrap_err());
    for want in ["3 problems", "\"exakt\" is not an option. Did you mean \"exact\"", "legacy, exact", "\"repat\" is not an option. Did you mean \"repeat\"", "\"horizontl\" is not an option. Did you mean \"horizontal\""] {
        assert!(e.contains(want), "{want}: {e}");
    }
    let e = format!("{:#}", build(json!({"effect": "blur", "quality": 3})).unwrap_err());
    assert!(e.contains("one of its option names"), "{e}");
}

#[test]
fn every_blur_of_the_family_is_in_the_growing_list_and_every_choice_default_is_an_option() {
    for name in ["blur", "box_blur", "lens_blur", "directional_blur", "radial_blur", "glow", "film_halation", "shadow"] {
        assert!(super::grows_bounds(name), "{name}");
    }
    for entry in super::CATALOGUE {
        for p in entry.params {
            if let super::ParamKind::Choice { default, options } = p.kind {
                assert!(default < options.len(), "{} {}", entry.effect, p.name);
            }
        }
    }
}
