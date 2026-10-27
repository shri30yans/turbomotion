//! Unsharp mask: the picture plus a fraction of what a Gaussian blur of it takes away.

use anyhow::Result;
use serde::Deserialize;

use super::{
    param::Param,
    pass::{run, source, Ctx},
};
use crate::{
    kernel::{self, Edge},
    layer::Layer,
    shader::Uniforms,
};

/// The levels of 255 a threshold is written in.
const LEVELS: f64 = 255.0;

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct UnsharpMask {
    pub amount: Param,
    pub radius: Param,
    pub threshold: Param,
}

pub fn unsharp_mask(ctx: &Ctx, e: &UnsharpMask, layer: &Layer) -> Result<Layer> {
    // the soft copy repeats the picture's edge so that the layer's border is not mistaken for a detail
    let soft = kernel::Blur::gaussian([e.radius.at(ctx.t); 2], Edge::Repeat).apply(ctx.shaders, ctx.like, layer)?;
    let uniforms = Uniforms::new().f(e.amount.at(ctx.t) / 100.0).f(e.threshold.at(ctx.t) / LEVELS);
    run(ctx, layer, &ctx.shaders.effects.unsharp, uniforms, &[source(ctx, &soft)?])
}
