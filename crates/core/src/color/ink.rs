//! Flat colours: what a solid, a fill, a stroke, a glyph and a particle are painted with.
//!
//! A colour in a composition is an sRGB colour. Painted into a picture that is not in sRGB, Skia converts it, and Skia's
//! GPU backend then hands the converted colour to the fragment stage as an 8-bit attribute of the geometry (a paint
//! colour is "folded into the vertices" whenever it is constant). In display encoding that costs nothing: the colour was
//! 8 bits to begin with. In linear light it costs the darks: light between 0 and 1/255 has no representation, and
//! level 8 and level 16 both become the light of level 13. A uniform of a shader is a float on the GPU, so an [`Ink`]
//! paints a colour as a shader that returns its uniform, which Skia cannot fold into the geometry and converts exactly.
//!
//! A picture that is not tagged with a colour space is not converted, so there is nothing to protect there and the
//! colour is set on the paint as it always was.

use anyhow::Result;
use skia_safe::{Canvas, Color4f, Paint};

use crate::shader::Program;

/// The program that paints a flat colour, compiled once.
pub struct Ink {
    program: Program,
}

impl Ink {
    pub fn compile() -> Result<Self> {
        Ok(Self { program: Program::shader("ink", include_str!("../shaders/ink.sksl"))? })
    }

    /// Make `paint` paint `color` (an sRGB colour, not premultiplied) on `canvas`, replacing whatever colour or shader it
    /// painted with and keeping every other setting.
    pub fn apply(&self, paint: &mut Paint, canvas: &Canvas, color: Color4f) -> Result<()> {
        if canvas.image_info().color_space().is_none() {
            paint.set_shader(None).set_color4f(color, None);
            return Ok(());
        }
        let shader = self.program.instance_with(&[("color", &[color.r, color.g, color.b, color.a])], &[])?;
        paint.set_shader(shader).set_color4f(Color4f::new(1.0, 1.0, 1.0, 1.0), None);
        Ok(())
    }
}
