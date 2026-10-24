//! Building blocks the transitions share: running a shader over the whole frame, warping a frame, and blending frames.

use anyhow::{anyhow, Result};
use skia_safe::{Canvas, Color4f, Image, Paint, Rect, Shader, TileMode};

use crate::{{model::{H, W}}};
use turbomotion_core::{ease::{smooth}, geom::{Mat3}, layer::{linear, Layer}};

pub fn frame() -> Rect {
    Rect::from_wh(W as f32, H as f32)
}

/// `image` as a bilinear shader that continues its edge pixels (`Clamp`) or turns transparent (`Decal`) beyond them.
pub fn sampled(image: &Image, edge: TileMode) -> Result<Shader> {
    image.to_shader((edge, edge), linear(), None).ok_or_else(|| anyhow!("could not sample a frame"))
}

/// `image` as a shader that reads exact texels.
pub fn texels(image: &Image) -> Result<Shader> {
    image.to_shader((TileMode::Clamp, TileMode::Clamp), skia_safe::SamplingOptions::default(), None).ok_or_else(|| anyhow!("could not sample a frame"))
}

/// Run `shader` over a new frame-sized image.
pub fn run(like: &Canvas, shader: Shader) -> Result<Image> {
    run_sized(like, W, H, shader)
}

/// Run `shader` over a new `w` x `h` image.
pub fn run_sized(like: &Canvas, w: i32, h: i32, shader: Shader) -> Result<Image> {
    let mut paint = Paint::default();
    paint.set_shader(shader);
    Ok(Layer::render(like, w, h, |c| {
        c.draw_rect(Rect::from_wh(w as f32, h as f32), &paint);
    })?
    .image()
    .clone())
}

/// `image` warped by `m` (source pixel indices to frame pixel indices, as OpenCV's `warpAffine`), continuing its edge
/// pixels outward like `BORDER_REPLICATE`.
pub fn warped(like: &Canvas, image: &Image, m: &Mat3) -> Result<Image> {
    let shader = image
        .to_shader((TileMode::Clamp, TileMode::Clamp), linear(), Some(&m.to_skia()))
        .ok_or_else(|| anyhow!("could not warp a frame"))?;
    run(like, shader)
}

/// `a * (1 - k) + b * k`.
pub fn crossfade(like: &Canvas, a: &Image, b: &Image, k: f64) -> Result<Image> {
    let mut paint = Paint::default();
    paint.set_alpha_f(k as f32);
    Ok(Layer::render(like, W, H, |c| {
        c.draw_image(a, (0, 0), None);
        c.draw_image(b, (0, 0), Some(&paint));
    })?
    .image()
    .clone())
}

/// A takes `smooth(2u)` of its light away over the first half, then B comes up from black over the second.
pub fn fade_through_black(like: &Canvas, a: &Image, b: &Image, u: f64) -> Result<Image> {
    let (frame_image, level) = if u < 0.5 { (a, 1.0 - smooth(u * 2.0)) } else { (b, smooth((u - 0.5) * 2.0)) };
    let mut paint = Paint::default();
    paint.set_alpha_f(level as f32);
    Ok(Layer::render(like, W, H, |c| {
        c.clear(skia_safe::Color::BLACK);
        c.draw_image(frame_image, (0, 0), Some(&paint));
    })?
    .image()
    .clone())
}

/// Brighten `image` by `amount` on every channel without exceeding white.
pub fn flash(like: &Canvas, image: &Image, amount: f64, add: &skia_safe::Blender) -> Result<Image> {
    let mut paint = Paint::new(Color4f::new(amount as f32, amount as f32, amount as f32, 1.0), None);
    paint.set_blender(add.clone());
    Ok(Layer::render(like, W, H, |c| {
        c.draw_image(image, (0, 0), None);
        c.draw_rect(frame(), &paint);
    })?
    .image()
    .clone())
}
