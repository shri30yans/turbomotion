//! The SkSL programs the platform itself needs: the effects library and the two compositing blenders every renderer
//! uses for light (`add_clip`) and coverage (`union`). Each renderer compiles its own programs beside these.

use anyhow::Result;

use super::{effects, shader::Program};

pub struct Shaders {
    pub effects: effects::Programs,
    /// Additive light that clips at 1.0 (half-float layers do not clamp by themselves).
    pub add_clip: Program,
    /// Coverage union: `a + b - a*b` on alpha.
    pub union: Program,
}

impl Shaders {
    pub fn compile() -> Result<Self> {
        Ok(Self {
            effects: effects::Programs::compile()?,
            add_clip: Program::blender("add_clip", include_str!("shaders/add_clip.sksl"))?,
            union: Program::blender("union", include_str!("shaders/union.sksl"))?,
        })
    }
}
