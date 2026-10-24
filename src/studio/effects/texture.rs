//! Texture effects: film grain, scanlines and a vignette.

use anyhow::Result;
use serde::Deserialize;

use super::{
    param::{value, Param},
    pass::{run, Ctx},
};
use crate::studio::{layer::Layer, shader::Uniforms};

/// Grain is redrawn this many times a second.
const GRAIN_RATE: f64 = 24.0;

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Grain {
    /// Strength of the noise (0.1 is visible, 0.3 is heavy).
    pub amount: Option<Param>,
    /// Pattern seed.
    pub seed: Option<u32>,
}

pub fn grain(ctx: &Ctx, e: &Grain, layer: &Layer) -> Result<Layer> {
    let seed = (ctx.t * GRAIN_RATE).floor() + f64::from(e.seed.unwrap_or(0)) * 101.0;
    run(ctx, layer, &ctx.st.shaders.effects.grain, Uniforms::new().f(value(&e.amount, ctx.t, 0.12)).f(seed), &[])
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Scanlines {
    /// Distance between lines in pixels.
    pub spacing: Option<Param>,
    /// How much the dark part of each line dims the picture (0..1).
    pub strength: Option<Param>,
    /// Fraction of each spacing that is dark (0..1).
    pub thickness: Option<Param>,
}

pub fn scanlines(ctx: &Ctx, e: &Scanlines, layer: &Layer) -> Result<Layer> {
    let u = Uniforms::new().f(value(&e.spacing, ctx.t, 4.0).max(2.0)).f(value(&e.strength, ctx.t, 0.35)).f(value(&e.thickness, ctx.t, 0.5).clamp(0.05, 0.85));
    run(ctx, layer, &ctx.st.shaders.effects.scanlines, u, &[])
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Vignette {
    /// 0 = none, 1 = black corners.
    pub strength: Option<Param>,
    /// How far from the centre the falloff starts, as a fraction of the half diagonal (0..1).
    pub softness: Option<Param>,
}

pub fn vignette(ctx: &Ctx, e: &Vignette, layer: &Layer) -> Result<Layer> {
    let u = Uniforms::new().v2(f64::from(layer.width()), f64::from(layer.height())).f(value(&e.strength, ctx.t, 0.5)).f(value(&e.softness, ctx.t, 0.45));
    run(ctx, layer, &ctx.st.shaders.effects.vignette, u, &[])
}
