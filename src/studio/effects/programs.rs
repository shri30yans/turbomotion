//! The SkSL programs behind the effects, compiled once with the other shaders.

use anyhow::Result;

use crate::studio::{grade, shader::Program};

pub struct Programs {
    pub directional_blur: Program,
    pub radial_blur: Program,
    pub bright: Program,
    pub color: Program,
    pub grade: Program,
    pub distort: Program,
    pub fringe: Program,
    pub pixelate: Program,
    pub grain: Program,
    pub scanlines: Program,
    pub vignette: Program,
}

impl Programs {
    pub fn compile() -> Result<Self> {
        Ok(Self {
            directional_blur: Program::shader("directional blur", include_str!("../shaders/effects/directional_blur.sksl"))?,
            radial_blur: Program::shader("radial blur", include_str!("../shaders/effects/radial_blur.sksl"))?,
            bright: Program::shader("glow threshold", include_str!("../shaders/effects/bright.sksl"))?,
            color: Program::shader("colour adjust", include_str!("../shaders/effects/color.sksl"))?,
            grade: Program::shader("layer grade", &[grade::SKSL, include_str!("../shaders/effects/grade.sksl")].concat())?,
            distort: Program::shader("distort", include_str!("../shaders/effects/distort.sksl"))?,
            fringe: Program::shader("colour fringe", include_str!("../shaders/effects/fringe.sksl"))?,
            pixelate: Program::shader("pixelate", include_str!("../shaders/effects/pixelate.sksl"))?,
            grain: Program::shader("grain", include_str!("../shaders/effects/grain.sksl"))?,
            scanlines: Program::shader("scanlines", include_str!("../shaders/effects/scanlines.sksl"))?,
            vignette: Program::shader("vignette", include_str!("../shaders/effects/vignette.sksl"))?,
        })
    }
}
