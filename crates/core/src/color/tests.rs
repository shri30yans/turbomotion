//! Colour management on Skia's CPU raster: numbers first. The renderers use the GPU, which is checked end to end in
//! `comp`; here the conversions themselves are held to the published curves and matrices. The raster pipeline
//! evaluates the curve's power with a fast approximation, so the bounds below are those of that approximation: at most
//! `CURVE` in linear light, which is under a twentieth of one 8-bit display level anywhere on the curve.
const CURVE: f64 = 3e-4;

use skia_safe::{surfaces, AlphaType, Color, Color4f, ColorSpace, ColorType, ISize, ImageInfo, Paint, Surface};

use super::{Pipeline, Primaries, Profile, Transfer};

fn linear() -> Profile {
    Profile { primaries: Primaries::Srgb, transfer: Transfer::Linear }
}

/// A float surface in `space`: nothing is clamped or rounded, so what is read back is what the conversion produced.
fn float_surface(w: i32, space: ColorSpace) -> Surface {
    let info = ImageInfo::new(ISize::new(w, 1), ColorType::RGBAF32, AlphaType::Premul, space);
    surfaces::raster(&info, None, None).expect("a float surface")
}

/// The pixels as straight (not premultiplied) floats in the surface's own space.
fn read(s: &mut Surface) -> Vec<[f32; 4]> {
    let info = ImageInfo::new(s.image_info().dimensions(), ColorType::RGBAF32, AlphaType::Unpremul, s.image_info().color_space());
    let mut bytes = vec![0u8; info.compute_min_byte_size()];
    assert!(s.read_pixels(&info, &mut bytes, info.min_row_bytes(), (0, 0)));
    bytes.chunks_exact(16).map(|px| std::array::from_fn(|c| f32::from_ne_bytes(px[c * 4..c * 4 + 4].try_into().unwrap()))).collect()
}

