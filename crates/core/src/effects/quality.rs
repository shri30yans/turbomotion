//! The choice every effect that spreads light shares: the blur it was first built on, or the exact one.
//!
//! `legacy` is Skia's blur, which counts its light wrongly above a sigma of about 4 (a small source gains or loses up to
//! a quarter of its light, by sigma and by the size of the picture) and is kept so that every film made before the exact blur renders as it always did. `exact` is the
//! kernel blur ([`crate::kernel`]): the sigma asked for, none of the light lost, a reach of 4 sigma. It is a choice the
//! author makes per effect, never one the engine makes for them.

use anyhow::Result;
use serde::Deserialize;

use super::{
    pass::Ctx,
    spec::{choice, ParamSpec},
};
use crate::{
    kernel::{self, Edge},
    layer::Layer,
};

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    #[default]
    Legacy,
    Exact,
}

impl Quality {
    pub const NAMES: &'static [&'static str] = &["legacy", "exact"];
}

/// The `quality` parameter, declared once for every effect that has it.
pub const QUALITY: ParamSpec = choice(
    "quality",
    "legacy",
    Quality::NAMES,
    "`legacy` is the blur films were first made with, kept so they render as they did (above a sigma of 4 a small source gains or loses up to a quarter of its light); `exact` is the kernel blur: the sigma asked for, no light lost, a reach of 4 sigma.",
);

/// `layer` blurred by `sigma` pixels in every direction, beyond which it is transparent: the one blur the glows, the
/// shadow and the halation are built on.
pub fn soften(ctx: &Ctx, layer: &Layer, sigma: f64, quality: Quality) -> Result<Layer> {
    match quality {
        Quality::Legacy => layer.blurred(ctx.like, sigma),
        Quality::Exact => kernel::Blur::gaussian([sigma; 2], Edge::Transparent).apply(ctx.shaders, ctx.like, layer),
    }
}

/// How far light spreads from a pixel blurred by `sigma` (the layer must grow by this to hold it).
pub fn spread(sigma: f64, quality: Quality) -> f64 {
    match quality {
        Quality::Legacy => super::bounds::GAUSSIAN_REACH * sigma,
        Quality::Exact => kernel::Blur::gaussian([sigma; 2], Edge::Transparent).reach()[0],
    }
}
