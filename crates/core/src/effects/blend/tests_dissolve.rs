//! The dissolves: the share of pixels kept is the layer's alpha, whole pixels are kept or dropped, the pattern is a
//! function of the pixel and the seed alone, and it looks like noise (no stripes, no correlation with a neighbour or with
//! the next frame). Measured on a large flat area, on the CPU raster; the pattern is made of exact whole numbers, so the
//! GPU draws the very same one (`comp` checks a dissolve end to end).

use skia_safe::{surfaces, AlphaType, Color, Color4f, ColorType, ISize, ImageInfo, Paint};

use super::{
    tests::{programs, read},
    Blend,
};
use crate::layer::Layer;

const SIDE: i32 = 512;

/// Which pixels of a `SIDE` square a dissolve of an opaque white layer keeps over opaque black, row by row.
fn kept(blend: Blend, opacity: f32, frame: u32) -> Vec<bool> {
    let info = ImageInfo::new(ISize::new(SIDE, SIDE), ColorType::RGBAF32, AlphaType::Premul, None);
    let mut below = surfaces::raster(&info, None, None).expect("a surface");
    below.canvas().clear(Color::BLACK);
    let layer = Layer::render(below.canvas(), SIDE, SIDE, |c| {
        c.clear(Color::WHITE);
    })
    .expect("a layer");
    let mut paint = Paint::default();
    paint.set_alpha_f(opacity);
    let picture = blend.stage(&programs(), below.canvas(), &layer, &mut paint, frame).expect("staged");
    below.canvas().draw_image(picture.image(), (0, 0), Some(&paint));
    read(&mut below).iter().map(|p| p[0] > 0.5).collect()
}

fn share(pixels: &[bool]) -> f64 {
    pixels.iter().filter(|k| **k).count() as f64 / pixels.len() as f64
}

/// How closely a share of a flat area must match the chance it is meant to have. The kept pixels are independent draws
/// at that chance, so over `SIDE`^2 = 262144 of them the share has a standard deviation of at most 0.001
/// (sqrt(0.25 / 262144)); four and a half of them is 0.0045.
const SHARE: f64 = 0.0045;

#[test]
fn the_share_of_pixels_kept_is_the_opacity() {
    for opacity in [0.05, 0.3, 0.5, 0.75, 0.95] {
        for blend in [Blend::Dissolve, Blend::DancingDissolve] {
            let got = share(&kept(blend, opacity, 7));
            assert!((got - f64::from(opacity)).abs() < SHARE, "{blend:?} at opacity {opacity}: {got:.4} of the pixels are kept");
        }
    }
    assert_eq!(share(&kept(Blend::Dissolve, 1.0, 0)), 1.0, "an opaque layer is kept whole");
    assert_eq!(share(&kept(Blend::Dissolve, 0.0, 0)), 0.0, "a transparent one is dropped whole");
}

#[test]
fn the_share_is_the_same_in_every_part_of_the_area() {
    let pixels = kept(Blend::Dissolve, 0.3, 0);
    let n = SIDE as usize / 4;
    for (qx, qy) in (0..4).flat_map(|x| (0..4).map(move |y| (x, y))) {
        let block: Vec<bool> = (0..n).flat_map(|y| (0..n).map(move |x| (qx * n + x, qy * n + y))).map(|(x, y)| pixels[y * SIDE as usize + x]).collect();
        // 128 x 128 = 16384 pixels: a standard deviation of 0.0036 at most; four and a half of them is 0.016
        assert!((share(&block) - 0.3).abs() < 0.016, "block {qx},{qy}: {:.4}", share(&block));
    }
}

