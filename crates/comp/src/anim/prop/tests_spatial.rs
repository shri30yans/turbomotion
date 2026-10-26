//! Motion paths: bezier curves through the keys of a position, measured by distance travelled.

use serde_json::json;

use super::{
    key::Tangents,
    spatial::{stops, Path},
    testkit::{at, dist, position},
};

/// A gentle S-curve through three keys, travelled at a steady pace.
fn s_curve() -> super::VecProp {
    position(json!({"keys": [
        {"t": 0, "v": [0, 0], "ease": "linear"},
        {"t": 2, "v": [300, 120], "ease": "linear"},
        {"t": 4, "v": [500, 0], "ease": "linear"}]}))
}

/// The speed in pixels per second at `t`, from two samples a little apart.
fn speed(p: &super::VecProp, t: f64) -> f64 {
    let h = 1e-4;
    dist(&at(p, t - h), &at(p, t + h)) / (2.0 * h)
}

#[test]
fn the_curve_passes_through_its_keys_exactly() {
    let p = s_curve();
    assert_eq!(at(&p, 0.0), vec![0.0, 0.0]);
    assert_eq!(at(&p, 2.0), vec![300.0, 120.0]);
    assert_eq!(at(&p, 4.0), vec![500.0, 0.0]);
    assert!(dist(&at(&p, 2.0 - 1e-9), &[300.0, 120.0]) < 1e-5);
}

#[test]
fn auto_bezier_handles_are_a_sixth_of_the_way_between_the_neighbours() {
    let a = [0.0, 0.0, 0.0];
    let (b, c) = ([60.0, 30.0, 0.0], [120.0, 0.0, 0.0]);
    let s = stops(&[(a, &Tangents::Auto), (b, &Tangents::Auto), (c, &Tangents::Auto)]);
    assert_eq!(s[1].out, [20.0, 0.0, 0.0], "(c - a) / 6");
    assert_eq!(s[1].into, [-20.0, 0.0, 0.0], "its mirror");
    assert_eq!(s[0].out, [10.0, 5.0, 0.0], "the first key uses its one neighbour: (b - a) / 6");
    assert_eq!(s[2].into, [-10.0, 5.0, 0.0], "the last: (b - c) / 6");
    // a side that faces a key in the same place has no handle
    let s = stops(&[(a, &Tangents::Auto), (a, &Tangents::Auto), (c, &Tangents::Auto)]);
    assert_eq!((s[1].into, s[0].out), ([0.0; 3], [0.0; 3]));
}

#[test]
fn the_direction_is_continuous_through_an_auto_key() {
    let p = s_curve();
    let (before, after) = (track(&p).direction(2.0 - 1e-6).unwrap(), track(&p).direction(2.0 + 1e-6).unwrap());
    let dot: f64 = before.iter().zip(after).map(|(a, b)| a * b).sum();
    assert!(dot > 1.0 - 1e-9, "turned by {} degrees", dot.acos().to_degrees());
}

fn track(p: &super::VecProp) -> &super::Track {
    match p {
        super::VecProp::Joined { track, .. } => track,
        super::VecProp::Axes(_) => panic!("keys of whole vectors"),
    }
}

#[test]
fn a_linear_ease_travels_the_curve_at_a_constant_speed() {
    // each segment has its own time, so the speed is length / duration within a segment, steady to 0.1 %
    let p = s_curve();
    for (i, (from, to)) in [(0.0, 2.0), (2.0, 4.0)].into_iter().enumerate() {
        let samples: Vec<f64> = (1..40).map(|k| speed(&p, from + (to - from) * f64::from(k) / 40.0)).collect();
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!(samples.iter().all(|s| (s - mean).abs() / mean < 1e-3), "segment {i}: {samples:?}");
    }
}

#[test]
fn roving_keys_keep_one_speed_across_the_whole_path() {
    // two roving points between fixed keys at 0 and 3 seconds, a linear ease: the speed is the same all the way
    let p = position(json!({"keys": [
        {"t": 0, "v": [0, 0], "ease": "linear"},
        {"roving": true, "v": [50, 200]},
        {"roving": true, "v": [400, 220]},
        {"t": 3, "v": [450, 20], "ease": "linear"}]}));
    let samples: Vec<f64> = (1..120).map(|k| speed(&p, 3.0 * f64::from(k) / 120.0)).collect();
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    let worst = samples.iter().map(|s| (s - mean).abs() / mean).fold(0.0, f64::max);
    assert!(worst < 1e-3, "speed varies by {worst}: {samples:?}");
    // the same points as fixed keys at even times move at very different speeds
    let fixed = position(json!({"keys": [
        {"t": 0, "v": [0, 0], "ease": "linear"}, {"t": 1, "v": [50, 200], "ease": "linear"},
        {"t": 2, "v": [400, 220], "ease": "linear"}, {"t": 3, "v": [450, 20], "ease": "linear"}]}));
    assert!(speed(&fixed, 1.5) > 1.5 * speed(&fixed, 0.5), "{} vs {}", speed(&fixed, 1.5), speed(&fixed, 0.5));
}

#[test]
fn a_roving_point_is_passed_exactly_when_the_arc_length_reaches_it() {
    let p = position(json!({"keys": [
        {"t": 0, "v": [0, 0], "spatial": "linear", "ease": "linear"},
        {"roving": true, "v": [100, 0], "spatial": "linear"},
        {"t": 4, "v": [400, 0], "spatial": "linear", "ease": "linear"}]}));
    // a straight line of 400 px in 4 s: 100 px/s, so the roving point at x = 100 is reached at t = 1
    assert!(dist(&at(&p, 1.0), &[100.0, 0.0]) < 1e-9);
}

