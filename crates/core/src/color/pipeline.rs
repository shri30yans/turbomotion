//! Which profile a composition works in and which its frames leave in.

use skia_safe::ColorSpace;

use super::profile::Profile;
use crate::dither::Dither;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pipeline {
    /// What the pictures in the composition mean: blending, blurring and resampling happen on these numbers.
    pub working: Profile,
    /// What the finished frame means: a gamut and the sRGB curve, as the file is tagged.
    pub output: Profile,
    /// How the finished frame is rounded to 8 bits. It changes no picture and no conversion, so it is not colour
    /// management: [`Pipeline::is_managed`] does not see it.
    pub dither: Dither,
}

impl Default for Pipeline {
    /// Display-encoded sRGB in and out: nothing is converted and nothing is tagged.
    fn default() -> Self {
        Self { working: Profile::SRGB, output: Profile::SRGB, dither: Dither::None }
    }
}

impl Pipeline {
    /// Whether any picture is kept in or leaves in anything but plain sRGB. When it is not, no surface is tagged and
    /// Skia converts nothing.
    pub fn is_managed(&self) -> bool {
        (self.working, self.output) != (Profile::SRGB, Profile::SRGB)
    }

    /// The tag of the composition's pictures (none: untagged).
    pub fn working_space(&self) -> Option<ColorSpace> {
        self.is_managed().then(|| self.working.space())
    }

    /// The tag of the finished frame (none: untagged).
    pub fn output_space(&self) -> Option<ColorSpace> {
        self.is_managed().then(|| self.output.space())
    }

    /// Whether the working numbers are not the output's, so a picture must be flattened over its backdrop in the working
    /// profile before it is converted: converting a translucent picture and then flattening it is not the same colour.
    pub fn converts_at_output(&self) -> bool {
        self.working != self.output
    }
}
