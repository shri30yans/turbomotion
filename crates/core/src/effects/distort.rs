//! Distortions: wave, twirl, bulge, pixelate and colour fringing.

use std::f64::consts::TAU;

use anyhow::Result;
use serde::Deserialize;

use super::{
    param::Param,
    pass::{run, Ctx},
};
use crate::{layer::Layer, shader::Uniforms};

const WAVE: f64 = 0.0;
const TWIRL: f64 = 1.0;
const BULGE: f64 = 2.0;

/// The layer pixel at fractions `(x, y)` of the layer's content.
fn centre(ctx: &Ctx, layer: &Layer, x: &Param, y: &Param) -> (f64, f64) {
    ctx.space_of(layer).at(x.at(ctx.t), y.at(ctx.t))
}

fn distort(ctx: &Ctx, layer: &Layer, mode: f64, shape: [f64; 4], at: (f64, f64)) -> Result<Layer> {
    run(ctx, layer, &ctx.shaders.effects.distort, Uniforms::new().f(mode).v4(shape[0], shape[1], shape[2], shape[3]).v2(at.0, at.1), &[])
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Wave {
    pub amplitude: Param,
    pub wavelength: Param,
    pub speed: Param,
    pub angle: Param,
}

pub fn wave(ctx: &Ctx, e: &Wave, layer: &Layer) -> Result<Layer> {
    let shape = [e.amplitude.at(ctx.t), e.wavelength.at(ctx.t), TAU * e.speed.at(ctx.t) * ctx.t, e.angle.at(ctx.t).to_radians()];
    distort(ctx, layer, WAVE, shape, (0.0, 0.0))
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Twirl {
    pub angle: Param,
    pub radius: Param,
    pub x: Param,
    pub y: Param,
}

pub fn twirl(ctx: &Ctx, e: &Twirl, layer: &Layer) -> Result<Layer> {
    let shape = [e.angle.at(ctx.t).to_radians(), e.radius.at(ctx.t), 0.0, 0.0];
    distort(ctx, layer, TWIRL, shape, centre(ctx, layer, &e.x, &e.y))
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Bulge {
    pub strength: Param,
    pub radius: Param,
    pub x: Param,
    pub y: Param,
}

pub fn bulge(ctx: &Ctx, e: &Bulge, layer: &Layer) -> Result<Layer> {
    let shape = [e.strength.at(ctx.t), e.radius.at(ctx.t), 0.0, 0.0];
    distort(ctx, layer, BULGE, shape, centre(ctx, layer, &e.x, &e.y))
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Pixelate {
    pub size: Param,
}

pub fn pixelate(ctx: &Ctx, e: &Pixelate, layer: &Layer) -> Result<Layer> {
    run(ctx, layer, &ctx.shaders.effects.pixelate, Uniforms::new().f(e.size.at(ctx.t)), &[])
}

/// Below this separation the layer is left alone.
const MIN_FRINGE: f64 = 0.05;

fn fringe(ctx: &Ctx, layer: &Layer, shift: (f64, f64), radial: f64) -> Result<Layer> {
    let space = ctx.space_of(layer);
    let (w, h) = (space.width(), space.height());
    let (cx, cy) = space.centre();
    run(ctx, layer, &ctx.shaders.effects.fringe, Uniforms::new().v2(cx, cy).v2(shift.0, shift.1).f(radial).f(w.hypot(h) / 2.0), &[])
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ChromaticAberration {
    pub amount: Param,
}

pub fn chromatic_aberration(ctx: &Ctx, e: &ChromaticAberration, layer: &Layer) -> Result<Layer> {
    let amount = e.amount.at(ctx.t);
    if amount.abs() < MIN_FRINGE {
        return Ok(Layer::from_image(layer.image().clone()));
    }
    fringe(ctx, layer, (0.0, 0.0), amount)
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct RgbSplit {
    pub distance: Param,
    pub angle: Param,
}

pub fn rgb_split(ctx: &Ctx, e: &RgbSplit, layer: &Layer) -> Result<Layer> {
    let distance = e.distance.at(ctx.t);
    let (sin, cos) = e.angle.at(ctx.t).to_radians().sin_cos();
    fringe(ctx, layer, (cos * distance, sin * distance), 0.0)
}
