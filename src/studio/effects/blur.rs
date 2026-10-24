//! Blurs and glow: Gaussian, directional (motion), radial (zoom) and a glow that adds a blurred copy back on top.

use anyhow::Result;
use serde::Deserialize;
use skia_safe::{color_filters, Paint};

use super::{
    param::{value, Param},
    pass::{run, Ctx},
};
use crate::studio::{
    layer::{solid_color, Layer},
    model::Rgb,
    shader::Uniforms,
};

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Blur {
    /// Gaussian sigma in pixels.
    pub radius: Option<Param>,
}

pub fn blur(ctx: &Ctx, e: &Blur, layer: &Layer) -> Result<Layer> {
    layer.blurred(ctx.like, value(&e.radius, ctx.t, 8.0))
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct DirectionalBlur {
    /// Length of the smear in pixels.
    pub distance: Option<Param>,
    /// Direction of the smear in degrees (0 = horizontal, 90 = vertical).
    pub angle: Option<Param>,
}

/// Below this smear length the layer is left alone.
const MIN_SMEAR: f64 = 1.5;
const MAX_TAPS: f64 = 64.0;

pub fn directional_blur(ctx: &Ctx, e: &DirectionalBlur, layer: &Layer) -> Result<Layer> {
    let distance = value(&e.distance, ctx.t, 40.0);
    if distance.abs() < MIN_SMEAR {
        return Ok(Layer::from_image(layer.image().clone()));
    }
    let (sin, cos) = value(&e.angle, ctx.t, 0.0).to_radians().sin_cos();
    let taps = (distance.abs() / 2.0).round().clamp(3.0, MAX_TAPS);
    run(ctx, layer, &ctx.st.shaders.effects.directional_blur, Uniforms::new().v2(cos * distance, sin * distance).f(taps), &[])
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct RadialBlur {
    /// Extra zoom at the end of the smear: 0.1 smears 10%.
    pub amount: Option<Param>,
    /// Centre as fractions of the frame.
    pub x: Option<Param>,
    pub y: Option<Param>,
}

pub fn radial_blur(ctx: &Ctx, e: &RadialBlur, layer: &Layer) -> Result<Layer> {
    let amount = value(&e.amount, ctx.t, 0.1);
    if amount.abs() < 0.002 {
        return Ok(Layer::from_image(layer.image().clone()));
    }
    let centre = (value(&e.x, ctx.t, 0.5) * f64::from(layer.width()), value(&e.y, ctx.t, 0.5) * f64::from(layer.height()));
    run(ctx, layer, &ctx.st.shaders.effects.radial_blur, Uniforms::new().v2(centre.0, centre.1).f(amount).f(24.0), &[])
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Glow {
    /// Blur sigma of the halo in pixels.
    pub radius: Option<Param>,
    /// How much of the halo is added back (1 = as bright as the source).
    pub strength: Option<Param>,
    /// Only parts brighter than this (0..1 luma) glow; 0 makes everything glow.
    pub threshold: Option<Param>,
    /// Colour of the halo; leave out to glow in the layer's own colours.
    pub color: Option<Rgb>,
}

pub fn glow(ctx: &Ctx, e: &Glow, layer: &Layer) -> Result<Layer> {
    let (strength, threshold) = (value(&e.strength, ctx.t, 1.0), value(&e.threshold, ctx.t, 0.0));
    let source = if threshold > 0.0 {
        run(ctx, layer, &ctx.st.shaders.effects.bright, Uniforms::new().f(threshold), &[])?
    } else {
        Layer::from_image(layer.image().clone())
    };
    let halo = source.blurred(ctx.like, value(&e.radius, ctx.t, 24.0))?;
    let mut paint = Paint::default();
    paint.set_blender(ctx.st.shaders.union.blend(Uniforms::new())?);
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

