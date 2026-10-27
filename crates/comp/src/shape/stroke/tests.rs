//! The outline of a variable-width stroke, drawn on a raster and measured: the width across the stroke at points along
//! the path against the profile that was asked for, and a constant-width outline against Skia's own stroker.

use skia_safe::{surfaces, ContourMeasureIter, ImageInfo, Paint, PaintCap, PaintJoin, PaintStyle, Path};

use super::{coverage::Coverage, outline, taper::Tapered, Cap, Dim, Join, Profile, Style, Taper, Tip};
use crate::anim::ease::Ease;

const SIDE: i32 = 480;

/// `path` filled (or stroked, with `stroke`) white on a raster `SIDE` across.
fn draw(path: &Path, stroke: Option<(f32, PaintCap, PaintJoin)>) -> Coverage {
    let mut surface = surfaces::raster(&ImageInfo::new_a8((SIDE, SIDE)), None, None).expect("a raster surface");
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    match stroke {
        Some((width, cap, join)) => paint.set_style(PaintStyle::Stroke).set_stroke_width(width).set_stroke_cap(cap).set_stroke_join(join),
        None => paint.set_style(PaintStyle::Fill),
    };
    surface.canvas().draw_path(path, &paint);
    let mut bytes = vec![0u8; (SIDE * SIDE) as usize];
    assert!(surface.read_pixels(&ImageInfo::new_a8((SIDE, SIDE)), &mut bytes, SIDE as usize, (0, 0)));
    Coverage::new(SIDE, SIDE, bytes.into_iter().map(|b| f64::from(b) / 255.0).collect())
}

fn style() -> Style {
    Style { cap: Cap::Butt, join: Join::Miter }
}

fn dim(value: f64) -> Dim {
    Dim { value, fraction: false }
}

fn tip(length: f64, width: f64) -> Option<Tip> {
    Some(Tip { length: dim(length), width: dim(width) })
}

fn ease(name: &str) -> Ease {
    Ease::parse(name).expect("an ease")
}

/// The point and the unit normal `s` pixels along the first contour of `path`, from Skia's own measure.
fn at(path: &Path, s: f64) -> ((f64, f64), (f64, f64)) {
    let contour = ContourMeasureIter::new(path, false, None).next().expect("a contour");
    let (p, t) = contour.pos_tan(s as f32).expect("a point on the contour");
    ((f64::from(p.x), f64::from(p.y)), (-f64::from(t.y), f64::from(t.x)))
}

/// Measure the stroke of `path` under `taper` at the distances `along` and return (measured, expected) pairs. The
/// expected width is written out here, from the numbers of the taper, not read from the profile that drew it.
fn measure(path: &Path, full: f64, taper: &Taper, along: &[f64], expected: impl Fn(f64) -> f64) -> Vec<(f64, f64, f64)> {
    let filled = outline(path, style(), |length| taper.profile(length, full));
    let raster = draw(&filled, None);
    along.iter().map(|&s| (s, raster.width_across(at(path, s).0, at(path, s).1, full), expected(s))).collect()
}

fn assert_within(pairs: &[(f64, f64, f64)], tolerance: f64) {
    for &(s, measured, expected) in pairs {
        assert!((measured - expected).abs() <= tolerance, "at {s}: measured {measured:.3}, expected {expected:.3} ({pairs:?})");
    }
}

fn line() -> Path {
    Path::polygon(&[(40.0, 240.0).into(), (440.0, 240.0).into()], false, None, None)
}

#[test]
fn a_straight_stroke_is_as_wide_as_the_taper_says_at_the_start_the_middle_and_the_end() {
    // 400 long, 20 wide; 100 px from a point at the start, 80 px from a width of 6 at the end
    let taper = Taper { start: tip(100.0, 0.0), end: tip(80.0, 6.0), ease: Ease::Linear };
    let expected = |s: f64| if s < 100.0 { 20.0 * s / 100.0 } else if s > 320.0 { 6.0 + 14.0 * (400.0 - s) / 80.0 } else { 20.0 };
    assert_within(&measure(&line(), 20.0, &taper, &[3.0, 25.0, 50.0, 99.0, 200.0, 321.0, 360.0, 397.0], expected), 0.25);
}

