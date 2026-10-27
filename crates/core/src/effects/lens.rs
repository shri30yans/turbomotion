//! Blurs with a shape other than a Gaussian: the box, and the lens whose iris is a disc or a polygon.
//! They exist only as the kernel blur ([`crate::kernel`]), so there is no `quality` to choose.

use anyhow::Result;
use serde::Deserialize;

use super::{param::Param, pass::Ctx};
use crate::{
    kernel::{self, Axes, Edge, Iris, Shape},
    layer::Layer,
};

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct BoxBlur {
    pub radius: Param,
    pub iterations: Param,
    pub dimensions: Axes,
    pub edge: Edge,
}

/// Times a box is repeated, which is a whole number however the parameter was animated.
pub fn iterations(p: &Param, t: f64) -> u32 {
    p.at(t).round().max(0.0) as u32
}

pub fn box_blur(ctx: &Ctx, e: &BoxBlur, layer: &Layer) -> Result<Layer> {
    let radius = e.dimensions.along(e.radius.at(ctx.t));
    kernel::Blur { shape: Shape::Box { radius, iterations: iterations(&e.iterations, ctx.t) }, edge: e.edge }.apply(ctx.shaders, ctx.like, layer)
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct LensBlur {
    pub radius: Param,
    pub blades: Param,
    pub rotation: Param,
    pub roundness: Param,
    pub edge: Edge,
}

impl LensBlur {
    pub fn iris(&self, t: f64) -> Iris {
        Iris { radius: self.radius.at(t), blades: self.blades.at(t).round().max(0.0) as u32, rotation: self.rotation.at(t), roundness: self.roundness.at(t) }
    }
}

pub fn lens_blur(ctx: &Ctx, e: &LensBlur, layer: &Layer) -> Result<Layer> {
    kernel::Blur { shape: Shape::Disc(e.iris(ctx.t)), edge: e.edge }.apply(ctx.shaders, ctx.like, layer)
}
