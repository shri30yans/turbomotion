//! The SkSL programs behind the transitions, compiled once with the other shaders.

use anyhow::Result;

use turbomotion_core::{shader::{Program}};

pub struct Programs {
    pub radial: Program,
    pub luma: Program,
    pub ripple: Program,
    pub whip: Program,
    pub zoom_blur: Program,
    pub leak: Program,
    pub slide: Program,
    pub glitch: Program,
    pub area: Program,
    pub pixel_up: Program,
}

impl Programs {
    pub fn compile() -> Result<Self> {
        Ok(Self {
            radial: Program::shader("radial transition", include_str!("../shaders/transitions/radial.sksl"))?,
            luma: Program::shader("luma transition", include_str!("../shaders/transitions/luma.sksl"))?,
            ripple: Program::shader("ripple transition", include_str!("../shaders/transitions/ripple.sksl"))?,
            whip: Program::shader("whip transition", include_str!("../shaders/transitions/whip.sksl"))?,
            zoom_blur: Program::shader("zoom blur", include_str!("../shaders/transitions/zoom_blur.sksl"))?,
            leak: Program::shader("leak transition", include_str!("../shaders/transitions/leak.sksl"))?,
            slide: Program::shader("slide transition", include_str!("../shaders/transitions/slide.sksl"))?,
            glitch: Program::shader("glitch transition", include_str!("../shaders/transitions/glitch.sksl"))?,
            area: Program::shader("area resize", include_str!("../shaders/transitions/area.sksl"))?,
            pixel_up: Program::shader("pixel enlargement", include_str!("../shaders/transitions/pixel_up.sksl"))?,
        })
    }
}
