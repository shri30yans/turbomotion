//! Box, iris and smear kernels, the edge behaviours and the gradient, measured on pictures read back as floats.

use super::{
    rig::{painted, red, Rig},
    Blur, Edge, Iris, Shape, Smear,
};
use crate::layer::Layer;

const SIDE: i32 = 129;
const MIDDLE: i32 = SIDE / 2;

/// A unit pixel in the middle of a `SIDE` square, put through `blur`: the red of every pixel, and the distance of each
/// from the middle pixel.
fn impulse(blur: impl Fn(&Rig, &skia_safe::Canvas, &Layer) -> Layer) -> Vec<(f64, f64)> {
    let mut rig = Rig::new();
    let mut like = rig.target(SIDE, SIDE, None);
    let picture = painted(&like, SIDE, SIDE, |x, y| if (x, y) == (MIDDLE, MIDDLE) { 1.0 } else { 0.0 });
    let out = blur(&rig, like.canvas(), &picture);
    red(&mut like, &out).into_iter().enumerate().map(|(i, v)| (v, f64::from(i as i32 % SIDE - MIDDLE).hypot(f64::from(i as i32 / SIDE - MIDDLE)))).collect()
}

fn run(blur: Blur) -> Vec<(f64, f64)> {
    impulse(|rig, like, picture| blur.apply(&rig.shaders, like, picture).expect("the blur runs"))
}

fn at(pixels: &[(f64, f64)], dx: i32, dy: i32) -> f64 {
    pixels[((MIDDLE + dy) * SIDE + MIDDLE + dx) as usize].0
}

fn energy(pixels: &[(f64, f64)]) -> f64 {
    pixels.iter().map(|p| p.0).sum()
}

fn boxed(radius: [f64; 2], iterations: u32) -> Blur {
    Blur { shape: Shape::Box { radius, iterations }, edge: Edge::Transparent }
}

#[test]
fn a_box_is_flat_over_exactly_its_width_and_keeps_its_light() {
    let pixels = run(boxed([5.0, 0.0], 1));
    for d in -5..=5 {
        assert!((at(&pixels, d, 0) - 1.0 / 11.0).abs() < 1e-4, "{d}: {}", at(&pixels, d, 0));
    }
    assert_eq!([at(&pixels, 6, 0), at(&pixels, -6, 0), at(&pixels, 0, 1)], [0.0; 3], "the support is the radius, along the axis only");
    assert!((energy(&pixels) - 1.0).abs() < 0.005);
    // a fractional radius ends in a lighter pixel, so the width is continuous in the radius
    let pixels = run(boxed([2.5, 0.0], 1));
    assert!((at(&pixels, 2, 0) - 1.0 / 6.0).abs() < 1e-4 && (at(&pixels, 3, 0) - 0.5 / 6.0).abs() < 1e-4, "{} {}", at(&pixels, 2, 0), at(&pixels, 3, 0));
}

#[test]
fn repeated_boxes_widen_as_the_variances_add() {
    // a box of radius r has variance r(r + 1) / 3 along its axis; n passes add n of them
    let (r, n) = (4.0, 3);
    let pixels = run(boxed([r, 0.0], n));
    let second: f64 = (-MIDDLE..=MIDDLE).map(|d| at(&pixels, d, 0) * f64::from(d * d)).sum();
    let want = f64::from(n) * r * (r + 1.0) / 3.0;
    assert!((second / want - 1.0).abs() < 0.01, "{second} against {want}");
    assert!((energy(&pixels) - 1.0).abs() < 0.005);
    assert_eq!(at(&pixels, 3 * 4 + 1, 0), 0.0, "n boxes reach n times the radius");
}

fn iris(radius: f64, blades: u32, roundness: f64) -> Blur {
    Blur { shape: Shape::Disc(Iris { radius, blades, rotation: 0.0, roundness }), edge: Edge::Transparent }
}

