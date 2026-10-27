//! The blur family as effects, on pictures read back as floats: the choice between the legacy and the exact blur, what
//! each reaches, that its numbers animate, and the effects only the kernel can do (box, lens, unsharp, spin).

use serde_json::{json, Value};
use skia_safe::Surface;

use super::{apply, apply_in, Effect, Space};
use crate::{
    kernel::rig::{painted, pixels, red, Rig},
    layer::Layer,
};

const SIDE: i32 = 161;
const MIDDLE: i32 = SIDE / 2;

fn built(params: Value) -> Effect {
    let mut o = params.as_object().expect("an object").clone();
    let name = o.remove("effect").and_then(|n| n.as_str().map(str::to_string)).expect("an effect name");
    Effect::build(&name, &o).unwrap_or_else(|e| panic!("{name}: {e:#}"))
}

/// A picture of `SIDE` square, `level` at each pixel, and the surface it is drawn on.
fn picture(rig: &mut Rig, level: impl Fn(i32, i32) -> f32) -> (Surface, Layer) {
    let like = rig.target(SIDE, SIDE, None);
    let layer = painted(&like, SIDE, SIDE, level);
    (like, layer)
}

fn impulse(x: i32, y: i32) -> impl Fn(i32, i32) -> f32 {
    move |px, py| f32::from((px, py) == (x, y))
}

/// The red of the picture after `effect` at time `t`.
fn through(effect: Value, t: f64, level: impl Fn(i32, i32) -> f32) -> Vec<f64> {
    let mut rig = Rig::new();
    let (mut like, layer) = picture(&mut rig, level);
    let out = apply(&rig.shaders, like.canvas(), layer, &[built(effect)], t).expect("the effect runs");
    red(&mut like, &out)
}

/// The most pixels from the middle that anything is lit, along the row through it.
fn extent(values: &[f64]) -> i32 {
    let row = &values[(MIDDLE * SIDE) as usize..((MIDDLE + 1) * SIDE) as usize];
    row.iter().enumerate().filter(|(_, v)| **v > 0.0).map(|(x, _)| (x as i32 - MIDDLE).abs()).max().unwrap_or(0)
}

/// The standard deviation of the light along the row through the middle.
fn spread_across(values: &[f64]) -> f64 {
    let row = &values[(MIDDLE * SIDE) as usize..((MIDDLE + 1) * SIDE) as usize];
    let light: f64 = row.iter().sum();
    (row.iter().enumerate().map(|(x, v)| v * f64::from((x as i32 - MIDDLE).pow(2))).sum::<f64>() / light).sqrt()
}

#[test]
fn a_blur_that_does_not_say_which_quality_is_the_legacy_blur_to_the_last_bit() {
    let mut rig = Rig::new();
    let (mut like, layer) = picture(&mut rig, |x, y| if (x / 9 + y / 9) % 2 == 0 { 0.9 } else { 0.1 });
    let by_hand = layer.blurred(like.canvas(), 5.0).expect("the legacy blur");
    let want = pixels(&mut like, &by_hand);
    for params in [json!({"effect": "blur", "radius": 5}), json!({"effect": "blur", "radius": 5, "quality": "legacy"})] {
        let out = apply(&rig.shaders, like.canvas(), layer.clone(), &[built(params.clone())], 0.0).expect("the effect runs");
        assert!(pixels(&mut like, &out) == want, "{params}");
    }
}

#[test]
fn an_exact_blur_has_the_sigma_asked_for_along_the_axes_it_blurs_and_none_along_the_other() {
    for sigma in [4.0, 12.0] {
        let all = through(json!({"effect": "blur", "radius": sigma, "quality": "exact"}), 0.0, impulse(MIDDLE, MIDDLE));
        assert!((spread_across(&all) / sigma - 1.0).abs() < 0.03, "sigma {sigma}: {}", spread_across(&all));
        let row = through(json!({"effect": "blur", "radius": sigma, "quality": "exact", "dimensions": "horizontal"}), 0.0, impulse(MIDDLE, MIDDLE));
        assert!((spread_across(&row) / sigma - 1.0).abs() < 0.03);
        assert_eq!(row[((MIDDLE - 1) * SIDE + MIDDLE) as usize], 0.0, "nothing reaches the rows above, so a horizontal blur is a row");
    }
}

#[test]
fn the_pixels_a_blur_lights_are_the_reach_the_layer_is_grown_by() {
    let cases = [
        json!({"effect": "blur", "radius": 8, "quality": "exact"}),
        json!({"effect": "glow", "radius": 8, "quality": "exact"}),
        json!({"effect": "film_halation", "radius": 8, "threshold": 0.5, "quality": "exact"}),
        json!({"effect": "box_blur", "radius": 5.5, "iterations": 3}),
        json!({"effect": "lens_blur", "radius": 10.6, "blades": 0}),
        json!({"effect": "directional_blur", "distance": 40, "angle": 0, "quality": "exact"}),
        json!({"effect": "blur", "radius": 4}),
        json!({"effect": "directional_blur", "distance": 40, "angle": 0}),
    ];
    for case in cases {
        let reach = built(case.clone()).grows(0.0, [0.0, 0.0, 1.0, 1.0])[2];
        let tail = extent(&through(case.clone(), 0.0, impulse(MIDDLE, MIDDLE)));
        // an exact blur lights nothing past its reach and, but for the weightless last pixels of a bilinear read, all of it
        assert!(f64::from(tail) <= reach && f64::from(tail) >= reach - 2.0, "{case}: lit {tail} px out, grown by {reach}");
    }
}