#[test]
fn the_width_follows_the_ease_measured_from_each_tip() {
    for name in ["power2.out", "power3.in", "sine.inOut", "back.out(1.7)"] {
        let e = ease(name);
        let taper = Taper { start: tip(120.0, 2.0), end: tip(120.0, 2.0), ease: e };
        let expected = move |s: f64| {
            let from_tip = s.min(400.0 - s);
            if from_tip < 120.0 { 2.0 + 18.0 * e.at(from_tip / 120.0) } else { 20.0 }
        };
        assert_within(&measure(&line(), 20.0, &taper, &[2.0, 30.0, 60.0, 90.0, 119.0, 200.0, 281.0, 340.0, 398.0], expected), 0.25);
    }
}

#[test]
fn a_curved_stroke_has_the_same_width_across_it_as_along_a_straight_one() {
    let mut curve = Path::new();
    curve.move_to((40.0, 400.0));
    curve.cubic_to((40.0, 60.0), (440.0, 60.0), (440.0, 400.0));
    let length = f64::from(ContourMeasureIter::new(&curve, false, None).next().expect("a contour").length());
    let taper = Taper { start: tip(0.3 * length, 0.0), end: tip(0.2 * length, 4.0), ease: ease("power2.inOut") };
    let e = ease("power2.inOut");
    let expected = move |s: f64| {
        if s < 0.3 * length {
            24.0 * e.at(s / (0.3 * length))
        } else if length - s < 0.2 * length {
            4.0 + 20.0 * e.at((length - s) / (0.2 * length))
        } else {
            24.0
        }
    };
    let along: Vec<f64> = [0.01, 0.1, 0.2, 0.3, 0.5, 0.7, 0.8, 0.9, 0.99].iter().map(|u| u * length).collect();
    assert_within(&measure(&curve, 24.0, &taper, &along, expected), 0.25);
}

#[test]
fn a_taper_longer_than_half_the_path_is_held_to_half() {
    // 400 long: both tapers asked for 1000 px, so each is 200 and the stroke is full width only at the middle
    let taper = Taper { start: tip(1000.0, 0.0), end: tip(1000.0, 0.0), ease: Ease::Linear };
    assert_within(&measure(&line(), 20.0, &taper, &[20.0, 100.0, 200.0, 300.0, 380.0], |s| 20.0 * s.min(400.0 - s) / 200.0), 0.25);
}

#[test]
fn a_closed_path_is_tapered_from_its_start_point_and_pinches_there() {
    let circle = Path::circle((240.0, 240.0), 150.0, None);
    let length = f64::from(ContourMeasureIter::new(&circle, true, None).next().expect("a contour").length());
    let taper = Taper { start: tip(150.0, 0.0), end: tip(100.0, 0.0), ease: Ease::Linear };
    let expected = move |s: f64| if s < 150.0 { 16.0 * s / 150.0 } else if length - s < 100.0 { 16.0 * (length - s) / 100.0 } else { 16.0 };
    let along = [5.0, 50.0, 100.0, 150.0, 400.0, length - 100.0, length - 50.0, length - 5.0];
    assert_within(&measure(&circle, 16.0, &taper, &along, expected), 0.25);
}

