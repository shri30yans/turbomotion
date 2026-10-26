//! Drawing a mode: the two SkSL programs behind the table's `Program` and `Dissolve` rows, and [`Blend::stage`].

use anyhow::{Context, Result};
use skia_safe::{BlendMode, Canvas, Paint, Rect, TileMode};

use super::{table, Blend, How};
use crate::{
    layer::{linear, Layer},
    shader::{Program, Uniforms},
};

pub struct Programs {
    /// The blender every `How::Program` mode shares. Each mode has a constant in the source, named after its row and
    /// numbered by its place among those rows, so the table is the only list of them.
    mix: Program,
    /// The pattern a dissolve keeps pixels by.
    dissolve: Program,
}

impl Programs {
    pub fn compile() -> Result<Self> {
        let codes: String = table::programs().enumerate().map(|(code, m)| format!("const int {} = {code};\n", m.name.to_ascii_uppercase())).collect();
        let mix = Program::blender("blend", &format!("{codes}{}", include_str!("../../shaders/blend.sksl")))?;
        Ok(Self { mix, dissolve: Program::shader("dissolve", include_str!("../../shaders/dissolve.sksl"))? })
    }

    /// `img` with each pixel kept whole (as its straight colour, opaque) when the pattern at that pixel falls below its
    /// alpha times `opacity`, and dropped otherwise. The pattern is a function of the pixel and `seed` alone.
    fn dissolved(&self, like: &Canvas, img: &Layer, opacity: f32, seed: u32) -> Result<Layer> {
        let source = img.image().to_shader((TileMode::Clamp, TileMode::Clamp), linear(), None).context("the layer image has no shader form")?;
        let shader = self.dissolve.instance_with(&[("seed", &[seed as f32]), ("opacity", &[opacity])], &[("source", source)])?;
        let (w, h) = (img.width(), img.height());
        Layer::try_render(like, w, h, |c| {
            let mut paint = Paint::default();
            paint.set_shader(shader);
            c.draw_rect(Rect::from_wh(w as f32, h as f32), &paint);
            Ok(())
        })
    }
}

impl Blend {
    /// Prepare to draw `img` in this mode: set what `paint` needs and return the picture to draw with it. `paint`'s
    /// alpha is the layer's opacity; `frame` is the frame number, which a dancing dissolve re-rolls its pattern by.
    ///
    /// A dissolve spends the opacity itself (it is the chance a pixel is kept), so the paint comes back opaque and the
    /// picture it returns is the dissolved one; every other mode returns `img` and leaves the opacity to the paint.
    pub fn stage(self, programs: &Programs, like: &Canvas, img: &Layer, paint: &mut Paint, frame: u32) -> Result<Layer> {
        match self.mode().how {
            How::Native(mode) => {
                paint.set_blend_mode(mode);
                Ok(img.clone())
            }
            How::Program => {
                let code = self.program_code().with_context(|| format!("{} is not one of the shared blender's modes", self.name()))?;
                paint.set_blender(programs.mix.blend(Uniforms::new().f(code as f64))?);
                Ok(img.clone())
            }
            How::Dissolve { dancing } => {
                let dissolved = programs.dissolved(like, img, paint.alpha_f(), if dancing { frame } else { 0 })?;
                paint.set_alpha_f(1.0);
                paint.set_blend_mode(BlendMode::SrcOver);
                Ok(dissolved)
            }
        }
    }
}
