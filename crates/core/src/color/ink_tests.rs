//! Flat colours on the GPU, in numbers: a dark colour painted into a linear-light picture arrives as the light the sRGB
//! curve says, at every one of the 256 levels, and nothing is changed in a picture that is not tagged.

use skia_safe::{gpu::{self, Budgeted, SurfaceOrigin}, AlphaType, Color4f, ColorSpace, ColorType, ISize, Paint, Rect, Surface};

use super::{tests::srgb_to_linear, Ink, Primaries, Profile, Transfer};
use crate::gpu::Gpu;

fn surface(gpu: &mut Gpu, color_type: ColorType, space: Option<ColorSpace>) -> Surface {
    let info = skia_safe::ImageInfo::new(ISize::new(16, 16), color_type, AlphaType::Premul, space);
    gpu::surfaces::render_target(&mut gpu.context, Budgeted::Yes, &info, None, SurfaceOrigin::TopLeft, None, false, false).expect("a render target")
}

/// What a half-float can be off by: it keeps 11 significant bits, so rounding is at most 2^-11 of the value; twice that
/// leaves room for the conversion's own rounding.
const HALF_FLOAT_RELATIVE_ERROR: f64 = 2.0 / 2048.0;

/// The premultiplied floats at pixel (8, 8), in the surface's own space (nothing is converted on the way out).
fn read(s: &mut Surface) -> [f32; 4] {
    let info = skia_safe::ImageInfo::new(ISize::new(1, 1), ColorType::RGBAF32, AlphaType::Premul, s.image_info().color_space());
    let mut bytes = [0u8; 16];
    assert!(s.read_pixels(&info, &mut bytes, 16, (8, 8)));
    std::array::from_fn(|i| f32::from_ne_bytes(bytes[i * 4..i * 4 + 4].try_into().expect("four bytes")))
}

/// `color` painted over an area much smaller than the picture, antialiased as a solid or a shape is.
fn paint_small(ink: &Ink, s: &mut Surface, color: Color4f) {
    s.canvas().clear(Color4f::new(0.0, 0.0, 0.0, 0.0));
    let mut paint = Paint::default();
    ink.apply(&mut paint, s.canvas(), color).expect("an ink paint");
    paint.set_anti_alias(true);
    s.canvas().draw_rect(Rect::from_xywh(4.0, 4.0, 8.0, 8.0), &paint);
}

#[test]
fn every_level_painted_into_linear_light_arrives_as_the_light_the_curve_says() {
    let mut gpu = Gpu::new().expect("a GPU");
    let ink = Ink::compile().expect("the ink program");
    let mut s = surface(&mut gpu, ColorType::RGBAF16, Some(Profile { primaries: Primaries::Srgb, transfer: Transfer::Linear }.space()));
    for level in 0..=255u32 {
        let v = level as f32 / 255.0;
        paint_small(&ink, &mut s, Color4f::new(v, v, v, 1.0));
        let want = srgb_to_linear(f64::from(v));
        for got in &read(&mut s)[..3] {
            assert!((f64::from(*got) - want).abs() <= want * HALF_FLOAT_RELATIVE_ERROR, "level {level}: light {got} against {want}");
        }
    }
}

#[test]
fn alpha_is_coverage_and_scales_the_light() {
    let mut gpu = Gpu::new().expect("a GPU");
    let ink = Ink::compile().expect("the ink program");
    let mut s = surface(&mut gpu, ColorType::RGBAF16, Some(Profile { primaries: Primaries::Srgb, transfer: Transfer::Linear }.space()));
    paint_small(&ink, &mut s, Color4f::new(0.5, 0.5, 0.5, 0.25));
    let [r, g, b, a] = read(&mut s);
    let light = srgb_to_linear(0.5) as f32;
    assert!((a - 0.25).abs() < 1e-3, "alpha {a}");
    assert!([r, g, b].iter().all(|c| (c - 0.25 * light).abs() < 1e-4), "premultiplied light {r} {g} {b} against {}", 0.25 * light);
}

#[test]
fn a_picture_that_is_not_tagged_takes_the_colour_as_the_paint_always_did() {
    let mut gpu = Gpu::new().expect("a GPU");
    let ink = Ink::compile().expect("the ink program");
    let mut s = surface(&mut gpu, ColorType::RGBAF16, None);
    let mut paint = Paint::default();
    ink.apply(&mut paint, s.canvas(), Color4f::new(0.25, 0.5, 0.75, 1.0)).expect("an ink paint");
    assert!(paint.shader().is_none(), "no shader is made for a picture that nothing converts");
    assert_eq!(paint.color4f(), Color4f::new(0.25, 0.5, 0.75, 1.0));
}