#[test]
fn a_zoom_lights_no_more_than_the_growth_it_asks_for_past_the_content() {
    // content 41 px square, in the middle of the picture; its last pixel is the source, 20 px from the centre
    let (side, a) = (41, 0.4);
    let mut rig = Rig::new();
    let (mut like, layer) = picture(&mut rig, impulse(MIDDLE + side / 2, MIDDLE));
    let space = Space { origin: [-f64::from(MIDDLE - side / 2); 2], content: [0.0, 0.0, f64::from(side), f64::from(side)] };
    for quality in ["legacy", "exact"] {
        let zoom = built(json!({"effect": "radial_blur", "amount": a, "x": 0.5, "y": 0.5, "quality": quality}));
        let out = apply_in(&rig.shaders, like.canvas(), layer.clone(), std::slice::from_ref(&zoom), 0.0, space).expect("the effect runs");
        let tail = extent(&red(&mut like, &out)) - side / 2;
        let grown = zoom.grows(0.0, space.content)[2];
        assert!(f64::from(tail) <= grown.ceil() && grown - f64::from(tail) < 4.0, "{quality}: lit {tail} px past the content, grown by {grown}");
    }
}

#[test]
fn an_animated_radius_is_read_at_the_time_of_the_frame() {
    let effect = json!({"effect": "blur", "radius": [[0, 2], [1, 16]], "quality": "exact"});
    let (early, late) = (through(effect.clone(), 0.0, impulse(MIDDLE, MIDDLE)), through(effect, 1.0, impulse(MIDDLE, MIDDLE)));
    assert!((spread_across(&early) / 2.0 - 1.0).abs() < 0.03 && (spread_across(&late) / 16.0 - 1.0).abs() < 0.03, "{} then {}", spread_across(&early), spread_across(&late));
}

#[test]
fn a_blur_that_repeats_the_edge_keeps_a_flat_picture_flat_where_a_transparent_one_fades_it() {
    let flat = |_: i32, _: i32| 1.0;
    let transparent = through(json!({"effect": "blur", "radius": 6, "quality": "exact"}), 0.0, flat);
    let repeated = through(json!({"effect": "blur", "radius": 6, "quality": "exact", "edge": "repeat"}), 0.0, flat);
    assert!(transparent[0] < 0.30 && transparent[MIDDLE as usize * SIDE as usize + MIDDLE as usize] > 0.999, "the corner of a transparent-edged blur keeps a quarter of its light: {}", transparent[0]);
    assert!(repeated.iter().all(|v| (v - 1.0).abs() < 1e-3), "a repeated edge has nothing to fade into");
}

#[test]
fn a_box_is_flat_as_wide_as_its_radius_and_a_lens_as_round_as_its_iris() {
    let row_of = |case: Value| {
        let values = through(case, 0.0, impulse(MIDDLE, MIDDLE));
        values[(MIDDLE * SIDE) as usize..((MIDDLE + 1) * SIDE) as usize].to_vec()
    };
    let b = row_of(json!({"effect": "box_blur", "radius": 4, "iterations": 1}));
    assert!(b.iter().filter(|v| **v > 0.0).count() == 9 && b.iter().filter(|v| **v > 0.0).all(|v| (v - 1.0 / 81.0).abs() < 1e-4), "a box of radius 4 is nine equal pixels wide, a ninth of a ninth each on the row through its source: {:?}", &b[MIDDLE as usize - 5..MIDDLE as usize + 6]);
    let lens = through(json!({"effect": "lens_blur", "radius": 10, "blades": 0}), 0.0, impulse(MIDDLE, MIDDLE));
    let lit: Vec<usize> = (0..lens.len()).filter(|i| lens[*i] > 0.0).collect();
    let disc: f64 = lit.iter().map(|i| lens[*i]).sum();
    assert!((disc - 1.0).abs() < 0.005, "the iris keeps the light: {disc}");
    let area = std::f64::consts::PI * 10.0 * 10.0;
    assert!((lit.len() as f64 / area - 1.0).abs() < 0.12, "{} pixels of a disc of area {area:.0}", lit.len());
    assert!(lit.iter().all(|i| lens[*i] < 1.05 / area), "flat topped: no pixel weighs more than one over the area");
}

