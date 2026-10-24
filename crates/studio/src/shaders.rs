//! Every SkSL program the studio renderer uses, compiled once per renderer.

use anyhow::Result;

use super::transitions;
use turbomotion_core::{grade, shader::Program, shaders::Shaders as CoreShaders};

pub struct Shaders {
    pub radial: Program,
    pub sweep: Program,
    pub dust: Program,
    pub bright: Program,
    pub post: Program,
    pub glow_top: Program,
    pub extrude_face: Program,
    pub ring: Program,
    pub wear: Program,
    pub bokeh: Program,
    pub edge_tint: Program,
    pub paper: Program,
    pub globe: Program,
    pub plate: Program,
    pub transitions: transitions::Programs,
    /// The platform's programs: effects library and the `add_clip` / `union` blenders.
    pub core: CoreShaders,
}

impl Shaders {
    pub fn compile() -> Result<Self> {
        Ok(Self {
            radial: Program::shader("radial", include_str!("shaders/radial.sksl"))?,
            sweep: Program::shader("sweep", include_str!("shaders/sweep.sksl"))?,
            dust: Program::shader("dust", include_str!("shaders/dust.sksl"))?,
            bright: Program::shader("bright", &[grade::SKSL, include_str!("shaders/bright.sksl")].concat())?,
            post: Program::shader("post", &[grade::SKSL, include_str!("shaders/post.sksl")].concat())?,
            glow_top: Program::shader("glow_top", include_str!("shaders/glow_top.sksl"))?,
            extrude_face: Program::shader("extrude_face", include_str!("shaders/extrude_face.sksl"))?,
            ring: Program::shader("ring", include_str!("shaders/ring.sksl"))?,
            wear: Program::shader("wear", include_str!("shaders/wear.sksl"))?,
            bokeh: Program::shader("bokeh", include_str!("shaders/bokeh.sksl"))?,
            edge_tint: Program::shader("edge_tint", include_str!("shaders/edge_tint.sksl"))?,
            paper: Program::shader("paper", include_str!("shaders/paper.sksl"))?,
            globe: Program::shader("globe", include_str!("shaders/globe.sksl"))?,
            transitions: transitions::Programs::compile()?,
            core: CoreShaders::compile()?,
            plate: Program::shader("plate", &[grade::SKSL, include_str!("shaders/plate.sksl")].concat())?,
        })
    }
}
