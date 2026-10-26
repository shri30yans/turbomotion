//! The encoding an effect computes in.

use anyhow::{anyhow, Result};
use skia_safe::{Canvas, ColorFilter, ColorSpace, Shader};

/// How an effect expects the numbers it reads and writes to relate to light.
///
/// Most effects were written against what the author sees on screen: a threshold of 0.5 means mid-grey, contrast pivots
/// on 0.5, a hue rotation keeps the brightness the eye reports. They are `Display`. Effects that move or spread light
/// (blurs, glows, bloom, halation, resampling) are physically right on light itself and say `Linear`.
///
/// When the working profile is linear light an effect is run in its own encoding, converted on the way in and out by the
/// same Skia colour management that tags the surfaces; when it is not, nothing needs converting and every effect sees
/// the working numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Encoding {
    /// The numbers are display-encoded (the sRGB curve).
    #[default]
    Display,
    /// The numbers are proportional to light.
    Linear,
}

impl Encoding {
    pub const NAMES: [&'static str; 2] = ["display", "linear"];

    pub fn name(self) -> &'static str {
        Self::NAMES[usize::from(self == Self::Linear)]
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "display" => Some(Self::Display),
            "linear" => Some(Self::Linear),
            _ => None,
        }
    }

    /// The colour space an effect of this encoding computes in when the pictures of `like` are linear light: none when
    /// they already are in it (or are not tagged at all).
    fn computing_in(self, like: &Canvas) -> Option<ColorSpace> {
        let working = like.image_info().color_space()?;
        let display = working.with_srgb_gamma();
        (self == Self::Display && display != working).then_some(display)
    }

    /// `shader` evaluated in this encoding, drawn on `like`'s pictures: its child shaders (the pictures it samples)
    /// arrive in the encoding, and what it returns is taken to be in it. Colour uniforms declared `layout(color)`
    /// arrive in it as well.
    pub fn shader(self, like: &Canvas, shader: Shader) -> Shader {
        match self.computing_in(like) {
            Some(space) => shader.with_working_color_space(space),
            None => shader,
        }
    }

    /// `filter` evaluated in this encoding (see [`Encoding::shader`]).
    pub fn filter(self, like: &Canvas, filter: ColorFilter) -> Result<ColorFilter> {
        match self.computing_in(like) {
            Some(space) => filter.with_working_color_space(space).ok_or_else(|| anyhow!("Skia could not move a colour filter into the display encoding")),
            None => Ok(filter),
        }
    }
}
