//! The kernels in numbers, on the GPU the renderers use, read back as floats: the width a Gaussian really has, whether any
//! light is gained or lost, and whether the picture of a hard edge is the closed-form profile.

use skia_safe::ColorSpace;

use super::{
    rig::{painted, red, Rig, SIGMAS},
    taps, Blur, Edge, Iris, Shape,
};

/// A single unit pixel in the middle of a `side` square, blurred by a Gaussian of `sigma`: the red of every pixel.
fn blurred_impulse(sigma: f64, space: Option<ColorSpace>) -> (Vec<f64>, usize) {
    let side = 2 * taps::gaussian_taps(sigma) as i32 + 9;
    let mut rig = Rig::new();
    let mut like = rig.target(side, side, space);
    let picture = painted(&like, side, side, |x, y| if (x, y) == (side / 2, side / 2) { 1.0 } else { 0.0 });
    let out = Blur::gaussian([sigma; 2], Edge::Transparent).apply(&rig.shaders, like.canvas(), &picture).expect("the blur runs");
    (red(&mut like, &out), side as usize)
}

/// The standard deviation of the light along `line`, about its middle.
fn spread(line: &[f64]) -> f64 {
    let middle = line.len() / 2;
    let (light, second) = line.iter().enumerate().fold((0.0, 0.0), |(l, m), (i, v)| (l + v, m + v * (i as f64 - middle as f64).powi(2)));
    (second / light).sqrt()
}

#[test]
fn an_impulse_blurs_to_the_sigma_asked_for_in_every_size_of_blur() {
    for sigma in SIGMAS {
        let (values, side) = blurred_impulse(sigma, None);
        let across = spread(&values[side / 2 * side..(side / 2 + 1) * side]);
        let down = spread(&(0..side).map(|y| values[y * side + side / 2]).collect::<Vec<_>>());
        for (way, got) in [("across", across), ("down", down)] {
            assert!((got / sigma - 1.0).abs() < 0.03, "sigma {sigma} {way}: measured {got:.4} ({:+.3}%)", 100.0 * (got / sigma - 1.0));
        }
    }
}

#[test]
fn no_light_is_gained_or_lost_in_display_numbers_or_in_linear_light() {
    for space in [None, Some(ColorSpace::new_srgb_linear())] {
        for sigma in SIGMAS {
            let (values, _) = blurred_impulse(sigma, space.clone());
            let energy: f64 = values.iter().sum();
            assert!((energy - 1.0).abs() < 0.005, "sigma {sigma} {}: the picture holds {energy:.5} of the 1.0 it began with", if space.is_some() { "linear" } else { "display" });
        }
    }
}

#[test]
fn the_pixels_of_a_hard_edge_follow_the_closed_form_error_function_within_a_level() {
    // a step at pixel 160 of a 320 px row; pixel `i` holds the share of the kernel on the white side. The kernel is the
    // Gaussian *sampled* at whole pixels (which is what keeps an impulse at the sigma asked for), so it follows the
    // continuous profile to the midpoint rule's error, 0.67 of a level at sigma 2 and falling as 1/sigma^2
    let (w, h, edge) = (320, 4, 160);
    for sigma in [2.0, 4.0, 8.0, 16.0, 32.0] {
        let mut rig = Rig::new();
        let mut like = rig.target(w, h, None);
        let picture = painted(&like, w, h, |x, _| if x >= edge { 1.0 } else { 0.0 });
        let out = Blur::gaussian([sigma, 0.0], Edge::Repeat).apply(&rig.shaders, like.canvas(), &picture).expect("the blur runs");
        let values = red(&mut like, &out);
        let worst = (0..w).map(|x| (values[x as usize] - phi((f64::from(x - edge) + 0.5) / sigma)).abs()).fold(0.0, f64::max);
        assert!(worst * 255.0 < 1.0, "sigma {sigma}: {:.4} of a level from the error function", worst * 255.0);
    }
}

/// The standard normal's distribution function.
pub(super) fn phi(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

/// Abramowitz and Stegun 7.1.26's series is good to 1.5e-7, three orders under what the tests ask.
fn erf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.327_591_1 * x.abs());
    let poly = t * (0.254_829_592 + t * (-0.284_496_736 + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429))));
    (1.0 - poly * (-x * x).exp()).copysign(x)
}

#[test]
fn a_sigma_too_wide_for_the_pass_is_an_error_that_names_the_limit_and_nothing_is_narrowed() {
    let wide = Blur::gaussian([f64::from(taps::MAX_TAPS) / taps::REACH + 1.0, 0.0], Edge::Transparent);
    let e = wide.check().unwrap_err().to_string();
    assert!(e.contains("taps each side") && e.contains(&taps::MAX_TAPS.to_string()), "{e}");
    assert!(Blur::gaussian([f64::NAN, 0.0], Edge::Transparent).check().is_err());
    assert!(Blur::gaussian([-1.0, 0.0], Edge::Transparent).check().is_err());
    let iris = |radius| Blur { shape: Shape::Disc(Iris { radius, blades: 0, rotation: 0.0, roundness: 1.0 }), edge: Edge::Transparent };
    assert!(iris(31.0).check().is_ok());
    assert!(iris(32.0).check().unwrap_err().to_string().contains("lens pass reads at most 32"));
    assert!(Blur { shape: Shape::Box { radius: [3.0; 2], iterations: 0 }, edge: Edge::Repeat }.check().is_err());
}
