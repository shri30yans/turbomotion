//! Colour effects: tone (brightness, contrast, saturation, hue, exposure, gamma), invert, tint and the scene grade.

use anyhow::Result;
use serde::Deserialize;

use super::{
    param::Param,
    pass::{run, Ctx},
};
use crate::{grade::uniforms as grade_uniforms, layer::Layer, color::Rgb, grade::Grade, shader::Uniforms};

/// Neutral values of the `color.sksl` fields: brightness, contrast, saturation, hue | exposure, gamma, invert, tint.
#[derive(Clone, Copy)]
struct Adjust {
    tone: [f64; 4],
    curve: [f64; 4],
    tint: Rgb,
}

impl Default for Adjust {
    fn default() -> Self {
        Self { tone: [0.0, 1.0, 1.0, 0.0], curve: [0.0, 1.0, 0.0, 0.0], tint: [0, 0, 0] }
    }
}

fn adjust(ctx: &Ctx, layer: &Layer, a: Adjust) -> Result<Layer> {
    let [r, g, b] = a.tint.map(|v| f64::from(v) / 255.0);
    let u = Uniforms::new().v4(a.tone[0], a.tone[1], a.tone[2], a.tone[3]).v4(a.curve[0], a.curve[1], a.curve[2], a.curve[3]).v3(r, g, b);
    run(ctx, layer, &ctx.shaders.effects.color, u, &[])
}

/// Effects with one `amount` that sets one field of the shader's neutral values.
macro_rules! single_value_effects {
    ($( $name:ident / $func:ident: sets $field:ident[$idx:expr]; )*) => {$(
        #[derive(Debug, Deserialize, Clone)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            pub amount: Param,
        }

        pub fn $func(ctx: &Ctx, e: &$name, layer: &Layer) -> Result<Layer> {
            let mut a = Adjust::default();
            a.$field[$idx] = e.amount.at(ctx.t);
            adjust(ctx, layer, a)
        }
    )*};
}

single_value_effects! {
    Brightness / brightness: sets tone[0];
    Contrast / contrast: sets tone[1];
    Saturation / saturation: sets tone[2];
    Exposure / exposure: sets curve[0];
    Gamma / gamma: sets curve[1];
    Invert / invert: sets curve[2];
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Hue {
    pub degrees: Param,
}

pub fn hue(ctx: &Ctx, e: &Hue, layer: &Layer) -> Result<Layer> {
    let mut a = Adjust::default();
    a.tone[3] = e.degrees.at(ctx.t).to_radians();
    adjust(ctx, layer, a)
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Tint {
    pub color: Rgb,
    pub amount: Param,
}

pub fn tint(ctx: &Ctx, e: &Tint, layer: &Layer) -> Result<Layer> {
    let mut a = Adjust::default();
    a.curve[3] = e.amount.at(ctx.t);
    a.tint = e.color;
    adjust(ctx, layer, a)
}

/// The scene colour grade (`lift`, `gamma`, `gain`, `sat`, `tint`, `tint_k`, `contrast`) on one layer.
pub fn grade(ctx: &Ctx, e: &Grade, layer: &Layer) -> Result<Layer> {
    run(ctx, layer, &ctx.shaders.effects.grade, grade_uniforms(e), &[])
}
