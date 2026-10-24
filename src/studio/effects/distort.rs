//! Distortions: wave, twirl, bulge, pixelate and colour fringing.

use std::f64::consts::TAU;

use anyhow::Result;
use serde::Deserialize;

use super::{
    param::{value, Param},
    pass::{run, Ctx},
};
use crate::studio::{layer::Layer, shader::Uniforms};

const WAVE: f64 = 0.0;
const TWIRL: f64 = 1.0;
const BULGE: f64 = 2.0;

fn centre(ctx: &Ctx, layer: &Layer, x: &Option<Param>, y: &Option<Param>) -> (f64, f64) {
    (value(x, ctx.t, 0.5) * f64::from(layer.width()), value(y, ctx.t, 0.5) * f64::from(layer.height()))
}

fn distort(ctx: &Ctx, layer: &Layer, mode: f64, shape: [f64; 4], at: (f64, f64)) -> Result<Layer> {
    run(ctx, layer, &ctx.st.shaders.effects.distort, Uniforms::new().f(mode).v4(shape[0], shape[1], shape[2], shape[3]).v2(at.0, at.1), &[])
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Wave {
    /// How far pixels move, in pixels.
    pub amplitude: Option<Param>,
    /// Distance between crests in pixels.
    pub wavelength: Option<Param>,
    /// Crests travelling past per second (negative reverses).
    pub speed: Option<Param>,
    /// Direction the wave travels in degrees (0 = left to right); pixels move across it.
    pub angle: Option<Param>,
}

pub fn wave(ctx: &Ctx, e: &Wave, layer: &Layer) -> Result<Layer> {
    let shape = [value(&e.amplitude, ctx.t, 12.0), value(&e.wavelength, ctx.t, 160.0).max(1.0), TAU * value(&e.speed, ctx.t, 0.5) * ctx.t, value(&e.angle, ctx.t, 0.0).to_radians()];
    distort(ctx, layer, WAVE, shape, (0.0, 0.0))
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Twirl {
    /// Turn at the centre in degrees (it eases to nothing at the radius).
    pub angle: Option<Param>,
    /// Reach in pixels.
    pub radius: Option<Param>,
    /// Centre as fractions of the frame.
    pub x: Option<Param>,
    pub y: Option<Param>,
}

pub fn twirl(ctx: &Ctx, e: &Twirl, layer: &Layer) -> Result<Layer> {
    let shape = [value(&e.angle, ctx.t, 90.0).to_radians(), value(&e.radius, ctx.t, 300.0).max(1.0), 0.0, 0.0];
    distort(ctx, layer, TWIRL, shape, centre(ctx, layer, &e.x, &e.y))
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Bulge {
    /// 0.5 magnifies the centre, negative pinches it in.
    pub strength: Option<Param>,
    /// Reach in pixels.
    pub radius: Option<Param>,
    pub x: Option<Param>,
    pub y: Option<Param>,
}

pub fn bulge(ctx: &Ctx, e: &Bulge, layer: &Layer) -> Result<Layer> {
    let shape = [value(&e.strength, ctx.t, 0.5), value(&e.radius, ctx.t, 300.0).max(1.0), 0.0, 0.0];
    distort(ctx, layer, BULGE, shape, centre(ctx, layer, &e.x, &e.y))
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Pixelate {
    /// Block size in pixels.
    pub size: Option<Param>,
}

pub fn pixelate(ctx: &Ctx, e: &Pixelate, layer: &Layer) -> Result<Layer> {
    run(ctx, layer, &ctx.st.shaders.effects.pixelate, Uniforms::new().f(value(&e.size, ctx.t, 16.0).max(1.0)), &[])
}

fn fringe(ctx: &Ctx, layer: &Layer, shift: (f64, f64), radial: f64) -> Result<Layer> {
    let (w, h) = (f64::from(layer.width()), f64::from(layer.height()));
    run(ctx, layer, &ctx.st.shaders.effects.fringe, Uniforms::new().v2(w / 2.0, h / 2.0).v2(shift.0, shift.1).f(radial).f(w.hypot(h) / 2.0), &[])
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ChromaticAberration {
    /// Red/blue separation in pixels at the frame corners (zero at the centre).
    pub amount: Option<Param>,
}

pub fn chromatic_aberration(ctx: &Ctx, e: &ChromaticAberration, layer: &Layer) -> Result<Layer> {
    let amount = value(&e.amount, ctx.t, 6.0);
    if amount.abs() < 0.05 {
        return Ok(Layer::from_image(layer.image().clone()));
    }
    fringe(ctx, layer, (0.0, 0.0), amount)
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct RgbSplit {
    /// Red and blue part ways by this many pixels (green stays).
    pub distance: Option<Param>,
    /// Direction in degrees (0 = horizontal).
    pub angle: Option<Param>,
}

pub fn rgb_split(ctx: &Ctx, e: &RgbSplit, layer: &Layer) -> Result<Layer> {
    let distance = value(&e.distance, ctx.t, 8.0);
    let (sin, cos) = value(&e.angle, ctx.t, 0.0).to_radians().sin_cos();
    fringe(ctx, layer, (cos * distance, sin * distance), 0.0)
}
