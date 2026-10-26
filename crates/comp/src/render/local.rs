//! Layer space (`effect_space: "layer"`): a layer is painted in its own pixels into a picture just big enough for it and
//! its effects (`eval::buffer`), its masks are drawn there, and only then is the picture carried to the frame through the
//! layer's matrix. These are the three functions that differ from the screen path; effects run through `fx` as usual,
//! given the picture's [`Space`](turbomotion_core::effects::Space).

use anyhow::{anyhow, Result};
use skia_safe::{CubicResampler, FilterMode, Matrix, MipmapMode, Paint, SamplingOptions};
use turbomotion_core::layer::Layer;

use super::{content::paint_content, post, Ctx};
use crate::{
    eval::{buffer::LocalBuffer, Frame, LayerState},
    plan::CompPlan,
};

/// The picture a layer-space layer draws into, which the evaluator sized.
fn buffer_of(st: &LayerState) -> Result<&LocalBuffer> {
    st.local.as_ref().ok_or_else(|| anyhow!("layer {:?} runs in layer space but the evaluator gave it no picture", st.label))
}

/// The layer's content in its own pixels, in a picture of the buffer's size whose top-left pixel is layer pixel `origin`.
pub fn paint(ctx: &Ctx, like: &skia_safe::Canvas, frame: &Frame, comp: &CompPlan, i: usize) -> Result<Layer> {
    let (st, lp) = (&frame.layers[i], &comp.layers[i]);
    let b = buffer_of(st)?;
    Layer::try_render(like, b.size[0], b.size[1], |c| {
        c.translate((-b.origin[0] as f32, -b.origin[1] as f32));
        paint_content(ctx, c, frame, comp, st, lp, false)
    })
}

/// The layer's masks as coverage in the buffer's picture.
pub fn mask_coverage(like: &skia_safe::Canvas, comp: &CompPlan, st: &LayerState) -> Result<Layer> {
    let b = buffer_of(st)?;
    post::mask_coverage_in(like, (b.size[0], b.size[1]), comp, st, |c| {
        c.translate((-b.origin[0] as f32, -b.origin[1] as f32));
    })
}

/// How the picture is resampled on its way to the frame: an interpolating cubic (Catmull-Rom, sharp, exact at whole
/// pixels) when it stays the same size or grows, and bilinear over mipmaps when it shrinks, so a reduced picture is
/// averaged instead of skipping pixels.
pub fn sampling(m: &Matrix) -> SamplingOptions {
    let scale = f64::from(m.scale_x()).hypot(f64::from(m.skew_y()));
    if scale < SHRINKS_BELOW {
        SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear)
    } else {
        SamplingOptions::from(CubicResampler::catmull_rom())
    }
}

/// A picture drawn at less than this scale counts as shrunk.
const SHRINKS_BELOW: f64 = 0.999;

/// Draw `img` (the layer's picture) onto `c` through the layer's matrix, with its top-left pixel at layer pixel `origin`,
/// using `paint` for blend mode and opacity (edges are always anti-aliased).
pub fn draw_placed(c: &skia_safe::Canvas, st: &LayerState, img: &Layer, paint: &Paint) -> Result<()> {
    let b = buffer_of(st)?;
    c.save();
    c.concat_44(&st.screen.to_m44());
    c.translate((b.origin[0] as f32, b.origin[1] as f32));
    let mut paint = paint.clone();
    paint.set_anti_alias(true);
    c.draw_image_with_sampling_options(img.image(), (0, 0), sampling(&c.local_to_device_as_3x3()), Some(&paint));
    c.restore();
    Ok(())
}

/// The layer's picture carried into a frame-sized picture through the layer's matrix.
pub fn place(like: &skia_safe::Canvas, frame: &Frame, st: &LayerState, img: &Layer) -> Result<Layer> {
    Layer::try_render(like, frame.width as i32, frame.height as i32, |c| draw_placed(c, st, img, &Paint::default()))
}
