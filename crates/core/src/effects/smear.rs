//! Directional (motion) and radial (zoom or spin) blur. `legacy` is the original sampled shader; `exact` is the
//! kernel's line integral ([`crate::kernel::Smear`]), which also turns.

use anyhow::{bail, Result};
use serde::Deserialize;

use super::{
    param::Param,
    pass::{run, Ctx},
    quality::Quality,
};
use crate::{color::Encoding, kernel::Smear, layer::Layer, shader::Uniforms};

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct DirectionalBlur {
    pub distance: Param,
    pub angle: Param,
    pub quality: Quality,
}

/// Below this smear length the layer is left alone.
pub(super) const MIN_SMEAR: f64 = 1.5;
/// Samples along a smear are at least this many and at most `MAX_TAPS`: more costs time, fewer shows steps.
const MIN_TAPS: f64 = 3.0;
const MAX_TAPS: f64 = 64.0;
/// One sample per this many pixels of smear.
const PIXELS_PER_TAP: f64 = 2.0;

/// Smear `layer` `distance` pixels along `angle` degrees (the legacy blur; post-processing uses it as it is).
pub fn smear(ctx: &Ctx, layer: &Layer, distance: f64, angle: f64) -> Result<Layer> {
    let ctx = &ctx.computing_in(Encoding::Linear);
    if distance.abs() < MIN_SMEAR {
        return Ok(Layer::from_image(layer.image().clone()));
    }
    let (sin, cos) = angle.to_radians().sin_cos();
    let taps = (distance.abs() / PIXELS_PER_TAP).round().clamp(MIN_TAPS, MAX_TAPS);
    run(ctx, layer, &ctx.shaders.effects.directional_blur, Uniforms::new().v2(cos * distance, sin * distance).f(taps), &[])
}

pub fn directional_blur(ctx: &Ctx, e: &DirectionalBlur, layer: &Layer) -> Result<Layer> {
    let (distance, angle) = (e.distance.at(ctx.t), e.angle.at(ctx.t));
    match e.quality {
        Quality::Legacy => smear(ctx, layer, distance, angle),
        Quality::Exact => exact(ctx, layer, Smear::line(distance, angle)),
    }
}

/// What the picture `like`'s canvas draws into: a smear that does not move anything returns the picture itself.
fn exact(ctx: &Ctx, layer: &Layer, smear: Smear) -> Result<Layer> {
    if smear.is_still() {
        return Ok(Layer::from_image(layer.image().clone()));
    }
    smear.apply(ctx.shaders, ctx.like, layer)
}

/// What a radial blur does to a point.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Radial {
    #[default]
    Zoom,
    Spin,
}

impl Radial {
    pub const NAMES: &'static [&'static str] = &["zoom", "spin"];
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct RadialBlur {
    pub amount: Param,
    pub x: Param,
    pub y: Param,
    pub mode: Radial,
    pub quality: Quality,
}

/// Below this zoom the layer is left alone.
const MIN_ZOOM: f64 = 0.002;
/// Samples along a zoom smear.
const ZOOM_TAPS: f64 = 24.0;

pub fn radial_blur(ctx: &Ctx, e: &RadialBlur, layer: &Layer) -> Result<Layer> {
    let amount = e.amount.at(ctx.t);
    let centre = ctx.space_of(layer).at(e.x.at(ctx.t), e.y.at(ctx.t));
    match (e.quality, e.mode) {
        (Quality::Legacy, Radial::Spin) => bail!("a spin blur needs `quality: exact`; the legacy radial blur only zooms"),
        (Quality::Legacy, Radial::Zoom) => {
            if amount.abs() < MIN_ZOOM {
                return Ok(Layer::from_image(layer.image().clone()));
            }
            run(ctx, layer, &ctx.shaders.effects.radial_blur, Uniforms::new().v2(centre.0, centre.1).f(amount).f(ZOOM_TAPS), &[])
        }
        (Quality::Exact, mode) => {
            // the kernel works on the picture's own pixels; the layer may sit at an offset in the frame
            let origin = ctx.space_of(layer).origin;
            let centre = [centre.0 - origin[0], centre.1 - origin[1]];
            exact(ctx, layer, if mode == Radial::Spin { Smear::spin(centre, amount) } else { Smear::zoom(centre, amount) })
        }
    }
}
