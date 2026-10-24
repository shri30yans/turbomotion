//! Running a shader over a layer: the one way every effect turns a layer into a new layer.

use anyhow::{anyhow, Result};
use skia_safe::{Canvas, Paint, Rect, Shader, TileMode};

use crate::studio::{
    layer::{linear, Layer},
    shader::{Program, Uniforms},
    stage::Stage,
};

/// What an effect may use while it runs: the shared services, the canvas that provides the GPU context, and the time.
pub struct Ctx<'a> {
    pub st: &'a Stage,
    pub like: &'a Canvas,
    pub t: f64,
}

/// `layer` as a bilinear shader that is transparent beyond its edges.
pub fn source(layer: &Layer) -> Result<Shader> {
    layer.image().to_shader((TileMode::Decal, TileMode::Decal), linear(), None).ok_or_else(|| anyhow!("could not sample a layer"))
}

/// Run `program` over `layer`: the layer is the first child shader, `more` follow, and the result has the layer's size.
pub fn run(ctx: &Ctx, layer: &Layer, program: &Program, uniforms: Uniforms, more: &[Shader]) -> Result<Layer> {
    let mut children = vec![source(layer)?];
    children.extend_from_slice(more);
    let shader = program.instance(uniforms, &children)?;
    let mut paint = Paint::default();
    paint.set_shader(shader);
    Layer::render(ctx.like, layer.width(), layer.height(), |c| {
        c.draw_rect(Rect::from_wh(layer.width() as f32, layer.height() as f32), &paint);
    })
}
