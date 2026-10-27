//! What an effect's parameters ask of the engine that it cannot give, found when the effect is built (with the largest
//! value each animated parameter ever takes) so that a film which cannot render never starts to.

use anyhow::{bail, Result};

use super::{quality::Quality, smear::Radial, Effect};
use crate::kernel::{Blur, Edge, Iris, Shape};

/// The Gaussian at `sigma` the exact blur would run, or nothing for the legacy blur, which has no limit.
fn gaussian(quality: Quality, sigma: f64) -> Result<()> {
    match quality {
        Quality::Legacy => Ok(()),
        Quality::Exact => Blur::gaussian([sigma; 2], Edge::Transparent).check(),
    }
}

impl Effect {
    /// Whether the effect can run as it is parameterised, or what it asks for that the kernel cannot do.
    pub(super) fn check(&self) -> Result<()> {
        match self {
            Effect::Blur(e) => match e.quality {
                Quality::Legacy => Ok(()),
                Quality::Exact => Blur::gaussian(e.dimensions.along(e.radius.peak()), e.edge).check(),
            },
            Effect::Glow(e) => gaussian(e.quality, e.radius.peak()),
            Effect::FilmHalation(e) => gaussian(e.quality, e.radius.peak()),
            Effect::Shadow(e) => gaussian(e.quality, e.blur.peak()),
            Effect::Backdrop(e) => gaussian(e.quality, e.blur.peak()),
            Effect::BoxBlur(e) => Blur { shape: Shape::Box { radius: e.dimensions.along(e.radius.peak()), iterations: e.iterations.peak().round() as u32 }, edge: e.edge }.check(),
            Effect::LensBlur(e) => {
                let widest = Iris { radius: e.radius.peak(), blades: e.blades.peak().round() as u32, ..e.iris(0.0) };
                Blur { shape: Shape::Disc(widest), edge: e.edge }.check()
            }
            Effect::RadialBlur(e) if e.quality == Quality::Legacy && e.mode == Radial::Spin => bail!("a spin blur needs `quality: exact`; the legacy radial blur only zooms"),
            _ => Ok(()),
        }
    }
}
