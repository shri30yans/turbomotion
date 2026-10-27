//! Motion blur of the whole evaluated scene: eases and handles, rotation, nested comps, light, alpha, effects per sample,
//! and what a still layer costs. Each number is a closed form; the helpers are in `shutter_tests`.

use serde_json::{json, Value};

use super::{
    graph::{self, Stage},
    shutter_tests::{centre, ideal, mover, row, sample_centres, smear, worst, FRAME, H, LEVEL, SIZE, W},
    testkit::{bgra, render_at},
};
use crate::{eval::Evaluator, plan::{Plan, ROOT}};

fn pixels(film: Value, frame: f64) -> Vec<u8> {
    render_at(film, std::path::Path::new("."), frame / 30.0)
}

#[test]
fn a_property_with_an_ease_blurs_over_its_true_positions_not_a_straight_line() {
    // x = 100 + 600 u^2 over 2 frames (power1.in, GSAP's quadratic); frame 1 is u = 0.5, x = 250, and a 360 degree shutter spans u in 0.25..0.75.
    // The average of the true positions is 12.5 px past 250 (600 * var(u)); a straight line through the speed at 250 would
    // centre the smear on 250.
    let mut film = mover(0.0, json!({"shutter_angle": 360, "shutter_phase": -180}), true);
    film["layers"][0]["pos"] = json!({"from": [100, 30], "to": [700, 30], "at": 0, "dur": 2.0 / 30.0, "ease": "power1.in"});
    let r = row(film, 1.0, 30);
    let at = |f: f64| 100.0 + 600.0 * (f / 2.0).powi(2);
    let centres: Vec<f64> = (0..16).map(|k| at(0.5 + (f64::from(k) + 0.5) / 16.0)).collect();
    let mean = centres.iter().sum::<f64>() / 16.0;
    let s = smear(&r);
    assert!((s.centroid - mean).abs() < 0.3, "centroid {:.2} against the true mean {mean:.2}", s.centroid);
    assert!(mean - at(1.0) > 10.0 && (s.centroid - at(1.0)).abs() > 10.0, "a straight line would say {}: {:.2}", at(1.0), s.centroid);
    assert!(worst(&r, &ideal(&centres, 16)) <= 2.0);
}

/// The centres the first layer (a `size` square) has at the moments of the shutter: the oracle for handles and paths, which
/// the evaluator owns (its tests hold the curves); here they are only checked to be what is drawn.
fn evaluated(film: &Value, frame: f64, angle: f64, phase: f64, n: usize) -> Vec<[f64; 2]> {
    let plan = Plan::from_value(film.clone(), std::path::Path::new(".")).unwrap();
    let ev = Evaluator::new(&plan);
    let half = film["layers"][0]["size"][0].as_f64().unwrap() / 2.0;
    (0..n).map(|k| ev.frame(ROOT, (frame + ((k as f64 + 0.5) / n as f64 * angle + phase) / 360.0) / 30.0).unwrap().layers[0].origin.map(|o| o + half)).collect()
}

#[test]
fn a_speed_and_influence_handle_blurs_over_the_eased_positions() {
    // a strong ease out (speed 0, influence 90) into a fast arrival: at frame 3 the layer is mid-acceleration
    let mut film = mover(0.0, json!({"shutter_angle": 360, "shutter_phase": -180}), true);
    film["layers"][0]["pos"] = json!({"keys": [
        {"t": 0, "v": [100, 30], "ease_out": {"speed": 0, "influence": 90}},
        {"t": 6.0 / 30.0, "v": [700, 30], "ease_in": {"speed": 0, "influence": 20}}]});
    let c: Vec<f64> = evaluated(&film, 3.0, 360.0, -180.0, 16).iter().map(|p| p[0]).collect();
    let r = row(film, 3.0, 30);
    assert!(c[15] - c[0] > 20.0, "the layer moves {:.1} px across the shutter", c[15] - c[0]);
    let off = worst(&r, &ideal(&c, 16));
    assert!(off <= 2.0, "within {off:.1} levels of the average of the evaluated positions");
}

