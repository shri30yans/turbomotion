//! Composite options on effects: the effect's result laid over its input through a mask, with a blend and an amount.
//! The same options work on platform effects and on shader plugins, because one implementation serves both.

use std::path::Path;

use serde_json::{json, Value};

use super::testkit::{bgra, render, render_at};
use crate::plan::Plan;

const W: usize = 64;
const H: usize = 32;
const ROW: usize = 16;
const DARK: usize = 8;
const BRIGHT: usize = 50;

fn solid(color: &str, x: f64) -> Value {
    json!({"type": "solid", "color": color, "size": [32, 32], "pos": [x, 16]})
}

/// A picture with a dark left half and a bright right half, carrying `fx` and any `extra` layers beside it.
fn halves(fx: Option<Value>, extra: Vec<Value>) -> Value {
    let mut layer = json!({"type": "precomp", "comp": "src"});
    if let Some(fx) = fx {
        layer["fx"] = json!([fx]);
    }
    let mut layers = vec![layer];
    layers.extend(extra);
    json!({"width": W, "height": H, "dur": 1, "layers": layers,
        "comps": {"src": {"width": W, "height": H, "dur": 1, "layers": [solid("#202020", 16.0), solid("#e0e0e0", 48.0)]}}})
}

fn invert(composite: Option<Value>) -> Value {
    let mut fx = json!({"effect": "invert", "amount": 1});
    if let Some(c) = composite {
        fx["composite"] = c;
    }
    fx
}

fn at(px: &[u8], x: usize) -> [u8; 4] {
    bgra(px, W, x, ROW)
}

#[test]
fn a_luma_mask_leaves_dark_pixels_alone_and_changes_bright_ones() {
    let base = render(halves(None, vec![]));
    let plain = render(halves(Some(invert(None)), vec![]));
    let masked = render(halves(Some(invert(Some(json!({"mask": {"kind": "luma", "lo": 0.5, "softness": 0}})))), vec![]));
    assert_eq!(at(&masked, DARK), at(&base, DARK), "below lo the input stays");
    assert_ne!(at(&masked, BRIGHT), at(&base, BRIGHT), "above lo the effect shows");
    assert_eq!(at(&masked, BRIGHT), at(&plain, BRIGHT), "at full mix the effect shows exactly as it would alone");
    let flipped = render(halves(Some(invert(Some(json!({"mask": {"kind": "luma", "lo": 0.5, "softness": 0, "invert": true}})))), vec![]));
    assert_eq!(at(&flipped, BRIGHT), at(&base, BRIGHT));
    assert_eq!(at(&flipped, DARK), at(&plain, DARK));
}

#[test]
fn a_mix_of_zero_is_the_input_and_a_mix_can_be_animated() {
    let base = render(halves(None, vec![]));
    let none = render(halves(Some(invert(Some(json!({"mix": 0})))), vec![]));
    assert_eq!(none, base, "mix 0 changes nothing");
    let ramp = json!({"mix": {"keys": [[0, 0, "none"], [0.5, 0.5, "none"], [1, 1]]}});
    let half = render_at(halves(Some(invert(Some(ramp))), vec![]), Path::new("."), 0.5);
    let (before, after) = (i32::from(at(&base, BRIGHT)[2]), i32::from(at(&render(halves(Some(invert(None)), vec![])), BRIGHT)[2]));
    let mid = i32::from(at(&half, BRIGHT)[2]);
    assert!((mid - (before + after) / 2).abs() <= 3, "at the key where mix is one half the picture is half way between: {before} {mid} {after}");
}

#[test]
fn add_never_darkens() {
    let base = render(halves(None, vec![]));
    let added = render(halves(Some(invert(Some(json!({"blend": "add"})))), vec![]));
    let brighter = base.chunks_exact(4).zip(added.chunks_exact(4)).filter(|(b, a)| a[..3].iter().zip(&b[..3]).any(|(a, b)| a > b)).count();
    assert!(base.chunks_exact(4).zip(added.chunks_exact(4)).all(|(b, a)| a[..3].iter().zip(&b[..3]).all(|(a, b)| a >= b)), "a channel got darker");
    assert!(brighter > 0, "and something got brighter");
}

#[test]
fn an_edges_mask_keeps_the_effect_to_outlines() {
    let value = |fx: Option<Value>| {
        let mut v = halves(fx, vec![]);
        v["comps"]["src"]["layers"] = json!([solid("#000000", 16.0), solid("#ffffff", 48.0)]);
        v
    };
    let base = render(value(None));
    let edged = render(value(Some(invert(Some(json!({"mask": {"kind": "edges", "width": 1.5}}))))));
    assert_eq!(at(&edged, 5), at(&base, 5), "far from the border nothing changes");
    assert_eq!(at(&edged, 58), at(&base, 58));
    assert_ne!(at(&edged, 31), at(&base, 31), "on the border the effect shows");
}

