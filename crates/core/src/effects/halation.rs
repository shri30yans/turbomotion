//! Film halation: light that passed the emulsion, bounced off the film base and exposed it again, which rings the
//! brightest parts with a wide red-orange halo. One thresholded copy of the picture is blurred with the engine's one
//! Gaussian ([`soften`]) and added back tinted, so the effect owns no blur of its own.
//!
//! `quality: legacy` keeps Skia's blur, which counts the light of a source smaller than its reduced pixels wrongly above a
//! sigma of 4 (the peak of a point source's halo is 2% low at sigma 16, 32 and 64). `quality: exact` is the kernel blur, whose
//! peak is the analytic Gaussian's (`halation_tests` asserts both numbers).
//!
//! The energy it adds is a number, not a look: summed over the picture the added luma is `strength` times the summed
//! excess `max(0, luma - threshold * coverage)`, because an exact blur keeps its sum and the tint is normalised to unit luma
//! (`halation_tests` measures it).

use anyhow::Result;
use serde::Deserialize;

use super::{
    param::Param,
    pass::{run, source, Ctx},
    quality::{soften, Quality},
};
use crate::{color::Rgb, layer::Layer, shader::Uniforms};

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct FilmHalation {
    pub threshold: Param,
    pub radius: Param,
    pub strength: Param,
    pub color: Rgb,
    pub quality: Quality,
}

pub fn film_halation(ctx: &Ctx, e: &FilmHalation, layer: &Layer) -> Result<Layer> {
    let effects = &ctx.shaders.effects;
    let excess = run(ctx, layer, &effects.halation_source, Uniforms::new().f(e.threshold.at(ctx.t)), &[])?;
    let halo = soften(ctx, &excess, e.radius.at(ctx.t), e.quality)?;
    run(ctx, layer, &effects.halation_add, Uniforms::new().rgb(e.color).f(e.strength.at(ctx.t)), &[source(ctx, &halo)?])
}
