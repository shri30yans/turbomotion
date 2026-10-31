//! Temporal interpolation: speed and influence handles, as After Effects' Keyframe Velocity dialog has them.

use serde_json::{json, Value};

use super::testkit::{reference, scalar, scalar_err};
use crate::anim::bezier;

/// Two keys, 0 to 100 over one second, with the given sides.
fn segment(out: Value, into: Value) -> super::Prop {
    scalar(json!({"keys": [{"t": 0, "v": 0, "ease_out": out}, {"t": 1, "v": 100, "ease_in": into}]}))
}

fn handle(speed: f64, percent: f64) -> Value {
    json!({"speed": speed, "influence": percent})
}

/// One side of a key: its speed and its influence.
type KeySide = (f64, f64);
/// A fidelity case: the name, the start and end value, the outgoing and the incoming side.
type Case = (&'static str, f64, f64, KeySide, KeySide);

#[test]
fn a_third_of_the_way_at_the_segments_own_speed_is_a_straight_line() {
    // influence 33.333.. % both sides at the speed 100/s: the control points sit on the chord at thirds, so the
    // value is 100 * t to the last digit
    let third = 100.0 / 3.0;
    let p = segment(handle(100.0, third), handle(100.0, third));
    for i in 0..=20 {
        let t = f64::from(i) / 20.0;
        assert!((p.base(t) - 100.0 * t).abs() < 1e-9, "t={t}: {}", p.base(t));
    }
    // 33.333 as typed in After Effects is off by 1e-5 of a third, so within 1e-3 of a hundred
    let typed = segment(handle(100.0, 33.333), handle(100.0, 33.333));
    assert!((0..=20).all(|i| (typed.base(f64::from(i) / 20.0) - 100.0 * f64::from(i) / 20.0).abs() < 1e-3));
}

#[test]
fn a_linear_side_is_the_segments_speed_with_a_third_influence() {
    let third = 100.0 / 3.0;
    let named = segment(json!("linear"), handle(40.0, 60.0));
    let spelled = segment(handle(100.0, third), handle(40.0, 60.0));
    for i in 0..=40 {
        let t = f64::from(i) / 40.0;
        assert!((named.base(t) - spelled.base(t)).abs() < 1e-9, "t={t}");
    }
}

#[test]
fn easy_ease_is_the_smoothstep_of_its_two_handles() {
    // speed 0 and influence 33.333.. % at both ends: x(u) = u, y(u) = 100 (3u^2 - 2u^3), so value(t) = 100 (3t^2 - 2t^3)
    let p = segment(json!("easy"), json!("easy"));
    for i in 0..=40 {
        let t = f64::from(i) / 40.0;
        let want = 100.0 * (3.0 * t * t - 2.0 * t * t * t);
        assert!((p.base(t) - want).abs() < 1e-9, "t={t}: {} vs {want}", p.base(t));
    }
    assert_eq!(p.base(0.5), 50.0);
}

#[test]
fn full_and_zero_influence_ease_matches_its_closed_form() {
    // out: speed 0, 100 %; in: 0 %. Control x are (1, 1): x(u) = 3u - 3u^2 + u^3, y(u) = 100 (3u^2 - 2u^3). Solving for
    // x = 0.5 gives u = 0.2063, which the reference finds by bisection
    let p = segment(handle(0.0, 100.0), handle(500.0, 0.0));
    for i in 1..20 {
        let x = f64::from(i) / 20.0;
        let want = reference(0.0, 100.0, 1.0, (0.0, 1.0), (500.0, 0.0), x);
        assert!((p.base(x) - want).abs() < 1e-9, "x={x}: {} vs {want}", p.base(x));
    }
    // at x = 0.5, u solves 3u - 3u^2 + u^3 = 0.5 (u = 0.20628), and y = 100 (3u^2 - 2u^3) = 11.012
    assert!((p.base(0.5) - 11.012).abs() < 5e-4, "{}", p.base(0.5));
}

#[test]
fn both_influences_at_a_hundred_stall_at_the_middle_and_still_solve() {
    // the x curve is flat at u = 0.5 (slope 0), where plain Newton fails; y(0.5) = 50 by symmetry
    let p = segment(handle(0.0, 100.0), handle(0.0, 100.0));
    assert!((p.base(0.5) - 50.0).abs() < 1e-9, "{}", p.base(0.5));
    let mut last = 0.0;
    for i in 0..=1000 {
        let v = p.base(f64::from(i) / 1000.0);
        assert!(v >= last - 1e-12, "not monotone at {i}: {v} < {last}");
        last = v;
    }
}

#[test]
fn the_time_mapping_is_monotone_and_exact_for_every_pair_of_influences() {
    for i0 in 0..=10 {
        for i1 in 0..=10 {
            let (x1, x2) = (f64::from(i0) / 10.0, 1.0 - f64::from(i1) / 10.0);
            let mut last = 0.0;
            for k in 0..=200 {
                let x = f64::from(k) / 200.0;
                let u = bezier::solve(x1, x2, x);
                assert!(u >= last - 1e-12, "u went back at x={x} for influences {i0}/{i1}");
                assert!((bezier::point([0.0, x1, x2, 1.0], u) - x).abs() < 1e-12, "x(u) != x at x={x} for {i0}/{i1}");
                last = u;
            }
        }
    }
}

#[test]
fn a_fast_out_speed_with_a_slow_in_speed_overshoots_and_comes_back() {
    // out: 400/s over half the time puts the first handle at 200, past the end at 100
    let p = segment(handle(400.0, 50.0), handle(0.0, 33.0));
    let peak = (0..=1000).map(|i| p.base(f64::from(i) / 1000.0)).fold(f64::MIN, f64::max);
    assert!(peak > 110.0, "peak {peak}");
    assert_eq!(p.base(0.0), 0.0);
    assert_eq!(p.base(1.0), 100.0);
    for i in 1..20 {
        let x = f64::from(i) / 20.0;
        assert!((p.base(x) - reference(0.0, 100.0, 1.0, (400.0, 0.5), (0.0, 0.33), x)).abs() < 1e-9);
    }
}

#[test]
fn the_speed_at_a_handle_is_the_slope_of_the_curve_there() {
    // out speed 250 units/s: just after the key the value rises by 250 per second
    let p = segment(handle(250.0, 40.0), handle(250.0, 40.0));
    let h = 1e-5;
    assert!((p.base(h) / h - 250.0).abs() < 0.5, "{}", p.base(h) / h);
    assert!(((p.base(1.0) - p.base(1.0 - h)) / h - 250.0).abs() < 0.5);
}

#[test]
fn a_hold_key_steps_exactly_at_the_next_key() {
    let p = scalar(json!({"keys": [{"t": 1, "v": 7, "ease_out": "hold"}, {"t": 2, "v": 9}, {"t": 3, "v": 2}]}));
    assert_eq!(p.base(0.0), 7.0);
    assert_eq!(p.base(1.999_999_9), 7.0);
    assert_eq!(p.base(2.0), 9.0);
    assert!((p.base(2.5) - 9.0).abs() > 1e-6, "the segment after a normal key moves");
}

#[test]
fn an_auto_bezier_key_takes_the_slope_between_its_neighbours_with_a_sixth_influence() {
    // keys (0,0) (1,10) (2,40): the middle key's speed is (40 - 0) / 2 = 20/s. The ends stand still.
    let auto = scalar(json!({"keys": [{"t": 0, "v": 0, "ease_out": "auto"}, {"t": 1, "v": 10, "ease_in": "auto", "ease_out": "auto"}, {"t": 2, "v": 40, "ease_in": "auto"}]}));
    let sixth = 100.0 / 6.0;
    for i in 0..=20 {
        let x = f64::from(i) / 20.0;
        assert!((auto.base(x) - reference(0.0, 10.0, 1.0, (0.0, 1.0 / 6.0), (20.0, 1.0 / 6.0), x)).abs() < 1e-9);
        assert!((auto.base(1.0 + x) - reference(10.0, 40.0, 1.0, (20.0, 1.0 / 6.0), (0.0, 1.0 / 6.0), x)).abs() < 1e-9);
    }
    let by_hand = scalar(json!({"keys": [{"t": 0, "v": 0, "ease_out": {"speed": 0, "influence": sixth}}, {"t": 1, "v": 10, "ease_in": {"speed": 20, "influence": sixth}}]}));
    assert!((auto.base(0.5) - by_hand.base(0.5)).abs() < 1e-9);
}

#[test]
fn a_continuous_key_shares_one_speed_between_its_sides() {
    let p = scalar(json!({"keys": [
        {"t": 0, "v": 0},
        {"t": 1, "v": 10, "continuous": true, "ease_in": {"speed": 30, "influence": 25}, "ease_out": {"influence": 60}},
        {"t": 3, "v": 50}]}));
    let q = scalar(json!({"keys": [
        {"t": 0, "v": 0},
        {"t": 1, "v": 10, "ease_in": {"speed": 30, "influence": 25}, "ease_out": {"speed": 30, "influence": 60}},
        {"t": 3, "v": 50}]}));
    // only the sides at the continuous key shape the curve; the others keep the default ease, and stay the same
    for i in 0..=30 {
        let t = f64::from(i) / 10.0;
        assert_eq!(p.base(t), q.base(t));
    }
    assert!(scalar_err(json!({"keys": [{"t": 0, "v": 0}, {"t": 1, "v": 1, "continuous": true, "ease_in": {"speed": 1, "influence": 20}, "ease_out": "linear"}]})).contains("continuous"));
}

#[test]
fn a_color_or_vector_with_a_hold_key_holds_every_component() {
    let c = super::ColorProp::parse(&json!({"keys": [{"t": 0, "v": "#ff0000", "ease_out": "hold"}, {"t": 1, "v": "#0000ff"}]})).unwrap();
    assert_eq!(c.at(0.99, &super::testkit::NoEnv).unwrap(), [255.0, 0.0, 0.0, 1.0]);
    assert_eq!(c.at(1.0, &super::testkit::NoEnv).unwrap(), [0.0, 0.0, 255.0, 1.0]);
}

#[test]
fn the_cases_of_the_after_effects_fidelity_run_match_the_reference_solver() {
    // the names are those of EffectCraft's keyframe-ease run against After Effects 26.3 (only its error summary is
    // published, not its samples): each is compared with an independent bisection at 21 samples
    let cases: [Case; 8] = [
        ("easy", 0.0, 100.0, (0.0, 1.0 / 3.0), (0.0, 1.0 / 3.0)),
        ("high_influence", 0.0, 100.0, (0.0, 0.9), (0.0, 0.9)),
        ("asymmetric", 0.0, 100.0, (30.0, 0.2), (-10.0, 0.75)),
        ("overshoot", 0.0, 100.0, (500.0, 0.6), (-200.0, 0.4)),
        ("descending", 100.0, 0.0, (-20.0, 0.5), (-300.0, 0.25)),
        ("equal_endpoints", 50.0, 50.0, (80.0, 0.4), (-80.0, 0.4)),
        ("maximum_influence", 0.0, 100.0, (10.0, 1.0), (10.0, 1.0)),
        ("minimum_maximum", 0.0, 100.0, (10.0, 0.001), (10.0, 1.0)),
    ];
    for (name, a, b, out, into) in cases {
        let p = scalar(json!({"keys": [
            {"t": 0, "v": a, "ease_out": handle(out.0, out.1 * 100.0)},
            {"t": 1, "v": b, "ease_in": handle(into.0, into.1 * 100.0)}]}));
        for i in 0..=20 {
            let x = f64::from(i) / 20.0;
            let want = reference(a, b, 1.0, out, into, x);
            // with both influences at 100 % the time curve is flat to third order at the middle, where bisection
            // can only locate u to 1e-5 (so 3e-4 of the value); the solver under test is exact there (see above)
            let tolerance = if out.1 == 1.0 && into.1 == 1.0 { 1e-3 } else { 1e-7 };
            assert!((p.base(x) - want).abs() < tolerance, "{name} at {x}: {} vs {want}", p.base(x));
        }
    }
}

#[test]
fn influence_33_with_speed_0_is_the_css_cubic_bezier_of_the_same_numbers() {
    // handles (0.33, 0) and (0.67, 1) on a 0..1 segment are cubic(0.33, 0, 0.67, 1)
    let p = scalar(json!({"keys": [{"t": 0, "v": 0, "ease_out": handle(0.0, 33.0)}, {"t": 1, "v": 1, "ease_in": handle(0.0, 33.0)}]}));
    let css = crate::anim::ease::Ease::Cubic(0.33, 0.0, 0.67, 1.0);
    for i in 0..=40 {
        let x = f64::from(i) / 40.0;
        assert!((p.base(x) - css.at(x)).abs() < 1e-12, "x={x}");
    }
}
