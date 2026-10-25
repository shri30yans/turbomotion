//! SkSL programs the composition renderer owns (the platform's live in `turbomotion-core`).

use anyhow::Result;
use turbomotion_core::shader::Program;

pub struct Programs {
    pub parallax: Program,
    pub lighting: Program,
    /// The coverage of a composite mask: a brightness band of the effect's input, and its outlines.
    pub luma_band: Program,
    pub edge_band: Program,
    /// Soft particle sprite.
    pub sprite: skia_safe::Image,
}

impl Programs {
    pub fn compile() -> Result<Programs> {
        Ok(Programs {
            parallax: Program::shader("parallax", include_str!("../shaders/parallax.sksl"))?,
            lighting: Program::shader("lighting", include_str!("../shaders/lighting.sksl"))?,
            luma_band: Program::shader("luma_band", include_str!("../shaders/luma_band.sksl"))?,
            edge_band: Program::shader("edge_band", include_str!("../shaders/edge_band.sksl"))?,
            sprite: super::particles::soft_sprite()?,
        })
    }
}