/// The sRGB transfer function (IEC 61966-2-1), from its definition.
pub(super) fn srgb_to_linear(v: f64) -> f64 {
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

fn linear_to_srgb(v: f64) -> f64 {
    if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
}

#[test]
fn the_default_pipeline_tags_nothing_and_converts_nothing() {
    let p = Pipeline::default();
    assert!(!p.is_managed() && !p.converts_at_output());
    assert!(p.working_space().is_none() && p.output_space().is_none());
}

#[test]
fn linear_light_keeps_pictures_in_light_and_converts_when_the_frame_leaves() {
    let p = Pipeline { working: linear(), output: Profile::SRGB };
    assert!(p.is_managed() && p.converts_at_output());
    assert_eq!(p.working_space(), Some(linear().space()));
    assert_eq!(p.output_space(), Some(Profile::SRGB.space()));
    let wide = Pipeline { working: Profile { primaries: Primaries::DisplayP3, transfer: Transfer::Srgb }, output: Profile { primaries: Primaries::DisplayP3, transfer: Transfer::Srgb } };
    assert!(wide.is_managed() && !wide.converts_at_output(), "a P3 film kept and delivered in P3 converts nothing at the end");
}

#[test]
fn every_profile_has_a_colour_space() {
    for primaries in [Primaries::Srgb, Primaries::DisplayP3, Primaries::Rec2020, Primaries::Bt470Bg, Primaries::Smpte170m] {
        for transfer in [Transfer::Srgb, Transfer::Linear] {
            let space = Profile { primaries, transfer }.space();
            assert_eq!(space.is_srgb(), primaries == Primaries::Srgb && transfer == Transfer::Srgb, "{primaries:?} {transfer:?}");
        }
    }
}

#[test]
fn srgb_to_linear_and_back_is_exact_at_all_256_levels() {
    let mut linear_side = float_surface(256, linear().space());
    let mut src = float_surface(256, Profile::SRGB.space());
    for level in 0..256 {
        let mut paint = Paint::new(Color4f::from(Color::from_rgb(level as u8, level as u8, level as u8)), &Profile::SRGB.space());
        paint.set_anti_alias(false);
        src.canvas().draw_rect(skia_safe::Rect::from_xywh(level as f32, 0.0, 1.0, 1.0), &paint);
    }
    linear_side.canvas().draw_image(src.image_snapshot(), (0, 0), None);
    let in_light = read(&mut linear_side);
    let mut back = float_surface(256, Profile::SRGB.space());
    back.canvas().draw_image(linear_side.image_snapshot(), (0, 0), None);
    let restored = read(&mut back);
    let (mut worst_light, mut worst_back) = (0.0f64, 0.0f64);
    for level in 0..256 {
        let want = srgb_to_linear(level as f64 / 255.0);
        worst_light = worst_light.max((f64::from(in_light[level][0]) - want).abs());
        worst_back = worst_back.max((f64::from(restored[level][0]) * 255.0 - level as f64).abs());
    }
    assert!(worst_light < CURVE, "linear values are off the IEC 61966-2-1 curve by {worst_light}");
    assert!(worst_back < 0.05, "the round trip moved a level by {worst_back} (of 255)");
}

#[test]
fn half_white_over_black_in_light_shows_as_188_where_encoded_blending_shows_128() {
    let mut light = float_surface(1, linear().space());
    light.canvas().clear(Color::BLACK);
    light.canvas().draw_paint(&Paint::new(Color4f::new(1.0, 1.0, 1.0, 0.5), &Profile::SRGB.space()));
    let mut shown = float_surface(1, Profile::SRGB.space());
    shown.canvas().draw_image(light.image_snapshot(), (0, 0), None);
    let in_light = read(&mut light)[0];
    // a white at 50% in the light-linear space: alpha is coverage, so the light is 0.5 and the display value is its encoding
    assert!((in_light[0] - 0.5).abs() < 1e-6, "linear value {}", in_light[0]);
    let shown = read(&mut shown)[0][0];
    let want = linear_to_srgb(0.5);
    assert!((f64::from(shown) - want).abs() < CURVE, "{shown} vs {want}");
    assert_eq!((want * 255.0).round() as u8, 188, "0.5 of the light is level 188 on the display");
    let mut encoded = float_surface(1, Profile::SRGB.space());
    encoded.canvas().clear(Color::BLACK);
    encoded.canvas().draw_paint(&Paint::new(Color4f::new(1.0, 1.0, 1.0, 0.5), &Profile::SRGB.space()));
    assert_eq!((read(&mut encoded)[0][0] * 255.0).round() as u8, 128, "blending the encoded numbers gives 128, a visibly darker half");
}

#[test]
fn premultiplication_is_undone_before_a_colour_is_converted() {
    let mut light = float_surface(1, linear().space());
    light.canvas().draw_paint(&Paint::new(Color4f::new(200.0 / 255.0, 100.0 / 255.0, 50.0 / 255.0, 0.5), &Profile::SRGB.space()));
    let px = read(&mut light)[0];
    assert!((px[3] - 0.5).abs() < 1e-6, "alpha is coverage and is never encoded: {}", px[3]);
    for (got, level) in px[..3].iter().zip([200.0, 100.0, 50.0]) {
        let want = srgb_to_linear(level / 255.0);
        assert!((f64::from(*got) - want).abs() < CURVE, "straight colour {got} vs {want}");
    }
    let mut shown = float_surface(1, Profile::SRGB.space());
    shown.canvas().draw_image(light.image_snapshot(), (0, 0), None);
    let back = read(&mut shown)[0];
    for (got, level) in back[..3].iter().zip([200.0, 100.0, 50.0]) {
        assert!((f64::from(*got) * 255.0 - level).abs() < 0.05, "round trip {} vs {level}", f64::from(*got) * 255.0);
    }
    assert!((back[3] - 0.5).abs() < 1e-6);
}

#[test]
fn display_p3_red_lands_outside_srgb_where_the_published_matrix_puts_it() {
    let mut p3 = float_surface(1, Profile { primaries: Primaries::DisplayP3, transfer: Transfer::Srgb }.space());
    p3.canvas().draw_paint(&Paint::new(Color4f::new(1.0, 0.0, 0.0, 1.0), &Profile { primaries: Primaries::DisplayP3, transfer: Transfer::Srgb }.space()));
    let mut srgb = float_surface(1, Profile::SRGB.space());
    srgb.canvas().draw_image(p3.image_snapshot(), (0, 0), None);
    let px = read(&mut srgb)[0];
    // Display P3 red as linear sRGB: the first column of the P3 -> sRGB matrix (Lindbloom / CSS Color 4), then encoded
    // with the sign-mirrored sRGB curve
    let want = [1.224_940, -0.042_056, -0.019_638].map(|c: f64| c.signum() * linear_to_srgb(c.abs()));
    for ((got, want), name) in px.iter().zip(want).zip(["r", "g", "b"]) {
        assert!((f64::from(*got) - want).abs() < 2e-3, "{name}: {got} vs {want}");
    }
    assert!(px[0] > 1.0 && px[1] < 0.0, "the colour is out of sRGB's range, not clipped: {px:?}");
}
