//! SkSL programs the composition renderer owns (the platform's live in `turbomotion-core`).

use anyhow::Result;
use turbomotion_core::shader::Program;

pub struct Programs {
    pub parallax: Program,
    /// Soft particle sprite.
    pub sprite: skia_safe::Image,
}

impl Programs {
    pub fn compile() -> Result<Programs> {
        Ok(Programs { parallax: Program::shader("parallax", include_str!("../shaders/parallax.sksl"))?, sprite: super::particles::soft_sprite()? })
    }
}