#[test]
fn a_curved_motion_path_smears_along_the_curve() {
    // a quarter circle of radius 60 in 3 frames, blurred over 2 of them (720 degrees): the mean position is inside the arc,
    // at 0.955 of its radius (R sin(30 deg) / (30 deg in radians)), 2.7 px short, and not on the path
    let film = json!({"width": 200, "height": 200, "fps": 30, "dur": 2, "bg": "#000000", "motion_blur": {"shutter_angle": 720, "shutter_phase": -360},
        "layers": [{"type": "solid", "color": "#ffffff", "size": [8, 8], "mblur": true,
            "pos": {"keys": [{"t": 0, "v": [160, 100], "spatial": "bezier", "out": [0, 33.1], "ease": "none"}, {"t": 0.1, "v": [100, 160], "spatial": "bezier", "in": [33.1, 0], "ease": "none"}]}}]});
    let p = pixels(film.clone(), 2.0);
    let (mut mass, mut mx, mut my) = (0.0, 0.0, 0.0);
    for y in 0..200 {
        for x in 0..200 {
            let v = f64::from(bgra(&p, 200, x, y)[2]) / 255.0;
            (mass, mx, my) = (mass + v, mx + v * (x as f64 + 0.5), my + v * (y as f64 + 0.5));
        }
    }
    let samples = evaluated(&film, 2.0, 720.0, -360.0, 16);
    let mean = [samples.iter().map(|s| s[0]).sum::<f64>() / 16.0, samples.iter().map(|s| s[1]).sum::<f64>() / 16.0];
    let on_path = evaluated(&film, 2.0, 0.0, 0.0, 1)[0];
    assert!(((mx / mass) - mean[0]).abs() < 0.3 && ((my / mass) - mean[1]).abs() < 0.3, "centroid ({:.2}, {:.2}) against ({:.2}, {:.2})", mx / mass, my / mass, mean[0], mean[1]);
    let off_path = ((mean[0] - on_path[0]).powi(2) + (mean[1] - on_path[1]).powi(2)).sqrt();
    assert!(off_path > 2.0, "the average of an arc is not on it: {off_path:.2} px");
}

#[test]
fn a_rotating_bar_sweeps_an_arc_whose_tip_is_where_the_closed_form_says() {
    // a 200 x 6 bar turning 90 degrees per frame about (400, 150); frame 1 is at 90 degrees, and a 180 degree shutter turns it
    // through +-22.5 degrees. A smear that followed the tip in a straight line would put it 7.6 px inside the arc.
    let film = json!({"width": W, "height": 300, "fps": 30, "dur": 2, "bg": "#000000", "motion_blur": {"shutter_angle": 180},
        "layers": [{"type": "solid", "color": "#ffffff", "size": [200, 6], "mblur": true, "pos": [400, 150],
            "rotation": {"from": 0, "to": 2700, "at": 0, "dur": 1, "ease": "none"}}]});
    let p = pixels(film, 1.0);
    let at = |x: f64, y: f64| f64::from(bgra(&p, W, x.floor() as usize, y.floor() as usize)[2]) / 255.0;
    for k in 0..16 {
        let theta = (90.0 + ((f64::from(k) + 0.5) / 16.0 * 180.0 - 90.0) / 360.0 * 90.0_f64).to_radians();
        let tip = |r: f64| (400.0 + r * theta.cos(), 150.0 + r * theta.sin());
        let (inside, outside) = (tip(98.5), tip(101.5));
        assert!(at(inside.0, inside.1) >= 0.9 / 16.0, "sample {k}: the tip's end at radius 98.5 is lit: {}", at(inside.0, inside.1));
        assert!(at(outside.0, outside.1) <= LEVEL, "sample {k}: nothing past the tip's radius 100: {}", at(outside.0, outside.1));
    }
}

