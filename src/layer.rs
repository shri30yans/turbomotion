//! Compositing helpers shared by the overlays.

use skia_safe::{canvas::SaveLayerRec, image_filters, Canvas, Paint, RRect, TileMode};

use crate::probe;

/// Draw `content` as one group at `opacity` (CSS `opacity`).
pub fn with_opacity(canvas: &Canvas, opacity: f32, content: impl FnOnce()) {
    canvas.save_layer_alpha_f(None, opacity);
    probe::fade(opacity, content);
    canvas.restore();
}

/// Draw `content` over a blurred copy of what is already on the canvas, clipped to `shape`
/// and faded as a whole (CSS `backdrop-filter: blur(sigma)` with `opacity`).
pub fn with_backdrop_blur(canvas: &Canvas, shape: RRect, sigma: f32, opacity: f32, content: impl FnOnce()) {
    let blur = image_filters::blur((sigma, sigma), TileMode::Clamp, None, None).expect("a finite blur radius");
    let mut alpha = Paint::default();
    alpha.set_alpha_f(opacity);
    canvas.save();
    canvas.clip_rrect(shape, None, true);
    canvas.save_layer(&SaveLayerRec::default().bounds(shape.bounds()).paint(&alpha).backdrop(&blur));
    probe::fade(opacity, content);
    canvas.restore();
    canvas.restore();
}

/// Draw `content` as one group, blurred by `sigma` and then faded by `opacity`
/// (an SVG group with `filter="blur"` and `opacity`).
pub fn with_blur(canvas: &Canvas, sigma: f32, opacity: f32, content: impl FnOnce()) {
    let blur = image_filters::blur((sigma, sigma), TileMode::Decal, None, None).expect("a finite blur radius");
    let mut paint = Paint::default();
    paint.set_alpha_f(opacity).set_image_filter(blur);
    canvas.save_layer(&SaveLayerRec::default().paint(&paint));
    probe::fade(opacity, content);
    canvas.restore();
}
