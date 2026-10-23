//! Graded camera move over a photograph, with optional depth-map parallax.

use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use skia_safe::{
    gpu::{DirectContext, Mipmapped}, runtime_effect::ChildPtr, Canvas, Data, FilterMode, Image, MipmapMode, Paint, Rect as SkRect,
    RuntimeEffect, SamplingOptions, TileMode,
};

use crate::{
    camera::{parse_view, Move, Placement, View},
    film::{Asset, Film, Shot},
    image::load_texture,
};

const SKSL: &str = include_str!("photo.sksl");
const VIGNETTE: f32 = 0.32;
/// Stat shots darken the photograph and pull the vignette in behind the big number.
const STAT_DIM: f32 = 0.58;
const STAT_VIGNETTE: f32 = 0.5;

#[derive(Clone, Copy)]
pub enum Grade {
    Warm = 0,
    Archival = 1,
    Painting = 2,
}

impl Grade {
    pub fn parse(name: &str) -> Result<Self> {
        match name {
            "warm" => Ok(Self::Warm),
            "archival" => Ok(Self::Archival),
            "painting" => Ok(Self::Painting),
            other => bail!("unknown grade {other:?}"),
        }
    }

    /// Illustrations and generated art default to the neutral painting grade.
    fn default_for(asset: &Asset) -> Self {
        if matches!(asset.kind.as_str(), "illustration" | "generated") { Self::Painting } else { Self::Warm }
    }
}

pub struct PhotoEffect(RuntimeEffect);

impl PhotoEffect {
    pub fn compile() -> Result<Self> {
        RuntimeEffect::make_for_shader(SKSL, None).map(Self).map_err(|e| anyhow!("photo shader: {e}"))
    }
}

/// One shot's decoded textures and camera, ready to draw at any frame.
pub struct Photo {
    image: Image,
    depth: Option<Image>,
    motion: Move,
    grade: Grade,
    dim: f32,
    vignette: f32,
}

/// How a shot grades its photograph.
struct Look {
    depth: bool,
    grade: Option<Grade>,
    dim: f32,
    vignette: f32,
}

impl Photo {
    /// A photo or illustration shot: grade, dim and depth parallax come from the shot's parameters;
    /// `house` is the grade the shot type imposes when the shot names none.
    pub fn for_shot(context: &mut DirectContext, film: &Film, public: &Path, shot: &Shot, house: Option<Grade>) -> Result<Self> {
        let look = Look {
            depth: shot.param("depth") == Some("1"),
            grade: shot.param("grade").map(Grade::parse).transpose()?.or(house),
            dim: shot.param("dim").map(str::parse).transpose().context("dim is not a number")?.unwrap_or(0.0),
            vignette: VIGNETTE,
        };
        Self::prepare(context, film, public, shot, look)
    }

    /// A stat shot's backdrop: warm grade, dimmed, flat (no parallax).
    pub fn dimmed(context: &mut DirectContext, film: &Film, public: &Path, shot: &Shot) -> Result<Self> {
        let look = Look { depth: false, grade: Some(Grade::Warm), dim: STAT_DIM, vignette: STAT_VIGNETTE };
        Self::prepare(context, film, public, shot, look)
    }

    fn prepare(context: &mut DirectContext, film: &Film, public: &Path, shot: &Shot, look: Look) -> Result<Self> {
        let id = shot.args.first().with_context(|| format!("{}: shot without an asset", shot.id))?;
        let asset = film.assets.get(id).with_context(|| format!("{}: unknown asset {id:?}", shot.id))?;
        let depth_file = asset.depth.as_deref().filter(|_| look.depth);
        let view = |key: &str, default: View| shot.param(key).map(parse_view).unwrap_or(Ok(default));
        Ok(Self {
            image: load_texture(context, &public.join(&asset.file), Mipmapped::No)?,
            depth: depth_file.map(|f| load_texture(context, &public.join(f), Mipmapped::No)).transpose()?,
            motion: Move {
                from: view("from", View { cx: 0.5, cy: 0.5, zoom: 1.12 })?,
                to: view("to", View { cx: 0.5, cy: 0.5, zoom: 1.0 })?,
                duration: shot.duration,
                has_depth: depth_file.is_some(),
            },
            grade: look.grade.unwrap_or_else(|| Grade::default_for(asset)),
            dim: look.dim,
            vignette: look.vignette,
        })
    }

    /// Where the camera puts the photograph at `seconds` into the shot.
    pub fn placement(&self, film: &Film, seconds: f32) -> Placement {
        let aspect = self.image.width() as f32 / self.image.height() as f32;
        self.motion.at(seconds, aspect, film.width as f32, film.height as f32)
    }

    pub fn draw(&self, canvas: &Canvas, effect: &PhotoEffect, film: &Film, seconds: f32) -> Result<()> {
        let (w, h) = (film.width as f32, film.height as f32);
        let place = self.placement(film, seconds);
        let sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::None);
        let child = |image: &Image| image.to_shader((TileMode::Clamp, TileMode::Clamp), sampling, None).map(ChildPtr::Shader);
        let image = child(&self.image).context("image shader")?;
        // Without a depth map the shader never samples `dep`; bind the photo to satisfy the child slot.
        let depth = child(self.depth.as_ref().unwrap_or(&self.image)).context("depth shader")?;
        let size = |i: &Image| [i.width() as f32, i.height() as f32];
        let depth_size = size(self.depth.as_ref().unwrap_or(&self.image));
        let mut uniforms = Vec::with_capacity(24 * 4);
        let mut push = |values: &[f32]| values.iter().for_each(|v| uniforms.extend_from_slice(&v.to_ne_bytes()));
        push(&[w, h]);
        push(&[place.rect.x, place.rect.y, place.rect.w, place.rect.h]);
        push(&size(&self.image));
        push(&depth_size);
        push(&place.shift);
        push(&place.focus);
        push(&[place.dolly, f32::from(self.depth.is_some()), self.grade as u8 as f32, self.vignette, self.dim]);
        let shader = effect.0.make_shader(Data::new_copy(&uniforms), &[image, depth], None).context("photo shader instance")?;
        let mut paint = Paint::default();
        paint.set_shader(shader);
        canvas.draw_rect(SkRect::from_wh(w, h), &paint);
        Ok(())
    }
}
