//! The Gaussian blur and the glow that adds a blurred copy back on top.
//! Each effect's parameters are declared in the registry (`mod.rs`); the structs only carry them.

use anyhow::Result;
use serde::Deserialize;
use skia_safe::{color_filters, Paint};

use super::{
    param::Param,
    pass::{run, Ctx},
    quality::{soften, Quality},
};
use crate::{
    color::{Encoding, Rgb},
    kernel::{self, Axes, Edge},
    layer::{solid_color, Layer},
    shader::Uniforms,
};

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Blur {
    pub radius: Param,
    pub quality: Quality,
    pub dimensions: Axes,
    pub edge: Edge,
}

pub fn blur(ctx: &Ctx, e: &Blur, layer: &Layer) -> Result<Layer> {
    let sigma = e.dimensions.along(e.radius.at(ctx.t));
    match e.quality {
        Quality::Legacy => layer.blurred_by_axis(ctx.like, sigma, e.edge.tile()),
        Quality::Exact => kernel::Blur::gaussian(sigma, e.edge).apply(ctx.shaders, ctx.like, layer),
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Glow {
    pub radius: Param,
    pub strength: Param,
    pub threshold: Param,
    pub color: Option<Rgb>,
    pub quality: Quality,
}

pub fn glow(ctx: &Ctx, e: &Glow, layer: &Layer) -> Result<Layer> {
    let (strength, threshold) = (e.strength.at(ctx.t), e.threshold.at(ctx.t));
    let source = if threshold > 0.0 {
        // the threshold is a brightness as the eye reports it; what passes it is spread as light
        run(&ctx.computing_in(Encoding::Display), layer, &ctx.shaders.effects.bright, Uniforms::new().f(threshold), &[])?
    } else {
        Layer::from_image(layer.image().clone())
    };
    let halo = soften(ctx, &source, e.radius.at(ctx.t), e.quality)?;
    let mut paint = Paint::default();
    paint.set_blender(ctx.shaders.union.blend(Uniforms::new())?);
    paint.set_color_filter(match e.color {
        Some(color) => solid_color(ctx.like, color, strength as f32)?,
        None => {
            let k = strength as f32;
            #[rustfmt::skip]
            let scale = [k, 0.0, 0.0, 0.0, 0.0, 0.0, k, 0.0, 0.0, 0.0, 0.0, 0.0, k, 0.0, 0.0, 0.0, 0.0, 0.0, k, 0.0];
            color_filters::matrix_row_major(&scale, None)
        }
    });
    Layer::render(ctx.like, layer.width(), layer.height(), |c| {
        c.draw_image(layer.image(), (0, 0), None);
        c.draw_image(halo.image(), (0, 0), Some(&paint));
    })
}
