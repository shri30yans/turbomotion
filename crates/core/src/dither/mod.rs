//! Dither at the one place a picture becomes 8-bit. Every picture is half-float; the finished frame is rounded to a byte
//! per channel exactly once, when it is drawn into the BGRA frame surface. With no dither that rounding is Skia's own,
//! to nearest, and nothing here runs. With one, the same draw goes through a shader that adds a tiled threshold before
//! the floor (see `shaders/dither.sksl`), on the numbers the file will hold, after the output profile's transfer: the
//! error it leaves is zero-mean and at most one level, and a smooth dark gradient shows no bands.
//!
//! What it cannot do is outlast the codec: a hardware encoder (`h264_videotoolbox`) and any 4:2:0 or lossy encode
//! re-quantise the frames, so the pattern survives only as far as that encode's own rounding (see
//! `parity/specs/dither.md`). The real fix for banding in a codec is a 10-bit export, which is a separate piece of work.

mod blue_noise;
mod patterns;
#[cfg(test)]
mod tests;

use anyhow::{anyhow, Result};
use serde::Deserialize;
use skia_safe::{images, AlphaType, Canvas, ColorType, Data, FilterMode, Image, ImageInfo, MipmapMode, Paint, SamplingOptions, Shader, TileMode};

use crate::shader::{Program, Uniforms};

/// How the finished frame is rounded to 8 bits: the composition key `color.dither`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Dither {
    /// Round to nearest, as every film always has (default).
    #[default]
    None,
    /// A 16 x 16 Bayer matrix: regular and cheap, with a faint cross-hatch.
    Ordered,
    /// A 64 x 64 void-and-cluster tile: the error is spread without a visible pattern.
    BlueNoise,
}

impl Dither {
    pub const NAMES: [&'static str; 3] = ["none", "ordered", "blue_noise"];

    fn tile(self) -> Result<Option<patterns::Tile>> {
        Ok(match self {
            Self::None => None,
            Self::Ordered => Some(patterns::ordered()),
            Self::BlueNoise => Some(blue_noise::tile()?),
        })
    }
}

/// The compiled program and pattern of a chosen dither, made once per renderer.
pub struct Dithering {
    program: Program,
    ranks: Shader,
}

impl Dithering {
    /// The dither `kind` names; none for [`Dither::None`], which leaves the frame's own rounding alone.
    pub fn new(kind: Dither) -> Result<Option<Self>> {
        let Some(tile) = kind.tile()? else { return Ok(None) };
        let side = tile.side as i32;
        let info = ImageInfo::new((side, side), ColorType::Alpha8, AlphaType::Premul, None);
        let image = images::raster_from_data(&info, Data::new_copy(&tile.ranks), tile.side).ok_or_else(|| anyhow!("could not make the dither tile"))?;
        let nearest = SamplingOptions::new(FilterMode::Nearest, MipmapMode::None);
        let ranks = image.to_shader((TileMode::Repeat, TileMode::Repeat), nearest, None).ok_or_else(|| anyhow!("could not tile the dither pattern"))?;
        Ok(Some(Self { program: Program::shader("dither", include_str!("../shaders/dither.sksl"))?, ranks }))
    }

    /// Draw `picture` over the whole of `canvas`, rounded to 8 bits through the pattern. `canvas` is the frame surface.
    pub fn draw(&self, canvas: &Canvas, picture: &Image) -> Result<()> {
        let shader = picture.to_shader((TileMode::Clamp, TileMode::Clamp), SamplingOptions::new(FilterMode::Nearest, MipmapMode::None), None).ok_or_else(|| anyhow!("could not sample the picture"))?;
        let mut paint = Paint::default();
        paint.set_shader(self.program.instance(Uniforms::new(), &[shader, self.ranks.clone()])?);
        canvas.draw_paint(&paint);
        Ok(())
    }
}