#[test]
fn an_outline_of_constant_width_is_what_skia_strokes_for_every_cap_and_join() {
    let mut zigzag = Path::new();
    zigzag.move_to((60.0, 400.0));
    for (x, y) in [(140.0, 80.0), (220.0, 400.0), (300.0, 100.0), (420.0, 380.0)] {
        zigzag.line_to((x, y));
    }
    let mut curve = Path::new();
    curve.move_to((60.0, 300.0));
    curve.quad_to((240.0, 20.0), (420.0, 300.0));
    let star = {
        let mut p = Path::new();
        for k in 0..10 {
            let (r, a) = (if k % 2 == 0 { 200.0 } else { 80.0 }, f64::from(k) * std::f64::consts::PI / 5.0 - 1.0);
            let (x, y) = (240.0 + r * a.cos(), 240.0 + r * a.sin());
            if k == 0 { p.move_to((x as f32, y as f32)) } else { p.line_to((x as f32, y as f32)) };
        }
        p.close();
        p
    };
    let steady = |length: f64| -> Tapered { Taper { start: None, end: None, ease: Ease::Linear }.profile(length, 14.0) };
    for (name, path) in [("zigzag", &zigzag), ("curve", &curve), ("star", &star)] {
        for (cap, skia_cap) in [(Cap::Butt, PaintCap::Butt), (Cap::Round, PaintCap::Round), (Cap::Square, PaintCap::Square)] {
            for (join, skia_join) in [(Join::Miter, PaintJoin::Miter), (Join::Round, PaintJoin::Round), (Join::Bevel, PaintJoin::Bevel)] {
                let ours = draw(&outline(path, Style { cap, join }, steady), None);
                let theirs = draw(path, Some((14.0, skia_cap, skia_join)));
                let worst = ours.values.iter().zip(&theirs.values).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
                let area = |r: &Coverage| r.values.iter().sum::<f64>();
                assert!(worst <= 0.35, "{name} {cap:?} {join:?}: a pixel differs by {worst}");
                assert!((area(&ours) - area(&theirs)).abs() <= 0.005 * area(&theirs), "{name} {cap:?} {join:?}: area {} against {}", area(&ours), area(&theirs));
            }
        }
    }
}

#[test]
fn the_area_of_a_constant_width_stroke_is_its_width_times_the_length_of_the_path() {
    // a quadratic's length from a fine polyline, written out here
    let (p0, p1, p2) = ((60.0f64, 300.0f64), (240.0f64, 20.0f64), (420.0f64, 300.0f64));
    let point = |t: f64| (((1.0 - t).powi(2) * p0.0 + 2.0 * (1.0 - t) * t * p1.0 + t * t * p2.0), ((1.0 - t).powi(2) * p0.1 + 2.0 * (1.0 - t) * t * p1.1 + t * t * p2.1));
    let length: f64 = (0..100_000).map(|k| (point(f64::from(k) / 1e5), point(f64::from(k + 1) / 1e5))).map(|(a, b)| ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()).sum();
    let mut curve = Path::new();
    curve.move_to((60.0, 300.0));
    curve.quad_to((240.0, 20.0), (420.0, 300.0));
    let ours = draw(&outline(&curve, style(), |l| Taper { start: None, end: None, ease: Ease::Linear }.profile(l, 14.0)), None);
    let area: f64 = ours.values.iter().sum();
    assert!((area - 14.0 * length).abs() <= 0.002 * 14.0 * length, "area {area} against {}", 14.0 * length);
}

#[test]
fn a_hairpin_bend_leaves_no_hole_where_the_two_sides_overlap() {
    let hairpin = Path::polygon(&[(60.0, 100.0).into(), (400.0, 240.0).into(), (60.0, 380.0).into()], false, None, None);
    let ours = draw(&outline(&hairpin, Style { cap: Cap::Butt, join: Join::Round }, |l| Taper { start: None, end: None, ease: Ease::Linear }.profile(l, 60.0)), None);
    // on the centre line of both legs, well inside the stroke, coverage is full
    for u in [0.1, 0.4, 0.6, 0.9] {
        let (x, y) = (60.0 + 340.0 * u, 100.0 + 140.0 * u);
        assert!(ours.sample(x, y) > 0.99, "a hole at ({x}, {y}): {}", ours.sample(x, y));
        assert!(ours.sample(x, 480.0 - y) > 0.99);
    }
}

#[test]
fn a_profile_that_is_not_a_taper_draws_through_the_same_walk() {
    struct Swell;
    impl Profile for Swell {
        fn width(&self, s: f64) -> f64 {
            10.0 + s / 20.0
        }
    }
    let swelling = draw(&outline(&line(), style(), |_| Swell), None);
    let (p, n) = at(&line(), 200.0);
    assert!((swelling.width_across(p, n, 30.0) - 20.0).abs() <= 0.25);
}
