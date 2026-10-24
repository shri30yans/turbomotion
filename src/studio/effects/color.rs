//! Colour effects: tone (brightness, contrast, saturation, hue, exposure, gamma), invert, tint and the scene grade.

use anyhow::Result;
use serde::Deserialize;

use super::{
    param::{value, Param},
    pass::{run, Ctx},
};
use crate::studio::{grade::uniforms as grade_uniforms, layer::Layer, model::{Grade, Rgb}, shader::Uniforms};

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
    run(ctx, layer, &ctx.st.shaders.effects.color, u, &[])
}

macro_rules! single_value_effects {
    ($( $name:ident / $func:ident: $doc:literal, default $default:expr, sets $field:ident[$idx:expr]; )*) => {$(
        #[derive(Debug, Deserialize, Clone)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            #[doc = $doc]
            pub amount: Option<Param>,
        }

        pub fn $func(ctx: &Ctx, e: &$name, layer: &Layer) -> Result<Layer> {
            let mut a = Adjust::default();
            a.$field[$idx] = value(&e.amount, ctx.t, $default);
            adjust(ctx, layer, a)
        }
    )*};
}

single_value_effects! {
    Brightness / brightness: "Added to every channel (0..1 scale): 0.2 is a clear lift, negative darkens.", default 0.0, sets tone[0];
    Contrast / contrast: "1 leaves the layer alone, 0 is flat grey, 2 is punchy.", default 1.0, sets tone[1];
    Saturation / saturation: "1 leaves the layer alone, 0 is black and white, 2 is vivid.", default 1.0, sets tone[2];
    Exposure / exposure: "Photographic stops: +1 doubles the light, -1 halves it.", default 0.0, sets curve[0];
    Gamma / gamma: "Mid-tone curve: above 1 lightens the mid-tones, below 1 darkens them.", default 1.0, sets curve[1];
    Invert / invert: "How far to the negative: 1 fully inverts, 0.5 is flat grey.", default 1.0, sets curve[2];
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Hue {
    /// Degrees around the colour wheel.
    pub degrees: Option<Param>,
}

pub fn hue(ctx: &Ctx, e: &Hue, layer: &Layer) -> Result<Layer> {
    let mut a = Adjust::default();
    a.tone[3] = value(&e.degrees, ctx.t, 0.0).to_radians();
    adjust(ctx, layer, a)
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Tint {
    pub color: Rgb,
    /// 0 keeps the layer's colours, 1 paints it all in `color` (the alpha shape stays).
    pub amount: Option<Param>,
}

pub fn tint(ctx: &Ctx, e: &Tint, layer: &Layer) -> Result<Layer> {
    let mut a = Adjust::default();
    a.curve[3] = value(&e.amount, ctx.t, 1.0);
    a.tint = e.color;
    adjust(ctx, layer, a)
}

/// The scene colour grade (`lift`, `gamma`, `gain`, `sat`, `tint`, `tint_k`, `contrast`) on one layer.
pub fn grade(ctx: &Ctx, e: &Grade, layer: &Layer) -> Result<Layer> {
    run(ctx, layer, &ctx.st.shaders.effects.grade, grade_uniforms(e), &[])
}
