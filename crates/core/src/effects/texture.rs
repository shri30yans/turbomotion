//! Texture effects: film grain, scanlines and a vignette.

use anyhow::Result;
use serde::Deserialize;

use super::{
    param::Param,
    pass::{run, Ctx},
};
use crate::{layer::Layer, shader::Uniforms};

/// Grain is redrawn this many times a second.
const GRAIN_RATE: f64 = 24.0;
/// Each seed shifts the grain pattern this many redraws along, so two seeds never show the same frame at once.
const SEED_STRIDE: f64 = 101.0;

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Grain {
    pub amount: Param,
    pub seed: Param,
}

pub fn grain(ctx: &Ctx, e: &Grain, layer: &Layer) -> Result<Layer> {
    let pattern = (ctx.t * GRAIN_RATE).floor() + e.seed.at(ctx.t) * SEED_STRIDE;
    run(ctx, layer, &ctx.shaders.effects.grain, Uniforms::new().f(e.amount.at(ctx.t)).f(pattern), &[])
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Scanlines {
    pub spacing: Param,
    pub strength: Param,
    pub thickness: Param,
}

pub fn scanlines(ctx: &Ctx, e: &Scanlines, layer: &Layer) -> Result<Layer> {
    let u = Uniforms::new().f(e.spacing.at(ctx.t)).f(e.strength.at(ctx.t)).f(e.thickness.at(ctx.t));
    run(ctx, layer, &ctx.shaders.effects.scanlines, u, &[])
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Vignette {
    pub strength: Param,
    pub softness: Param,
}

pub fn vignette(ctx: &Ctx, e: &Vignette, layer: &Layer) -> Result<Layer> {
    let space = ctx.space_of(layer);
    let u = Uniforms::new().v4(space.content[0], space.content[1], space.width(), space.height()).f(e.strength.at(ctx.t)).f(e.softness.at(ctx.t));
    run(ctx, layer, &ctx.shaders.effects.vignette, u, &[])
}