#[test]
fn a_disc_is_flat_topped_inside_its_radius_and_nothing_beyond_half_a_pixel_past_it() {
    let radius = 12.0;
    let pixels = run(iris(radius, 0, 1.0));
    let area = std::f64::consts::PI * radius * radius;
    assert!((energy(&pixels) - 1.0).abs() < 0.005, "{}", energy(&pixels));
    let plateau: Vec<f64> = pixels.iter().filter(|p| p.1 <= radius - 0.5).map(|p| p.0).collect();
    assert!(plateau.iter().all(|v| (v - plateau[0]).abs() < 0.005 * plateau[0]), "flat to half a percent");
    assert!((plateau[0] * area - 1.0).abs() < 0.02, "the height is one over the disc's area: {}", plateau[0] * area);
    assert!(pixels.iter().filter(|p| p.1 > radius + 0.5).all(|p| p.0 == 0.0), "no weight past the radius and the edge's half pixel");
    assert!(pixels.iter().any(|p| p.1 > radius - 0.5 && p.1 <= radius + 0.5 && p.0 > 0.0), "the edge is soft over one pixel");
}

#[test]
fn a_polygon_iris_has_its_corners_at_the_radius_and_its_sides_nearer() {
    let radius = 12.0;
    let hexagon = run(iris(radius, 6, 0.0));
    let area = 3.0 * 3.0_f64.sqrt() / 2.0 * radius * radius;
    assert!((energy(&hexagon) - 1.0).abs() < 0.005);
    let plateau = at(&hexagon, 0, 0);
    assert!((plateau * area - 1.0).abs() < 0.02, "the height is one over the hexagon's area: {}", plateau * area);
    // a flat side faces +x (rotation 0): its middle is 12 cos 30 = 10.39 away, while a corner 30 degrees round is 12 away
    assert_eq!(at(&hexagon, 11, 0), 0.0, "past the side's middle");
    assert!(at(&hexagon, 10, 0) > 0.0);
    let corner = (f64::from(radius as i32 - 1) * 30.0_f64.to_radians().cos()).round() as i32;
    assert!(at(&hexagon, corner, (f64::from(radius as i32 - 1) * 0.5).round() as i32) > 0.0, "the corner reaches nearly the radius");
    assert!(hexagon.iter().all(|p| p.1 <= radius + 0.5 || p.0 == 0.0), "no corner passes the radius");
    // roundness 1 rounds every corner away: the polygon is the disc
    assert_eq!(run(iris(radius, 6, 1.0)).iter().map(|p| p.0).collect::<Vec<_>>(), run(iris(radius, 0, 1.0)).iter().map(|p| p.0).collect::<Vec<_>>());
}

fn white(blur: Blur) -> Vec<f64> {
    let mut rig = Rig::new();
    let mut like = rig.target(64, 64, None);
    let picture = painted(&like, 64, 64, |_, _| 1.0);
    let out = blur.apply(&rig.shaders, like.canvas(), &picture).expect("the blur runs");
    red(&mut like, &out)
}

#[test]
fn a_transparent_edge_loses_the_light_that_leaves_and_a_repeated_edge_keeps_the_picture() {
    let fade = white(Blur::gaussian([4.0, 0.0], Edge::Transparent));
    assert!((fade[32 * 64] - 0.5).abs() < 0.1, "half the kernel is outside at the first pixel: {}", fade[32 * 64]);
    assert!((fade[32 * 64 + 32] - 1.0).abs() < 1e-3, "the middle is untouched");
    let kept = white(Blur::gaussian([4.0, 4.0], Edge::Repeat));
    assert!(kept.iter().all(|v| (v - 1.0).abs() < 1e-3), "a flat picture stays flat all the way to its corners");
    let iris = white(Blur { shape: Shape::Disc(Iris { radius: 6.0, blades: 0, rotation: 0.0, roundness: 1.0 }), edge: Edge::Repeat });
    assert!(iris.iter().all(|v| (v - 1.0).abs() < 1e-3));
}

