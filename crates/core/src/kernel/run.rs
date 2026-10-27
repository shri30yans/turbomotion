//! Running one kernel program over a picture: the picture is the program's `src`, read at whole pixels (or bilinearly for
//! a path), the sum is in the working light, the result is a new half-float picture of the same size.

use anyhow::{anyhow, Result};
use skia_safe::{Canvas, FilterMode, MipmapMode, Paint, Rect, SamplingOptions};

use super::Edge;
use crate::{
    color::Encoding,
    layer::Layer,
    shader::{Program, Uniforms},
};

/// How the program reads `src`: at the pixel's own value, or interpolated between pixels.
#[derive(Clone, Copy)]
pub(super) enum Reading {
    Whole,
    Smooth,
}

pub(super) fn run(like: &Canvas, layer: &Layer, program: &Program, uniforms: Uniforms, edge: Edge, reading: Reading) -> Result<Layer> {
    let filter = match reading {
        Reading::Whole => FilterMode::Nearest,
        Reading::Smooth => FilterMode::Linear,
    };
    let src = layer.image().to_shader((edge.tile(), edge.tile()), SamplingOptions::new(filter, MipmapMode::None), None).ok_or_else(|| anyhow!("could not sample a picture"))?;
    // a kernel spreads light: in a linear-light film the picture is read as it is, in a display one nothing converts
    let mut paint = Paint::default();
    paint.set_shader(Encoding::Linear.shader(like, program.instance(uniforms, &[src])?));
    Layer::render(like, layer.width(), layer.height(), |c| {
        c.draw_rect(Rect::from_wh(layer.width() as f32, layer.height() as f32), &paint);
    })
}