#[test]
fn an_alpha_mask_cuts_the_effect_at_the_inputs_own_shape() {
    let dot = |fx: Value| json!({"width": W, "height": H, "dur": 1, "layers": [{"type": "solid", "color": "#808080", "size": [16, 16], "pos": [32, 16], "fx": [fx]}]});
    let outline = json!({"effect": "outline", "width": 4, "color": [255, 255, 255]});
    let base = render(dot(json!({"effect": "opacity", "value": 1})));
    let free = render(dot(outline.clone()));
    let mut cut = outline;
    cut["composite"] = json!({"mask": "alpha"});
    let cut = render(dot(cut));
    let outside = 32 + 8 + 2;
    assert_ne!(at(&free, outside), at(&base, outside), "the outline reaches outside the square");
    assert_eq!(at(&cut, outside), at(&base, outside), "an alpha mask stops it at the square's edge");
}

#[test]
fn another_layers_brightness_can_be_the_mask() {
    let spot = json!({"type": "solid", "id": "spot", "color": "#ffffff", "size": [32, 32], "pos": [16, 16], "hidden": true});
    let base = render(halves(None, vec![]));
    let plain = render(halves(Some(invert(None)), vec![]));
    let masked = render(halves(Some(invert(Some(json!({"mask": {"kind": "layer", "layer": "spot"}})))), vec![spot.clone()]));
    assert_eq!(at(&masked, DARK), at(&plain, DARK), "inside the white spot the effect shows");
    assert_eq!(at(&masked, BRIGHT), at(&base, BRIGHT), "outside it the input stays");
    let flipped = render(halves(Some(invert(Some(json!({"mask": {"kind": "layer", "layer": "spot", "invert": true}})))), vec![spot]));
    assert_eq!(at(&flipped, DARK), at(&base, DARK));
}

#[test]
fn a_plugin_takes_the_same_options() {
    let duotone = |c: Option<Value>| {
        let mut fx = json!({"effect": "duotone"});
        if let Some(c) = c {
            fx["composite"] = c;
        }
        fx
    };
    let base = render(halves(None, vec![]));
    let plain = render(halves(Some(duotone(None)), vec![]));
    let masked = render(halves(Some(duotone(Some(json!({"mask": {"kind": "luma", "lo": 0.5, "softness": 0}})))), vec![]));
    assert_ne!(at(&plain, DARK), at(&base, DARK), "the plugin changes the dark half on its own");
    assert_eq!(at(&masked, DARK), at(&base, DARK));
    assert_eq!(at(&masked, BRIGHT), at(&plain, BRIGHT));
}

fn plan_error(composite: Value, extra: Vec<Value>) -> String {
    format!("{:#}", Plan::from_value(halves(Some(invert(Some(composite))), extra), Path::new(".")).err().expect("the plan should be rejected"))
}

#[test]
fn mistakes_in_the_options_are_plan_errors_that_say_what_is_valid() {
    let e = plan_error(json!({"mixx": 0.5}), vec![]);
    assert!(e.contains("Did you mean \"mix\"") && e.contains("mix, blend, mask"), "{e}");
    let e = plan_error(json!({"blend": "glitter"}), vec![]);
    assert!(e.contains("blend") && e.contains("glitter"), "{e}");
    let e = plan_error(json!({"mask": {"kind": "stripes"}}), vec![]);
    assert!(e.contains("use none, alpha, luma, edges or layer"), "{e}");
    let e = plan_error(json!({"mask": "luma"}), vec![]);
    assert!(e.contains("take options"), "{e}");
    let e = plan_error(json!({"mask": {"kind": "luma", "hii": 1}}), vec![]);
    assert!(e.contains("Did you mean \"hi\""), "{e}");
    let e = plan_error(json!({"mask": {"kind": "layer", "layer": "nothing"}}), vec![]);
    assert!(e.contains("does not exist"), "{e}");
    let e = plan_error(json!({"mask": {"kind": "layer", "layer": "cam"}}), vec![json!({"type": "camera", "id": "cam"})]);
    assert!(e.contains("has no picture"), "{e}");
}

#[test]
fn inspect_reports_the_evaluated_options() {
    let plan = Plan::from_value(halves(Some(invert(Some(json!({"mix": 0.25, "mask": {"kind": "luma", "lo": 0.4}})))), vec![]), Path::new(".")).expect("the plan");
    let frame = crate::eval::Evaluator::new(&plan).frame(crate::plan::ROOT, 0.0).expect("the frame");
    let fx = serde_json::to_value(&frame.layers[0].fx[0]).expect("serialises");
    assert_eq!(fx["composite"]["mix"], json!(0.25));
    assert_eq!(fx["composite"]["mask"]["kind"], json!("luma"));
    assert!((fx["composite"]["mask"]["lo"].as_f64().expect("lo") - 0.4).abs() < 1e-6);
}
