//! Strokes whose width changes along the outline, drawn as a filled outline instead of by Skia's stroker (which has one
//! width for the whole line).
//!
//! The width along a contour is a [`Profile`]: a function of the distance travelled. The outline is built from the
//! offset curves of the contour at half the profile's width either side ([`sample`] walks the contour into points whose
//! straight steps stay within a fiftieth of a pixel of both edges; [`pieces`] turns the points into polygons, with the
//! joins and caps of a stroke). A taper is one profile ([`taper`]); any other (a width profile written as `[(position,
//! width)]`, a width that goes negative and folds) is another implementation of the same trait, and the walk, the joins
//! and the caps do not change.
//!
//! A closed contour is walked from its first point to that point again, like an open one. It has no caps; the last
//! point is joined to the first with the stroke's join, so with no taper there is no seam, and with one the stroke
//! thins toward the start point from both sides.

#[cfg(test)]
pub(crate) mod coverage;
mod pieces;
mod sample;
pub mod taper;
mod vec2;

use anyhow::{bail, Result};
use skia_safe::{ContourMeasureIter, Path};

pub use taper::{Dim, Taper, Tip};

/// How wide the stroke is at each distance along a contour, measured across the stroke.
pub trait Profile {
    /// The width at distance `s` from the start of the contour. A negative width is taken as 0.
    fn width(&self, s: f64) -> f64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cap {
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Join {
    Miter,
    Round,
    Bevel,
}

impl Cap {
    pub fn parse(name: &str) -> Result<Cap> {
        match name {
            "butt" => Ok(Cap::Butt),
            "round" => Ok(Cap::Round),
            "square" => Ok(Cap::Square),
            other => bail!("a stroke cap is \"butt\", \"round\" or \"square\", not {other:?}"),
        }
    }
}

impl Join {
    pub fn parse(name: &str) -> Result<Join> {
        match name {
            "miter" => Ok(Join::Miter),
            "round" => Ok(Join::Round),
            "bevel" => Ok(Join::Bevel),
            other => bail!("a stroke join is \"miter\", \"round\" or \"bevel\", not {other:?}"),
        }
    }
}

/// How the ends and the corners of the stroke are finished.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub cap: Cap,
    pub join: Join,
}

/// The region a stroke covers, as a path to fill (nonzero rule): every contour of `path` stroked with the width
/// `profile_of(contour length)` gives along it.
pub fn outline<P: Profile>(path: &Path, style: Style, profile_of: impl Fn(f64) -> P) -> Path {
    let mut out = Path::new();
    for contour in ContourMeasureIter::new(path, false, None) {
        let length = f64::from(contour.length());
        if length.is_finite() && length > 0.0 {
            let profile = profile_of(length);
            pieces::stroke(&mut out, &sample::contour(&contour, &|s| profile.width(s)), contour.is_closed(), style);
        }
    }
    out
}

#[cfg(test)]
mod tests;
