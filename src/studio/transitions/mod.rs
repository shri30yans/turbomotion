//! Scene-to-scene transitions: each blends the outgoing frame `a` into the incoming frame `b` at progress `u`.
//!
//! Frames are opaque `W` x `H` images. Every transition is a few full-frame passes (a shader, or a warp followed by
//! a blend) so a frame is composed on the GPU; the ports follow the reference `fx.tr_*` functions one for one.

mod compose;
mod cube;
mod glitch;
mod lens;
mod pixel;
mod programs;
mod wipe;

pub use programs::Programs;

use anyhow::{bail, Result};
use skia_safe::{Canvas, Image};

use super::{ease::smooth, stage::Stage};

/// Which way a directional transition travels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Heading {
    Forward,
    Backward,
}

impl Heading {
    fn sign(self) -> f64 {
        match self {
            Self::Forward => 1.0,
            Self::Backward => -1.0,
        }
    }
}

/// A transition by its plan name.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Transition {
    Cut,
    Dissolve,
    Zoom,
    Whip(Heading),
    Glitch,
    Radial,
    Luma,
    Ripple,
    Spin,
    Cube(Heading),
    Leak,
    Slide(Heading),
    Pixel,
    Black,
}

impl Transition {
    pub fn parse(name: &str) -> Result<Self> {
        use Heading::{Backward, Forward};
        Ok(match name {
            "cut" => Self::Cut,
            "dissolve" => Self::Dissolve,
            "zoom" => Self::Zoom,
            "whip" => Self::Whip(Forward),
            "whip_l" => Self::Whip(Backward),
            "glitch" => Self::Glitch,
            "radial" => Self::Radial,
            "luma" => Self::Luma,
            "ripple" => Self::Ripple,
            "spin" => Self::Spin,
            "cube" => Self::Cube(Forward),
            "cube_l" => Self::Cube(Backward),
            "leak" => Self::Leak,
            "slide" => Self::Slide(Forward),
            "slide_l" => Self::Slide(Backward),
            "pixel" => Self::Pixel,
            "black" => Self::Black,
            other => bail!("unknown transition {other:?}"),
        })
    }
}

/// The frame between `a` and `b` at progress `u` of the transition called `name`.
pub fn blend(st: &Stage, like: &Canvas, name: &str, a: &Image, b: &Image, u: f64) -> Result<Image> {
    match Transition::parse(name)? {
        Transition::Cut => Ok(if u >= 0.5 { b.clone() } else { a.clone() }),
        Transition::Dissolve => compose::crossfade(like, a, b, smooth(u)),
        Transition::Black => compose::fade_through_black(like, a, b, u),
        Transition::Zoom => lens::zoom(st, like, a, b, u),
        Transition::Spin => lens::spin(st, like, a, b, u),
        Transition::Whip(heading) => wipe::whip(st, like, a, b, u, heading),
        Transition::Slide(heading) => wipe::slide(st, like, a, b, u, heading),
        Transition::Radial => wipe::radial(st, like, a, b, u),
        Transition::Luma => wipe::luma(st, like, a, b, u),
        Transition::Ripple => wipe::ripple(st, like, a, b, u),
        Transition::Leak => wipe::leak(st, like, a, b, u),
        Transition::Cube(heading) => cube::cube(st, like, a, b, u, heading),
        Transition::Glitch => glitch::glitch(st, like, a, b, u),
        Transition::Pixel => pixel::pixelate(st, like, a, b, u),
    }
}