#[test]
fn a_path_runs_on_along_its_last_tangent_when_the_timing_overshoots() {
    // leaving at 600 px/s with 80 % influence puts the first time handle far past the end of the 141 px diagonal
    let p = position(json!({"keys": [
        {"t": 0, "v": [0, 0], "spatial": "linear", "ease_out": {"speed": 600, "influence": 80}},
        {"t": 1, "v": [100, 100], "spatial": "linear"}]}));
    let ahead = (0..=1000).map(|i| at(&p, f64::from(i) / 1000.0)).map(|v| v[0]).fold(f64::MIN, f64::max);
    assert!(ahead > 110.0, "{ahead}");
    for i in 0..=100 {
        let v = at(&p, f64::from(i) / 100.0);
        assert!((v[0] - v[1]).abs() < 1e-9, "left the diagonal: {v:?}");
    }
    assert_eq!(at(&p, 1.0), vec![100.0, 100.0]);
}

#[test]
fn a_path_hold_stays_at_its_key_until_the_next() {
    let p = position(json!({"keys": [{"t": 0, "v": [10, 20], "ease_out": "hold"}, {"t": 1, "v": [300, 400]}]}));
    assert_eq!(at(&p, 0.999_9), vec![10.0, 20.0]);
    assert_eq!(at(&p, 1.0), vec![300.0, 400.0]);
}

#[test]
fn explicit_handles_make_the_curve_the_bezier_they_describe() {
    // the quarter circle approximation: handles 55.23 % of the radius
    let k = 0.552_284_749_8_f64 * 100.0;
    let p = position(json!({"keys": [
        {"t": 0, "v": [100, 0], "spatial": "bezier", "out": [0, k], "ease": "linear"},
        {"t": 1, "v": [0, 100], "spatial": "bezier", "in": [k, 0], "ease": "linear"}]}));
    for i in 0..=20 {
        let v = at(&p, f64::from(i) / 20.0);
        assert!((v[0].hypot(v[1]) - 100.0).abs() < 0.03, "radius {} at {i}", v[0].hypot(v[1]));
    }
    // its length is a quarter of the circle, 157.08, to 0.03 %; and arc length by 200 000 chords agrees to 1e-6
    let s = super::spatial::stops(&[([100.0, 0.0, 0.0], &Tangents::Bezier { into: [0.0; 3], out: [0.0, k, 0.0] }), ([0.0, 100.0, 0.0], &Tangents::Bezier { into: [k, 0.0, 0.0], out: [0.0; 3] })]);
    let path = Path::through(&s);
    assert!((path.length() - std::f64::consts::FRAC_PI_2 * 100.0).abs() / 157.08 < 3e-4, "{}", path.length());
    let chords: f64 = (0..200_000).map(|i| dist(&path_at(&s, f64::from(i) / 200_000.0), &path_at(&s, f64::from(i + 1) / 200_000.0))).sum();
    assert!((path.length() - chords).abs() / chords < 1e-6, "{} vs {chords}", path.length());
}

#[test]
fn halfway_round_a_quarter_circle_the_path_points_at_45_degrees() {
    // arc-length parametrisation: half the distance is the 45 degree point, (70.71, 70.71) on a 100 px circle, and
    // the direction of travel there is 135 degrees (counter-clockwise from +x at the start, +y at the end)
    let k = 0.552_284_749_8_f64 * 100.0;
    let p = position(json!({"keys": [
        {"t": 0, "v": [100, 0], "spatial": "bezier", "out": [0, k], "ease": "linear"},
        {"t": 2, "v": [0, 100], "spatial": "bezier", "in": [k, 0], "ease": "linear"}], "auto_orient": "path"}));
    let half = at(&p, 1.0);
    let want = 100.0 * std::f64::consts::FRAC_1_SQRT_2;
    assert!(dist(&half, &[want, want]) < 0.5, "{half:?} vs {want}");
    let d = p.heading(1.0).unwrap();
    assert!((d[1].atan2(d[0]).to_degrees() - 135.0).abs() < 0.5, "{d:?}");
}

/// The point of the cubic at parameter `u`, written out directly.
fn path_at(s: &[super::spatial::Stop], u: f64) -> [f64; 2] {
    let c = |k: usize| [s[0].at[k], s[0].at[k] + s[0].out[k], s[1].at[k] + s[1].into[k], s[1].at[k]];
    let f = |c: [f64; 4]| (1.0 - u).powi(3) * c[0] + 3.0 * (1.0 - u).powi(2) * u * c[1] + 3.0 * (1.0 - u) * u * u * c[2] + u.powi(3) * c[3];
    [f(c(0)), f(c(1))]
}

#[test]
fn a_position_in_3d_follows_a_curve_through_depth() {
    let p = position(json!({"keys": [{"t": 0, "v": [0, 0, 0], "ease": "linear"}, {"t": 1, "v": [100, 0, 300], "ease": "linear"}], "spatial": "linear"}));
    assert!(dist(&at(&p, 0.5), &[50.0, 0.0, 150.0]) < 1e-9);
}

#[test]
fn the_heading_of_a_straight_run_is_its_direction() {
    let p = position(json!({"keys": [{"t": 0, "v": [0, 0], "spatial": "linear"}, {"t": 1, "v": [0, 100], "spatial": "linear"}], "auto_orient": "path"}));
    let d = p.heading(0.5).expect("auto_orient on");
    assert!(dist(&d, &[0.0, 1.0, 0.0]) < 1e-12, "{d:?}");
    assert!((d[1].atan2(d[0]).to_degrees() - 90.0).abs() < 1e-9);
}
