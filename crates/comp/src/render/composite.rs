//! Laying an effect's result over its input through a mask (see `fxlib::composite`): the one implementation every
//! effect, built-in or plugin, shares.

use anyhow::{Context, Result};
use skia_safe::{Paint, Rect};
use turbomotion_core::layer::{linear, Layer};

use super::{fx::Scope, post};
use crate::{
    eval::fx::{CompositeState, MaskState},
    fxlib::composite::{Channel, CompositePlan},
    schema::MatteMode,
};

/// `result` laid over `input`: through the mask, with the blend mode, at `mix`.
pub fn apply(sc: &Scope, input: &Layer, result: Layer, plan: &CompositePlan, state: &CompositeState) -> Result<Layer> {
    let masked = match coverage(sc, input, state)? {
        Some(cov) => post::multiply_by(sc.like, &result, &cov)?,
        None => result,
    };
    let mut paint = Paint::default();
    paint.set_blend_mode(plan.blend.skia()).set_alpha_f(state.mix);
    Layer::try_render(sc.like, input.width(), input.height(), |c| {
        c.draw_image(input.image(), (0, 0), None);
        c.draw_image(masked.image(), (0, 0), Some(&paint));
        Ok(())
    })
}

/// Where the result shows, as the alpha of a layer; none when it shows everywhere.
fn coverage(sc: &Scope, input: &Layer, state: &CompositeState) -> Result<Option<Layer>> {
    Ok(match &state.mask {
        MaskState::Everywhere => None,
        MaskState::Alpha => Some(input.clone()),
        MaskState::Luma { lo, hi, softness, invert } => Some(band(sc, &sc.ctx.programs.luma_band, input, &[("lo", *lo), ("hi", *hi), ("softness", *softness), ("invert", f32::from(u8::from(*invert)))])?),
        MaskState::Edges { strength, width } => Some(band(sc, &sc.ctx.programs.edge_band, input, &[("strength", *strength), ("width", *width)])?),
        MaskState::Layer { layer, channel, invert } => {
            let mode = match (channel, invert) {
                (Channel::Luma, false) => MatteMode::Luma,
                (Channel::Luma, true) => MatteMode::LumaInverted,
                (Channel::Alpha, false) => MatteMode::Alpha,
                (Channel::Alpha, true) => MatteMode::AlphaInverted,
            };
            Some(post::matte_coverage(sc.like, sc.frame, sc.picture(*layer)?, mode, 1.0)?)
        }
    })
}

/// Run a mask program over `input`.
fn band(sc: &Scope, program: &turbomotion_core::shader::Program, input: &Layer, values: &[(&str, f32)]) -> Result<Layer> {
    let (w, h) = (input.width(), input.height());
    let source = input.image().to_shader((skia_safe::TileMode::Clamp, skia_safe::TileMode::Clamp), linear(), None).context("the layer image has no shader form")?;
    let floats: Vec<[f32; 1]> = values.iter().map(|(_, v)| [*v]).collect();
    let named: Vec<(&str, &[f32])> = values.iter().zip(&floats).map(|((n, _), f)| (*n, f.as_slice())).collect();
    let shader = program.instance_with(&named, &[("source", source)])?;
    Layer::try_render(sc.like, w, h, |c| {
        let mut paint = Paint::default();
        paint.set_shader(shader);
        c.draw_rect(Rect::from_wh(w as f32, h as f32), &paint);
        Ok(())
    })
}
