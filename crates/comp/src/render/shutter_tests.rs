//! Motion blur with the `motion_blur` key, measured on pixels: how long a smear is, where it sits, what its profile is.
//!
//! The scene is a white 20 px square on black, moving along x at a speed given in px per frame (30 fps), drawn at frame 5.
//! A frame's blur is the scene averaged over the open shutter, so every number follows from the closed form: a shutter of
//! A degrees opened at phase P degrees is open over the frame fractions `[P, P + A] / 360`, and the square's centre is at
//! `100 + speed * frame`. The expected row of pixels is computed here from that and from the exact coverage of a pixel by
//! a box, with no reference to the engine's sampling code.

use serde_json::{json, Value};

use super::testkit::render_at;

pub(super) const W: usize = 800;
pub(super) const H: usize = 60;
pub(super) const SIZE: f64 = 20.0;
/// The frame measured.
pub(super) const FRAME: f64 = 5.0;
/// Where the moving square's left edge starts, px.
const START: f64 = 100.0;
/// One level of an 8-bit channel, as an intensity.
pub(super) const LEVEL: f64 = 1.0 / 255.0;

/// The centre of the square at frame `f` when it moves `speed` px per frame.
pub(super) fn centre(speed: f64, f: f64) -> f64 {
    START + speed * f
}

/// A film with a white square moving `speed` px per frame, the `motion_blur` block `blur`, and the layer's own switch.
pub(super) fn mover(speed: f64, blur: Value, switch: bool) -> Value {
    json!({
        "width": W, "height": H, "fps": 30, "dur": 2, "bg": "#000000", "motion_blur": blur,
        "layers": [{"type": "solid", "id": "square", "color": "#ffffff", "size": [SIZE, SIZE], "mblur": switch,
            "pos": {"from": [START, 30], "to": [START + speed * 30.0, 30], "at": 0, "dur": 1, "ease": "none"}}]
    })
}

/// The red channel along row `y`, 0..1.
pub(super) fn row(film: Value, frame: f64, y: usize) -> Vec<f64> {
    let px = render_at(film, std::path::Path::new("."), frame / 30.0);
    (0..W).map(|x| f64::from(px[(y * W + x) * 4 + 2]) / 255.0).collect()
}

/// The row an average of `weights.len()` boxes of `SIZE` px gives: box `k` centred on `centres[k]`, counted `weights[k]`
/// times in the average of `n`. Each pixel's coverage is exact (the length of its overlap with the box).
pub(super) fn ideal(centres: &[f64], n: usize) -> Vec<f64> {
    (0..W)
        .map(|x| {
            let x = x as f64;
            centres.iter().map(|c| ((x + 1.0).min(c + SIZE / 2.0) - x.max(c - SIZE / 2.0)).max(0.0)).sum::<f64>() / n as f64
        })
        .collect()
}

/// The centres of the `n` samples of a shutter of `angle` degrees opened at `phase`, for a square moving `speed` px per frame
/// that is at `at` when the frame starts: sample `k` is in the middle of the `k`th of `n` equal slices of the open shutter.
pub(super) fn sample_centres(at: f64, speed: f64, angle: f64, phase: f64, n: usize) -> Vec<f64> {
    (0..n).map(|k| at + speed * ((k as f64 + 0.5) / n as f64 * angle + phase) / 360.0).collect()
}

/// The worst difference between two rows, in levels.
pub(super) fn worst(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max) / LEVEL
}

/// What a row of intensities says about a smear.
pub(super) struct Smear {
    /// First and last pixel above zero, and the number of pixels between and including them.
    pub first: usize,
    pub last: usize,
    pub extent: usize,
    /// The intensity-weighted mean position (pixel centres), px.
    pub centroid: f64,
    /// How far the square travelled, from the variance of the intensities: a box of `SIZE` swept over `L` has variance
    /// `(SIZE^2 + L^2) / 12`. Unlike the extent it does not fall short by a sample step, and it needs no threshold.
    pub travel: f64,
}

pub(super) fn smear(row: &[f64]) -> Smear {
    let lit: Vec<usize> = (0..row.len()).filter(|&x| row[x] > 0.0).collect();
    let (first, last) = (lit[0], lit[lit.len() - 1]);
    let mass: f64 = row.iter().sum();
    let centroid = row.iter().enumerate().map(|(x, v)| (x as f64 + 0.5) * v).sum::<f64>() / mass;
    let variance = row.iter().enumerate().map(|(x, v)| (x as f64 + 0.5 - centroid).powi(2) * v).sum::<f64>() / mass;
    Smear { first, last, extent: last - first + 1, centroid, travel: (12.0 * variance - SIZE * SIZE).max(0.0).sqrt() }
}

#[test]
fn a_100_px_per_frame_square_with_a_180_degree_shutter_smears_over_70_px() {
    let r = row(mover(100.0, json!({"shutter_angle": 180}), true), FRAME, 30);
    let s = smear(&r);
    // the closed form: the square's size plus the distance it travels while the shutter is open (100 * 180 / 360 = 50)
    assert!((s.travel + SIZE - 70.0).abs() < 1.5, "length {:.2} against 70", s.travel + SIZE);
    // the outer extent falls short by one sample step: the 16 squares stand 50/16 px apart and the outermost ones are
    // half a step inside the ends of the shutter. Counting whole pixels the two partly covered ones add up to one.
    let step = 50.0 / 16.0;
    assert!((s.extent as f64 - (70.0 - step)).abs() < 1.5, "extent {} against {}", s.extent, 70.0 - step);
    assert!((s.centroid - centre(100.0, FRAME)).abs() < 0.2, "the default phase centres the smear on the frame time: {:.2}", s.centroid);
    let expected = ideal(&sample_centres(centre(100.0, FRAME), 100.0, 180.0, -90.0, 16), 16);
    let off = worst(&r, &expected);
    assert!(off <= 2.0, "the row is the closed form to within {off:.1} levels");
}