#[test]
fn unsharp_adds_back_a_fraction_of_the_detail_and_leaves_flat_ground_and_coverage_alone() {
    let step = |x: i32, _: i32| if x < MIDDLE { 0.3 } else { 0.7 };
    let mut rig = Rig::new();
    let (mut like, layer) = picture(&mut rig, step);
    // a paint's colour is held in 8 bits, so the picture's own levels are the ones to compare with
    let plain = pixels(&mut like, &layer);
    let mut sharp = |params: Value| {
        let out = apply(&rig.shaders, like.canvas(), layer.clone(), &[built(params)], 0.0).expect("the effect runs");
        pixels(&mut like, &out)
    };
    let strong = sharp(json!({"effect": "unsharp_mask", "amount": 100, "radius": 3, "threshold": 0}));
    let row = |p: &[[f64; 4]], x: i32| p[(MIDDLE * SIDE + x) as usize];
    let (low, high) = (row(&plain, 10)[0], row(&plain, SIDE - 10)[0]);
    assert!((row(&strong, 10)[0] - low).abs() < 1e-3 && (row(&strong, SIDE - 10)[0] - high).abs() < 1e-3, "flat ground is as it was");
    assert!(row(&strong, MIDDLE - 1)[0] < low - 0.05 && row(&strong, MIDDLE)[0] > high + 0.05, "the edge overshoots on both sides: {} {}", row(&strong, MIDDLE - 1)[0], row(&strong, MIDDLE)[0]);
    assert!(strong.iter().all(|p| p[3] == 1.0), "coverage is not sharpened");
    let gated = sharp(json!({"effect": "unsharp_mask", "amount": 100, "radius": 3, "threshold": 255}));
    assert!((row(&gated, MIDDLE)[0] - high).abs() < 1e-3, "a threshold of every level leaves it alone");
}

#[test]
fn a_spin_turns_about_the_centre_of_the_content_wherever_the_layer_sits() {
    let mut rig = Rig::new();
    let (mut like, layer) = picture(&mut rig, impulse(MIDDLE + 40, MIDDLE));
    // the picture is the content (`SIDE - 60` square, with its centre at the picture's) plus 30 px of room on each side
    let space = Space { origin: [-30.0, -30.0], content: [0.0, 0.0, f64::from(SIDE - 60), f64::from(SIDE - 60)] };
    let spin = built(json!({"effect": "radial_blur", "amount": 40, "x": 0.5, "y": 0.5, "mode": "spin", "quality": "exact"}));
    let out = apply_in(&rig.shaders, like.canvas(), layer, &[spin], 0.0, space).expect("the effect runs");
    let values = red(&mut like, &out);
    let lit: Vec<f64> = (0..values.len()).filter(|i| values[*i] > 1e-4).map(|i| f64::from(i as i32 % SIDE - MIDDLE).hypot(f64::from(i as i32 / SIDE - MIDDLE))).collect();
    assert!(lit.len() > 20 && lit.iter().all(|r| (r - 40.0).abs() < 1.5), "an arc about the content's centre at 40 px: {:?}", lit.iter().cloned().fold((f64::MAX, 0.0), |(lo, hi), r| (f64::min(lo, r), f64::max(hi, r))));
}

/// Light a lone pixel keeps through `blur` of `sigma` on a picture of `side` pixels, in the legacy blur.
fn legacy_light_kept(side: i32, sigma: f64) -> f64 {
    let mut rig = Rig::new();
    let mut like = rig.target(side, side, None);
    let layer = painted(&like, side, side, impulse(side / 2, side / 2));
    let out = apply(&rig.shaders, like.canvas(), layer, &[built(json!({"effect": "blur", "radius": sigma}))], 0.0).expect("the effect runs");
    red(&mut like, &out).iter().sum()
}

#[test]
fn the_legacy_blur_counts_the_light_of_a_small_source_by_its_sigma_and_by_the_size_of_the_picture() {
    // what the exact blur replaces: exact to sigma 4, then wrong in a way that depends on both
    assert!((legacy_light_kept(256, 4.0) - 1.0).abs() < 0.001 && (legacy_light_kept(161, 4.0) - 1.0).abs() < 0.001);
    assert!(legacy_light_kept(256, 6.0) > 1.2, "a 256 px picture invents a quarter more light at sigma 6: {}", legacy_light_kept(256, 6.0));
    assert!(legacy_light_kept(161, 6.0) < 0.9, "a 161 px picture loses a tenth to a half of it: {}", legacy_light_kept(161, 6.0));
    assert!(legacy_light_kept(161, 8.0) < 0.001, "and at sigma 8 all of it: {}", legacy_light_kept(161, 8.0));
    // the exact blur keeps it, whatever the sigma and the picture
    for side in [161, 256] {
        for sigma in [6.0, 8.0, 16.0] {
            let mut rig = Rig::new();
            let mut like = rig.target(side, side, None);
            let layer = painted(&like, side, side, impulse(side / 2, side / 2));
            let out = apply(&rig.shaders, like.canvas(), layer, &[built(json!({"effect": "blur", "radius": sigma, "quality": "exact"}))], 0.0).expect("the effect runs");
            let kept: f64 = red(&mut like, &out).iter().sum();
            assert!((kept - 1.0).abs() < 0.005, "side {side} sigma {sigma}: {kept}");
        }
    }
}
