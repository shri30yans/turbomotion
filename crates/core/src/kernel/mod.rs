//! The one blur mechanism: kernels that spread light with the weights they say they have.
//!
//! A [`Blur`] is a shape (Gaussian, box or iris), the pixels beyond the picture's edge ([`Edge`]) and nothing else. Its
//! passes are separable where the shape is (one pass along x, one along y, each linear in the width of the kernel) and a
//! single pass over the iris's pixels where it is not. Every pass takes a weighted mean over the kernel's own weights in
//! 32-bit float, so the weights sum to one exactly (no light is gained or lost, whatever the sigma) and a wide kernel adds
//! no rounding of its own; the pictures between passes are half-float, as every picture is, and keep a smooth gradient
//! smooth. Glow, bloom, shadow, halation, the backdrop and the plugins' kernel passes all go through [`Blur::apply`]; none
//! has a blur of its own.
//!
//! The planner reads [`Blur::reach`], which comes from the same functions as the passes (`taps`), so a layer's buffer
//! grows by exactly what the kernel reads. Past a limit of the shader (`taps::MAX_TAPS` and friends) a blur is an error
//! that names the limit; it is never narrowed.

mod options;
mod programs;
mod run;
mod smear;
pub mod taps;

use anyhow::{ensure, Result};
use skia_safe::Canvas;

pub use programs::Programs;
pub use options::{Axes, Edge};
pub use smear::Smear;

use self::run::{run, Reading};
use crate::{layer::Layer, shader::Uniforms, shaders::Shaders};

/// Most times a box is repeated (a few passes already give a bell; the cost is linear in them).
pub const MAX_ITERATIONS: u32 = 16;

/// Most sides an iris has: past this a polygon is within 2% of a disc, so more would only cost shader time.
pub const MAX_BLADES: u32 = 16;

/// The opening of a lens: a disc, or a polygon with rounded corners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Iris {
    /// Pixels from the centre to the farthest part of the opening (a polygon's corners).
    pub radius: f64,
    /// Sides of the polygon; 0 is a disc.
    pub blades: u32,
    /// Degrees the polygon is turned.
    pub rotation: f64,
    /// 0 keeps the polygon's straight edges, 1 rounds it to a disc.
    pub roundness: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// A Gaussian of standard deviation `sigma` pixels along each axis; 0 leaves that axis alone.
    Gaussian { sigma: [f64; 2] },
    /// A box `2 radius + 1` pixels wide along each axis, repeated `iterations` times.
    Box { radius: [f64; 2], iterations: u32 },
    /// Every pixel in the iris, weighted by how much of the pixel it covers.
    Disc(Iris),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Blur {
    pub shape: Shape,
    pub edge: Edge,
}

/// One program run: a pass along an axis, or the iris over its square.
#[derive(Clone, Copy)]
enum Pass {
    Line { axis: [f64; 2], taps: u32, falloff: f64, last: f64 },
    Iris(Iris),
}

impl Blur {
    pub fn gaussian(sigma: [f64; 2], edge: Edge) -> Self {
        Self { shape: Shape::Gaussian { sigma }, edge }
    }

    /// Pixels the blur reads past a pixel along x and y: how far light spreads, and how much a layer must grow to hold it.
    pub fn reach(&self) -> [f64; 2] {
        match self.shape {
            Shape::Gaussian { sigma } => sigma.map(|s| f64::from(taps::gaussian_taps(s))),
            Shape::Box { radius, iterations } => radius.map(|r| f64::from(taps::box_taps(r).count * iterations)),
            Shape::Disc(iris) => [f64::from(taps::iris_taps(iris.radius)); 2],
        }
    }