#[test]
fn a_black_and_white_edge_moving_through_a_pixel_averages_light_in_linear_light_and_numbers_otherwise() {
    // a 100 px bar's right edge is at 650 at the frame time and moves 3.1 px between samples; pixel 650 is white in the 8
    // samples where the edge is past it and black in the other 8, so it holds half the light, which is 188 on screen
    let edge = |film: Value| bgra(&pixels(film, FRAME), W, 650, 30);
    let mut plain = mover(100.0, json!({"shutter_angle": 180}), true);
    plain["layers"][0]["size"] = json!([100, 20]);
    let mut linear = plain.clone();
    linear["color"] = json!({"linear_light": true});
    let (display, light) = (edge(plain), edge(linear));
    assert!((i32::from(display[2]) - 128).abs() <= 1, "averaged display numbers: {}", display[2]);
    assert!((i32::from(light[2]) - 188).abs() <= 1, "averaged light: {} (half the light is 187.5 on screen)", light[2]);
    assert!(i32::from(light[2]) - i32::from(display[2]) >= 58, "the two differ by 60 levels");
}

#[test]
fn a_half_covered_pixel_takes_half_its_colour_from_the_blurred_layer_and_half_from_what_is_behind() {
    // a red 100 px bar over blue: pixel 650 is red in half the samples. Averaged as premultiplied pictures it is half red over
    // half blue (128, 0, 128); averaging straight colours would have given a quarter red.
    let film = json!({"width": W, "height": H, "fps": 30, "dur": 2, "bg": "#000000", "motion_blur": {"shutter_angle": 180}, "layers": [
        {"type": "solid", "color": "#0000ff", "size": [W, H], "pos": [W / 2, H / 2]},
        {"type": "solid", "color": "#ff0000", "size": [100, SIZE], "mblur": true, "pos": {"from": [100, 30], "to": [3100, 30], "at": 0, "dur": 1, "ease": "none"}}]});
    let [b, g, r, a] = bgra(&pixels(film, FRAME), W, 650, 30);
    assert!(i32::from(r).abs_diff(128) <= 1 && i32::from(b).abs_diff(128) <= 1 && g == 0 && a == 255, "(r, g, b, a) = ({r}, {g}, {b}, {a})");
}

#[test]
fn layers_without_the_switch_stay_sharp_in_a_blurred_comp_and_a_nested_comp_follows_its_layer() {
    let mut film = mover(100.0, json!({"shutter_angle": 360}), true);
    film["layers"].as_array_mut().unwrap().push(json!({"type": "solid", "color": "#ffffff", "size": [SIZE, SIZE], "pos": {"from": [100, 45], "to": [3100, 45], "at": 0, "dur": 1, "ease": "none"}}));
    let (blurred, sharp) = (smear(&row(film.clone(), FRAME, 30)), smear(&row(film, FRAME, 45)));
    assert!(blurred.extent > 100 && (sharp.first, sharp.extent) == (590, 20), "{} {}", blurred.extent, sharp.extent);
    // a precomp whose layer has the switch carries all of its moving content; a switched layer inside one that has none is sharp
    let nested = |outer: bool, inner: bool| json!({"width": W, "height": H, "fps": 30, "dur": 2, "bg": "#000000", "motion_blur": {"shutter_angle": 360},
        "comps": {"inner": {"width": W, "height": H, "dur": 2, "layers": [{"type": "solid", "color": "#ffffff", "size": [SIZE, SIZE], "mblur": inner, "pos": {"from": [100, 30], "to": [3100, 30], "at": 0, "dur": 1, "ease": "none"}}]}},
        "layers": [{"type": "precomp", "comp": "inner", "mblur": outer}]});
    assert!(smear(&row(nested(true, false), FRAME, 30)).extent > 100, "the switch on the precomp layer blurs its content");
    assert_eq!(smear(&row(nested(false, true), FRAME, 30)).extent, 20, "a switch inside is not enough");
}

