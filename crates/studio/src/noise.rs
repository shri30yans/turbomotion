//! Seeded noise textures drawn from numpy's generator, so grain, worn ink and paper fibre match the reference.
//! Gaussian noise is stored as `0.5 + n * NOISE_SCALE` so it survives layers that clamp colours to 0..1.

use anyhow::{Context, Result};
use skia_safe::{AlphaType, Canvas, ColorType, CubicResampler, Data, Image, ImageInfo, Paint, Rect, SamplingOptions};

use super::{rng::{RandomState}};
use turbomotion_core::{layer::{Layer}};

/// Decode with `(value - 0.5) / NOISE_SCALE`.
pub const NOISE_SCALE: f32 = 0.1;

fn image(w: usize, h: usize, texel: impl Fn() -> f32) -> Result<Image> {
    let values: Vec<u8> = (0..w * h).flat_map(|_| [texel(), 0.0, 0.0, 1.0]).flat_map(|v| v.to_ne_bytes()).collect();
    let info = ImageInfo::new((w as i32, h as i32), ColorType::RGBAF32, AlphaType::Premul, None);
    skia_safe::images::raster_from_data(&info, Data::new_copy(&values), w * 16).context("noise image")
}

/// `rng.randn(h, w)` as an image (red channel, encoded).
pub fn gaussian(rng: &mut RandomState, w: usize, h: usize) -> Result<Image> {
    let rng = std::cell::RefCell::new(rng);
    image(w, h, || rng.borrow_mut().randn() as f32 * NOISE_SCALE + 0.5)
}

/// `rng.rand(h, w)` as an image (red channel, 0..1).
pub fn uniform(rng: &mut RandomState, w: usize, h: usize) -> Result<Image> {
    let rng = std::cell::RefCell::new(rng);
    image(w, h, || rng.borrow_mut().rand() as f32)
}

/// Stretch `small` to `w` x `h` with a cubic filter (`cv2.resize(..., INTER_CUBIC)`).
pub fn stretch(like: &Canvas, small: &Image, w: i32, h: i32) -> Result<Layer> {
    Layer::render(like, w, h, |c| {
        let mut paint = Paint::default();
        paint.set_anti_alias(false);
        c.draw_image_rect_with_sampling_options(small, None, Rect::from_wh(w as f32, h as f32), SamplingOptions::from(CubicResampler::catmull_rom()), &paint);
    })
}