    /// Whether the blur can be run, or what in it the passes cannot do.
    pub fn check(&self) -> Result<()> {
        match self.shape {
            Shape::Gaussian { sigma } => {
                ensure!(sigma.iter().all(|s| s.is_finite() && *s >= 0.0), "a Gaussian blur's sigma is a number of pixels, 0 or more, not {sigma:?}");
                let widest = sigma.map(taps::gaussian_taps).into_iter().max().unwrap_or(0);
                ensure!(widest <= taps::MAX_TAPS, "a Gaussian blur of sigma {} needs {widest} taps each side; the kernel pass reads at most {} (sigma {})", sigma[0].max(sigma[1]), taps::MAX_TAPS, f64::from(taps::MAX_TAPS) / taps::REACH);
            }
            Shape::Box { radius, iterations } => {
                ensure!(radius.iter().all(|r| r.is_finite() && *r >= 0.0), "a box blur's radius is a number of pixels, 0 or more, not {radius:?}");
                ensure!((1..=MAX_ITERATIONS).contains(&iterations), "a box blur repeats 1 to {MAX_ITERATIONS} times, not {iterations}");
                let widest = radius.map(|r| taps::box_taps(r).count).into_iter().max().unwrap_or(0);
                ensure!(widest <= taps::MAX_TAPS, "a box blur of radius {} needs {widest} taps each side; the kernel pass reads at most {}", radius[0].max(radius[1]), taps::MAX_TAPS);
            }
            Shape::Disc(iris) => {
                ensure!(iris.radius.is_finite() && iris.radius >= 0.0, "an iris radius is a number of pixels, 0 or more, not {}", iris.radius);
                ensure!(iris.blades == 0 || (3..=MAX_BLADES).contains(&iris.blades), "an iris has 0 blades (a disc) or 3 to {MAX_BLADES}, not {}", iris.blades);
                ensure!((0.0..=1.0).contains(&iris.roundness), "an iris roundness is 0 to 1, not {}", iris.roundness);
                ensure!(taps::iris_taps(iris.radius) <= taps::MAX_IRIS_TAPS, "an iris of radius {} needs {} taps each side; the lens pass reads at most {} (a radius of {})", iris.radius, taps::iris_taps(iris.radius), taps::MAX_IRIS_TAPS, f64::from(taps::MAX_IRIS_TAPS) - 0.5);
            }
        }
        Ok(())
    }

    fn passes(&self) -> Vec<Pass> {
        let line = |axis: [f64; 2], taps: u32, falloff: f64, last: f64| (taps > 0).then_some(Pass::Line { axis, taps, falloff, last });
        match self.shape {
            Shape::Gaussian { sigma: [x, y] } => [line([1.0, 0.0], taps::gaussian_taps(x), taps::falloff(x), 1.0), line([0.0, 1.0], taps::gaussian_taps(y), taps::falloff(y), 1.0)].into_iter().flatten().collect(),
            Shape::Box { radius: [x, y], iterations } => {
                let (bx, by) = (taps::box_taps(x), taps::box_taps(y));
                let once: Vec<Pass> = [line([1.0, 0.0], bx.count, 0.0, bx.last), line([0.0, 1.0], by.count, 0.0, by.last)].into_iter().flatten().collect();
                once.repeat(iterations as usize)
            }
            Shape::Disc(iris) => (taps::iris_taps(iris.radius) > 0).then_some(Pass::Iris(iris)).into_iter().collect(),
        }
    }

    /// `layer` blurred. A blur with nothing to spread returns the picture itself.
    pub fn apply(&self, shaders: &Shaders, like: &Canvas, layer: &Layer) -> Result<Layer> {
        self.check()?;
        let programs = &shaders.kernel;
        self.passes().iter().try_fold(layer.clone(), |picture, pass| match pass {
            Pass::Line { axis, taps, falloff, last } => run(like, &picture, &programs.separable, Uniforms::new().v2(axis[0], axis[1]).f(f64::from(*taps)).f(*falloff).f(*last), self.edge, Reading::Whole),
            Pass::Iris(iris) => {
                let uniforms = Uniforms::new().f(iris.radius).f(f64::from(iris.blades)).f(iris.rotation.to_radians()).f(iris.roundness).f(f64::from(taps::iris_taps(iris.radius)));
                run(like, &picture, &programs.iris, uniforms, self.edge, Reading::Whole)
            }
        })
    }
}

#[cfg(test)]
pub(crate) mod rig;
#[cfg(test)]
mod shape_tests;
#[cfg(test)]
mod tests;