#[test]
fn a_still_layer_is_drawn_once_and_exactly_as_without_blur_while_a_moving_one_is_averaged() {
    let layer = |id: &str, x: Value| json!({"type": "solid", "id": id, "color": "#ffffff", "size": [SIZE, SIZE], "mblur": true, "pos": x});
    let film = |layers: Vec<Value>| json!({"width": W, "height": H, "fps": 30, "dur": 2, "bg": "#101010", "motion_blur": {"shutter_angle": 180}, "layers": layers});
    let moving = json!({"from": [100, 20], "to": [3100, 20], "at": 0, "dur": 1, "ease": "none"});
    let averages = |film: &Value| {
        let plan = Plan::from_value(film.clone(), std::path::Path::new(".")).unwrap();
        let ev = Evaluator::new(&plan);
        let t = FRAME / 30.0;
        let (frame, samples) = (ev.frame(ROOT, t).unwrap(), graph::shutter::frames(&plan, &ev, t).unwrap());
        let g = graph::comp_graph(plan.root(), &frame, &samples);
        (samples.len(), g.passes.iter().filter(|p| matches!(p.stage, Stage::ShutterAverage { .. })).count())
    };
    assert_eq!(averages(&film(vec![layer("still", json!([300, 45])), layer("mover", moving.clone())])), (16, 1), "one of two is averaged");
    assert_eq!(averages(&film(vec![layer("still", json!([300, 45]))])), (16, 0), "nothing is");
    // and the pixels of a scene that is all still are those of the same scene with no blur at all
    let still = film(vec![layer("a", json!([300, 45])), layer("b", json!([500, 20]))]);
    let mut off = still.clone();
    off.as_object_mut().unwrap().remove("motion_blur");
    for l in off["layers"].as_array_mut().unwrap() {
        l["mblur"] = json!(false);
    }
    assert!(pixels(still, FRAME) == pixels(off, FRAME), "a still scene is byte for byte what it is without blur");
}

#[test]
fn a_layer_with_effects_is_never_taken_for_still_and_each_moment_runs_its_effects() {
    // platform effects are read at draw time, not held in the evaluated state, so a layer that has any cannot be compared
    let film = json!({"width": W, "height": H, "fps": 30, "dur": 2, "bg": "#000000", "motion_blur": {"shutter_angle": 180}, "layers": [
        {"type": "solid", "color": "#ffffff", "size": [SIZE, SIZE], "mblur": true, "pos": [300, 30], "fx": [{"effect": "blur", "radius": 3}]}]});
    let plan = Plan::from_value(film, std::path::Path::new(".")).unwrap();
    let ev = Evaluator::new(&plan);
    let t = FRAME / 30.0;
    let (frame, samples) = (ev.frame(ROOT, t).unwrap(), graph::shutter::frames(&plan, &ev, t).unwrap());
    let g = graph::comp_graph(plan.root(), &frame, &samples);
    let effects: Vec<usize> = g.passes.iter().filter(|p| matches!(p.stage, Stage::Effects { .. })).map(|p| p.frame).collect();
    assert_eq!(effects, (1..=16).collect::<Vec<_>>(), "the layer's effects run once for each of the 16 moments");
}

#[test]
fn the_older_key_is_the_same_blur_as_the_new_one_where_they_agree() {
    // "mblur" is centred on the frame whatever its angle: that is `motion_blur` with a phase of minus half the angle
    for angle in [180.0, 360.0] {
        let old = json!({"width": W, "height": H, "fps": 30, "dur": 2, "bg": "#000000", "mblur": {"samples": 8, "shutter": angle},
            "layers": mover(100.0, json!({}), true)["layers"]});
        let new = mover(100.0, json!({"shutter_angle": angle, "shutter_phase": -angle / 2.0, "samples": 8}), true);
        assert!(pixels(old, FRAME) == pixels(new, FRAME), "angle {angle}");
    }
}

#[test]
fn rendering_a_blurred_frame_twice_gives_the_same_bytes() {
    let film = mover(100.0, json!({"shutter_angle": 270, "shutter_phase": -30, "samples": 12}), true);
    assert!(pixels(film.clone(), FRAME) == pixels(film, FRAME));
}

#[test]
fn the_expected_row_helper_agrees_with_a_sharp_render() {
    // guards the oracle itself: a single sample is the square as drawn
    let r = row(mover(100.0, json!({"shutter_angle": 0}), true), FRAME, 30);
    assert!(worst(&r, &ideal(&sample_centres(centre(100.0, FRAME), 100.0, 0.0, 0.0, 1), 1)) <= 1.0);
}
