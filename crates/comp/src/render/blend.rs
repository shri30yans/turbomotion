//! A blend mode as the renderer draws it: what the compositor and an effect's `composite.blend` both go through.

use anyhow::Result;
use skia_safe::{Canvas, Paint};
use turbomotion_core::{effects::Blend, layer::Layer};

use super::Ctx;
use crate::eval::Frame;

/// The picture to draw and the paint to draw it with, for `img` laid over what is below in `blend` (normal when none) at
/// `opacity`. A dancing dissolve re-rolls its pattern every frame: the frame number is the time in frames of the
/// composition being drawn.
pub fn prepare(ctx: &Ctx, like: &Canvas, frame: &Frame, blend: Option<Blend>, img: &Layer, opacity: f32) -> Result<(Layer, Paint)> {
    let mut paint = Paint::default();
    paint.set_alpha_f(opacity);
    let picture = match blend {
        Some(blend) => blend.stage(&ctx.shaders.blend, like, img, &mut paint, (frame.t * f64::from(ctx.plan.fps)).round() as u32)?,
        None => img.clone(),
    };
    Ok((picture, paint))
}
