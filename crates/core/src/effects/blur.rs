//! Blurs and glow: Gaussian, directional (motion), radial (zoom) and a glow that adds a blurred copy back on top.
//! Each effect's parameters are declared in the registry (`mod.rs`); the structs only carry them.

use anyhow::Result;
use serde::Deserialize;
use skia_safe::{color_filters, Paint};

use super::{
    param::Param,
    pass::{run, Ctx},
};
use crate::{
    color::Rgb,
    layer::{solid_color, Layer},
    shader::Uniforms,
};

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Blur {
    pub radius: Param,
}

pub fn blur(ctx: &Ctx, e: &Blur, layer: &Layer) -> Result<Layer> {
    layer.blurred(ctx.like, e.radius.at(ctx.t))
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct DirectionalBlur {
    pub distance: Param,
    pub angle: Param,
}

/// Below this smear length the layer is left alone.
const MIN_SMEAR: f64 = 1.5;
/// Samples along a smear are at least this many and at most `MAX_TAPS`: more costs time, fewer shows steps.
const MIN_TAPS: f64 = 3.0;
const MAX_TAPS: f64 = 64.0;
/// One sample per this many pixels of smear.
const PIXELS_PER_TAP: f64 = 2.0;

/// Smear `layer` `distance` pixels along `angle` degrees.
pub fn smear(ctx: &Ctx, layer: &Layer, distance: f64, angle: f64) -> Result<Layer> {
    if distance.abs() < MIN_SMEAR {
        return Ok(Layer::from_image(layer.image().clone()));
    }
    let (sin, cos) = angle.to_radians().sin_cos();
    let taps = (distance.abs() / PIXELS_PER_TAP).round().clamp(MIN_TAPS, MAX_TAPS);
    run(ctx, layer, &ctx.shaders.effects.directional_blur, Uniforms::new().v2(cos * distance, sin * distance).f(taps), &[])
}

pub fn directional_blur(ctx: &Ctx, e: &DirectionalBlur, layer: &Layer) -> Result<Layer> {
    smear(ctx, layer, e.distance.at(ctx.t), e.angle.at(ctx.t))
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct RadialBlur {
    pub amount: Param,
    pub x: Param,
    pub y: Param,
}

/// Below this zoom the layer is left alone.
const MIN_ZOOM: f64 = 0.002;
/// Samples along a zoom smear.
const ZOOM_TAPS: f64 = 24.0;

pub fn radial_blur(ctx: &Ctx, e: &RadialBlur, layer: &Layer) -> Result<Layer> {
    let amount = e.amount.at(ctx.t);
    if amount.abs() < MIN_ZOOM {
        return Ok(Layer::from_image(layer.image().clone()));
    }
    let centre = (e.x.at(ctx.t) * f64::from(layer.width()), e.y.at(ctx.t) * f64::from(layer.height()));
    run(ctx, layer, &ctx.shaders.effects.radial_blur, Uniforms::new().v2(centre.0, centre.1).f(amount).f(ZOOM_TAPS), &[])
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Glow {
    pub radius: Param,
    pub strength: Param,
    pub threshold: Param,
    pub color: Option<Rgb>,
}

pub fn glow(ctx: &Ctx, e: &Glow, layer: &Layer) -> Result<Layer> {
    let (strength, threshold) = (e.strength.at(ctx.t), e.threshold.at(ctx.t));
    let source = if threshold > 0.0 {
        run(ctx, layer, &ctx.shaders.effects.bright, Uniforms::new().f(threshold), &[])?
    } else {
        Layer::from_image(layer.image().clone())
    };
    let halo = source.blurred(ctx.like, e.radius.at(ctx.t))?;
    let mut paint = Paint::default();
    paint.set_blender(ctx.shaders.union.blend(Uniforms::new())?);
    paint.set_color_filter(match e.color {
        Some(color) => solid_color(color, strength as f32),
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