#[test]
fn a_pixel_is_the_layer_whole_or_what_is_below_whole() {
    let info = ImageInfo::new(ISize::new(SIDE, 1), ColorType::RGBAF32, AlphaType::Premul, None);
    let mut below = surfaces::raster(&info, None, None).expect("a surface");
    below.canvas().clear(Color4f::new(0.1, 0.2, 0.3, 1.0));
    // a layer that is itself half transparent, drawn at half opacity: kept a quarter of the time, and then as its own
    // straight colour at full strength, not darkened by the alpha that decided it
    let layer = Layer::render(below.canvas(), SIDE, 1, |c| {
        c.clear(Color4f::new(0.8, 0.6, 0.4, 0.5));
    })
    .expect("a layer");
    let mut paint = Paint::default();
    paint.set_alpha_f(0.5);
    let picture = Blend::Dissolve.stage(&programs(), below.canvas(), &layer, &mut paint, 0).expect("staged");
    below.canvas().draw_image(picture.image(), (0, 0), Some(&paint));
    let pixels = read(&mut below);
    let (mut kept, mut dropped) = (0, 0);
    for p in &pixels {
        if (p[0] - 0.8).abs() < 2e-3 && (p[1] - 0.6).abs() < 2e-3 && (p[2] - 0.4).abs() < 2e-3 && p[3] == 1.0 {
            kept += 1;
        } else if (p[0] - 0.1).abs() < 1e-6 && (p[1] - 0.2).abs() < 1e-6 && (p[2] - 0.3).abs() < 1e-6 && p[3] == 1.0 {
            dropped += 1;
        }
    }
    assert_eq!(kept + dropped, SIDE, "every pixel is one or the other");
    assert!((f64::from(kept) / f64::from(SIDE) - 0.25).abs() < 0.07, "alpha 0.5 at opacity 0.5 keeps a quarter: {kept} of {SIDE}");
}

#[test]
fn the_pattern_depends_on_the_pixel_and_the_seed_alone() {
    let again = kept(Blend::Dissolve, 0.4, 0);
    assert_eq!(kept(Blend::Dissolve, 0.4, 0), again, "two renders agree bit for bit");
    assert_eq!(kept(Blend::Dissolve, 0.4, 91), again, "a plain dissolve ignores the frame");
    assert_eq!(kept(Blend::DancingDissolve, 0.4, 5), kept(Blend::DancingDissolve, 0.4, 5), "a dancing one repeats for the same frame");
    assert_ne!(kept(Blend::DancingDissolve, 0.4, 5), kept(Blend::DancingDissolve, 0.4, 6), "and changes with the frame");
    assert_eq!(kept(Blend::DancingDissolve, 0.4, 0), again, "frame 0 of a dancing dissolve is the plain pattern");
}

/// The share of pixels on which two patterns agree: a half when they are unrelated, 1 when they are the same.
fn agreement(a: &[bool], b: &[bool]) -> f64 {
    a.iter().zip(b).filter(|(x, y)| x == y).count() as f64 / a.len() as f64
}

#[test]
fn the_pattern_is_noise_neighbours_and_frames_are_unrelated() {
    let side = SIDE as usize;
    let here = kept(Blend::DancingDissolve, 0.5, 100);
    // at opacity 0.5 two unrelated pixels agree half the time: standard deviation 0.001, so 0.0045 is four and a half
    let tolerance = 0.0045;
    let shifted = |dx: usize, dy: usize| -> (Vec<bool>, Vec<bool>) {
        let (mut a, mut b) = (vec![], vec![]);
        for y in 0..side - dy {
            for x in 0..side - dx {
                a.push(here[y * side + x]);
                b.push(here[(y + dy) * side + x + dx]);
            }
        }
        (a, b)
    };
    for (dx, dy) in [(1, 0), (0, 1), (1, 1), (2, 0), (0, 2), (3, 5), (8, 0), (0, 8), (64, 0), (0, 64)] {
        let (a, b) = shifted(dx, dy);
        assert!((agreement(&a, &b) - 0.5).abs() < tolerance, "pixels {dx},{dy} apart agree {:.4} of the time", agreement(&a, &b));
    }
    for other in [101, 102, 1000, 70_000] {
        let next = kept(Blend::DancingDissolve, 0.5, other);
        assert!((agreement(&here, &next) - 0.5).abs() < tolerance, "frame 100 and {other} agree {:.4} of the time", agreement(&here, &next));
    }
}

#[test]
fn the_pattern_is_flat_across_its_whole_range() {
    // the kept share at 16 opacities is the share of the pattern below each: a uniform pattern gives i / 16
    for i in 1..16 {
        let opacity = i as f32 / 16.0;
        let got = share(&kept(Blend::Dissolve, opacity, 0));
        assert!((got - f64::from(opacity)).abs() < SHARE, "below {opacity}: {got:.4}");
    }
}
