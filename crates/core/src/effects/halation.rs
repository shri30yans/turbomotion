//! Film halation: light that passed the emulsion, bounced off the film base and exposed it again, which rings the
//! brightest parts with a wide red-orange halo. One thresholded copy of the picture is blurred with the engine's one
//! Gaussian ([`Layer::blurred`]) and added back tinted, so the effect owns no blur of its own.
//!
//! One caveat belongs to the engine's blur, not to this effect: Skia's GPU Gaussian is exact up to a sigma of 4 and
//! works at reduced resolution beyond it, where a source smaller than the reduced pixels is over- or under-counted
//! (measured: a one-pixel source returns 0.86x to 1.27x of its light depending on sigma; a 64 px square returns 0.99x to
//! 1.02x at sigma 12 and 0.95x to 0.96x at sigma 40). Halation inherits that, as `glow`, `bloom` and `shadow` do; making the blur exact would
//! change their pictures, so it is left alone and recorded in `parity/specs/film_halation.md`.
//!
//! The energy it adds is a number, not a look: where the blur is exact (sigma 4 and under), summed over the picture
//! the added luma is `strength` times the summed excess `max(0, luma - threshold * coverage)`, because the blur keeps
//! its sum and the tint is normalised to unit luma (`halation_tests` measures it).

use anyhow::Result;
use serde::Deserialize;

use super::{
    param::Param,
    pass::{run, source, Ctx},
};
use crate::{color::Rgb, layer::Layer, shader::Uniforms};

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct FilmHalation {
    pub threshold: Param,
    pub radius: Param,
    pub strength: Param,
    pub color: Rgb,
}

pub fn film_halation(ctx: &Ctx, e: &FilmHalation, layer: &Layer) -> Result<Layer> {
    let effects = &ctx.shaders.effects;
    let excess = run(ctx, layer, &effects.halation_source, Uniforms::new().f(e.threshold.at(ctx.t)), &[])?;
    let halo = excess.blurred(ctx.like, e.radius.at(ctx.t))?;
    run(ctx, layer, &effects.halation_add, Uniforms::new().rgb(e.color).f(e.strength.at(ctx.t)), &[source(ctx, &halo)?])
}
