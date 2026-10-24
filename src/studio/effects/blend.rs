//! How an element's layer combines with what is already on the scene.

use serde::Deserialize;
use skia_safe::BlendMode;

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Blend {
    Normal,
    Add,
    Screen,
    Multiply,
    Overlay,
    SoftLight,
    HardLight,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl Blend {
    pub fn skia(self) -> BlendMode {
        match self {
            Self::Normal => BlendMode::SrcOver,
            Self::Add => BlendMode::Plus,
            Self::Screen => BlendMode::Screen,
            Self::Multiply => BlendMode::Multiply,
            Self::Overlay => BlendMode::Overlay,
            Self::SoftLight => BlendMode::SoftLight,
            Self::HardLight => BlendMode::HardLight,
            Self::Darken => BlendMode::Darken,
            Self::Lighten => BlendMode::Lighten,
            Self::ColorDodge => BlendMode::ColorDodge,
            Self::ColorBurn => BlendMode::ColorBurn,
            Self::Difference => BlendMode::Difference,
            Self::Exclusion => BlendMode::Exclusion,
            Self::Hue => BlendMode::Hue,
            Self::Saturation => BlendMode::Saturation,
            Self::Color => BlendMode::Color,
            Self::Luminosity => BlendMode::Luminosity,
        }
    }
}
