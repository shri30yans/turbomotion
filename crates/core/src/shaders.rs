//! The SkSL programs the platform itself needs: the effects library, the layer blend modes and the two compositing
//! blenders every renderer uses for light (`add_clip`) and coverage (`union`). Each renderer compiles its own programs
//! beside these.

use anyhow::Result;

use super::{color::Ink, effects, kernel, shader::Program};

pub struct Shaders {
    pub effects: effects::Programs,
    /// The blur kernels every blur, glow, shadow and plugin kernel pass runs.
    pub kernel: kernel::Programs,
    /// What the layer blend modes that Skia lacks are drawn with.
    pub blend: effects::BlendPrograms,
    /// Additive light that clips at 1.0 (half-float layers do not clamp by themselves).
    pub add_clip: Program,
    /// Coverage union: `a + b - a*b` on alpha.
    pub union: Program,
    /// How a flat colour is painted without being held in 8 bits (see [`Ink`]).
    pub ink: Ink,
}

impl Shaders {
    pub fn compile() -> Result<Self> {
        Ok(Self {
            effects: effects::Programs::compile()?,
            kernel: kernel::Programs::compile()?,
            blend: effects::BlendPrograms::compile()?,
            add_clip: Program::blender("add_clip", include_str!("shaders/add_clip.sksl"))?,
            union: Program::blender("union", include_str!("shaders/union.sksl"))?,
            ink: Ink::compile()?,
        })
    }
}