#[test]
fn a_360_degree_shutter_smears_over_120_px_and_a_shut_shutter_or_a_switch_that_is_off_leaves_the_sharp_square() {
    let s = smear(&row(mover(100.0, json!({"shutter_angle": 360}), true), FRAME, 30));
    assert!((s.travel + SIZE - 120.0).abs() < 1.5, "{:.2}", s.travel + SIZE);
    for (what, film) in [("angle 0", mover(100.0, json!({"shutter_angle": 0}), true)), ("switch off", mover(100.0, json!({"shutter_angle": 360}), false))] {
        let s = smear(&row(film, FRAME, 30));
        assert_eq!((s.first, s.last, s.extent), (590, 609, 20), "{what}: the square centred on 600 is pixels 590..=609");
        assert!(s.travel < 0.5, "{what}: {:.2}", s.travel);
    }
}

#[test]
fn the_smear_has_a_flat_top_and_straight_ramps_when_the_samples_are_dense_enough() {
    // 8 px of travel (a 28.8 degree shutter at 100 px per frame) in 16 samples half a pixel apart: the 12 pixels in the
    // middle are covered by every sample (full intensity), and each ramp is 8 pixels wide
    let r = row(mover(100.0, json!({"shutter_angle": 28.8, "shutter_phase": -14.4}), true), FRAME, 30);
    for (x, v) in r.iter().enumerate().take(604).skip(596) {
        assert!(*v >= 1.0 - 2.0 * LEVEL, "the flat top at pixel {x}: {v}");
    }
    let off = worst(&r, &ideal(&sample_centres(600.0, 100.0, 28.8, -14.4, 16), 16));
    assert!(off <= 2.0, "the profile is the closed form to within {off:.1} levels");
}

#[test]
fn a_one_pixel_per_frame_mover_shows_no_stepping_with_16_samples() {
    // 0.5 px of travel: the edges' pixels hold fractions of 1/16 the closed form gives; no pixel is further than 2 levels
    let r = row(mover(1.0, json!({"shutter_angle": 180}), true), FRAME, 30);
    let off = worst(&r, &ideal(&sample_centres(centre(1.0, FRAME), 1.0, 180.0, -90.0, 16), 16));
    assert!(off <= 2.0, "within {off:.1} levels of the closed form");
}

#[test]
fn the_phase_slides_the_window_by_its_own_degrees() {
    // the smear's mean position is where the square is at the middle of the open shutter: (phase + angle / 2) / 360 frames on
    for phase in [-180.0, -90.0, 0.0, 90.0, -270.0] {
        let s = smear(&row(mover(100.0, json!({"shutter_angle": 180, "shutter_phase": phase}), true), FRAME, 30));
        let expected = centre(100.0, FRAME) + (phase + 90.0) / 360.0 * 100.0;
        assert!((s.centroid - expected).abs() < 0.2, "phase {phase}: centroid {:.2} against {expected}", s.centroid);
        assert!((s.travel + SIZE - 70.0).abs() < 1.5, "phase {phase}: the length does not depend on the phase: {:.2}", s.travel + SIZE);
    }
}

#[test]
fn the_first_frame_blurs_over_moments_before_time_zero_and_a_layer_is_only_there_from_its_start() {
    // frame 0 averages over [-0.25, 0.25] frames; the layer exists from 0, so 8 of the 16 samples show it, each 1/16
    let r = row(mover(100.0, json!({"shutter_angle": 180}), true), 0.0, 30);
    let expected = ideal(&sample_centres(START, 100.0, 180.0, -90.0, 16)[8..], 16);
    let off = worst(&r, &expected);
    assert!(off <= 2.0, "within {off:.1} levels of the closed form");
    assert!(r.iter().all(|v| *v <= 0.5 + 2.0 * LEVEL), "no pixel is brighter than half the samples");
}

#[test]
fn a_layer_that_enters_during_the_shutter_is_drawn_for_the_moments_it_is_there() {
    // it starts at 5.1 frames: not showing at frame 5, but showing for the samples of [4.75, 5.25] after 5.1. Its motion
    // is relative to its start, so its centre is START + 100 * (frame - 5.1) then.
    let mut film = mover(100.0, json!({"shutter_angle": 180}), true);
    film["layers"][0]["start"] = json!(5.1 / 30.0);
    let r = row(film, FRAME, 30);
    let showing: Vec<f64> = (0..16).map(|k| 4.75 + (f64::from(k) + 0.5) / 16.0 * 0.5).filter(|f| *f >= 5.1).map(|f| START + 100.0 * (f - 5.1)).collect();
    assert_eq!(showing.len(), 5);
    let off = worst(&r, &ideal(&showing, 16));
    assert!(off <= 2.0, "within {off:.1} levels of the closed form");
}

#[test]
fn a_block_with_no_switched_layer_evaluates_no_extra_frames() {
    let plan = crate::plan::Plan::from_value(mover(100.0, json!({"shutter_angle": 180}), false), std::path::Path::new(".")).unwrap();
    let ev = crate::eval::Evaluator::new(&plan);
    assert!(super::graph::shutter::frames(&plan, &ev, 5.0 / 30.0).unwrap().is_empty());
}