#[test]
fn a_smooth_gradient_stays_monotone_and_no_step_is_larger_than_the_ideal() {
    let width = 256;
    for sigma in [4.0, 16.0, 64.0] {
        let mut rig = Rig::new();
        let mut like = rig.target(width, 2, None);
        let picture = painted(&like, width, 2, |x, _| x as f32 / 255.0);
        let out = Blur::gaussian([sigma, 0.0], Edge::Repeat).apply(&rig.shaders, like.canvas(), &picture).expect("the blur runs");
        let row = &red(&mut like, &out)[..width as usize];
        let noise = 2.0_f64.powi(-10) / 2.0; // half a half-float step just under one
        let steps: Vec<f64> = row.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(steps.iter().all(|s| *s >= -noise), "sigma {sigma}: the ramp falls somewhere");
        assert!(steps.iter().all(|s| *s <= 1.0 / 255.0 + noise), "sigma {sigma}: a step of {:.5} against the ideal {:.5}", steps.iter().cloned().fold(0.0, f64::max), 1.0 / 255.0);
    }
}

#[test]
fn a_line_smear_is_a_box_the_length_of_the_path() {
    let smeared = impulse(|rig, like, picture| Smear::line(20.0, 0.0).apply(&rig.shaders, like, picture).expect("the smear runs"));
    assert!((energy(&smeared) - 1.0).abs() < 0.005, "{}", energy(&smeared));
    assert_eq!([at(&smeared, 11, 0), at(&smeared, -11, 0), at(&smeared, 0, 1)], [0.0; 3], "the path is 20 px long, along x");
    let top: Vec<f64> = (-9..=9).map(|d| at(&smeared, d, 0)).collect();
    assert!(top.iter().all(|v| (v - 0.05).abs() < 0.005), "flat at one over the length: {top:?}");
}

/// A unit pixel `OFFSET` right of the middle, smeared: each pixel's red, its distance from the smear's centre and the angle
/// it is at from there (degrees, 0 along +x).
fn orbit(smear: Smear) -> Vec<(f64, f64, f64)> {
    let mut rig = Rig::new();
    let mut like = rig.target(SIDE, SIDE, None);
    let picture = painted(&like, SIDE, SIDE, |x, y| if (x, y) == (MIDDLE + OFFSET, MIDDLE) { 1.0 } else { 0.0 });
    let out = smear.apply(&rig.shaders, like.canvas(), &picture).expect("the smear runs");
    red(&mut like, &out)
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            let (dx, dy) = (f64::from(i as i32 % SIDE - MIDDLE), f64::from(i as i32 / SIDE - MIDDLE));
            (v, dx.hypot(dy), dy.atan2(dx).to_degrees())
        })
        .collect()
}

const OFFSET: i32 = 40;

#[test]
fn a_spin_moves_a_point_along_its_circle_and_a_zoom_along_its_ray() {
    // the smear's centre is the middle pixel's centre, so a pixel `OFFSET` right of it is `OFFSET` away at angle 0
    let centre = [f64::from(MIDDLE) + 0.5; 2];
    let radius = f64::from(OFFSET);
    let spun = orbit(Smear::spin(centre, 36.0));
    let lit: Vec<&(f64, f64, f64)> = spun.iter().filter(|p| p.0 > 1e-4).collect();
    assert!(lit.len() > 20 && lit.iter().all(|p| (p.1 - radius).abs() < 1.5 && p.2.abs() < 18.0 + 1.5), "an arc of radius {radius} over 36 degrees");
    assert!((spun.iter().map(|p| p.0).sum::<f64>() - 1.0).abs() < 0.02, "a turn keeps the light");
    assert!(lit.iter().any(|p| p.2 > 15.0) && lit.iter().any(|p| p.2 < -15.0), "the arc is centred on the point and reaches both ends");
    let zoomed = orbit(Smear::zoom(centre, 0.1));
    let lit: Vec<&(f64, f64, f64)> = zoomed.iter().filter(|p| p.0 > 1e-4).collect();
    assert!(lit.iter().all(|p| p.1 > radius - 1.5 && p.1 < radius * 1.1 + 1.5 && p.2.abs() < 1.5), "a ray from {radius} to {}", radius * 1.1);
    assert!(lit.iter().any(|p| p.1 > radius * 1.1 - 1.5), "and it reaches the far end");
}
