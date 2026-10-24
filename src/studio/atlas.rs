//! The map atlas: coverage masks of the country and state outlines for the flat map, and the equirectangular textures
//! the globe shader samples. Heavy rasters are built once and cached; everything else is drawn per frame.

use std::{cell::OnceCell, path::{Path as FsPath, PathBuf}, rc::Rc};

use anyhow::{Context, Result};
use skia_safe::{surfaces, AlphaType, Canvas, ColorType, Image, ImageInfo, Paint, PaintStyle, Path, Point, Rect};

use super::{
    cache::Cache,
    color::fill,
    geo::{Geo, Ring, View},
    layer::{solid_color, Layer},
    model::{Rgb, H, W},
    shader::Uniforms,
    shaders::Shaders,
};

/// Equirectangular texture size: 0.0667 degrees per pixel.
pub const TEXTURE: (i32, i32) = (5400, 2700);
/// Opacity of the country borders on the flat map.
const BORDER_OPACITY: f32 = 0.9;

/// How an outline set becomes coverage.
#[derive(Clone, Copy)]
enum Brush {
    /// The union of the filled outlines (abutting outlines add up to full coverage, overlapping ones to one).
    Fill,
    /// Anti-aliased hairlines of this width.
    Stroke(f32),
}

type BaseKey = ([u64; 3], Rgb, Rgb);

/// The two globe textures: land (filled outlines) and borders.
pub struct Equirect {
    pub land: Image,
    pub border: Image,
}

pub struct Atlas {
    dir: PathBuf,
    geo: OnceCell<Geo>,
    bases: Cache<BaseKey, Layer>,
    equirect: OnceCell<Equirect>,
    highlights: Cache<Vec<String>, Image>,
}

impl Atlas {
    pub fn new(dir: &FsPath) -> Self {
        Self { dir: dir.to_path_buf(), geo: OnceCell::new(), bases: Cache::new(24), equirect: OnceCell::new(), highlights: Cache::new(8) }
    }

    /// The outlines, read from `geo.json` on first use.
    pub fn geo(&self) -> Result<&Geo> {
        if let Some(geo) = self.geo.get() {
            return Ok(geo);
        }
        let geo = Geo::load(&self.dir)?;
        Ok(self.geo.get_or_init(|| geo))
    }

    /// Land and borders of every country in `view`, in the given colours (cached while the view holds still).
    pub fn base(&self, shaders: &Shaders, cv: &Canvas, view: &View, land: Rgb, border: Rgb) -> Result<Rc<Layer>> {
        let key = (view.key().map(f64::to_bits), land, border);
        self.bases.get_or_try_insert(key, || {
            let geo = self.geo()?;
            let rings: Vec<&Ring> = geo.all_country_rings().collect();
            let land_mask = coverage(shaders, cv, &rings, view, Brush::Fill)?;
            let border_mask = coverage(shaders, cv, &rings, view, Brush::Stroke(1.0))?;
            Layer::render(cv, W, H, |c| {
                tint(c, &land_mask, land, 1.0);
                tint(c, &border_mask, border, BORDER_OPACITY);
            })
        })
    }

    /// Coverage of the union of `rings` in `view`.
    pub fn region(&self, shaders: &Shaders, cv: &Canvas, view: &View, rings: &[&Ring]) -> Result<Layer> {
        coverage(shaders, cv, rings, view, Brush::Fill)
    }

    /// Coverage of the outlines of `rings` in `view`, `width` pixels wide.
    pub fn outline(&self, shaders: &Shaders, cv: &Canvas, view: &View, rings: &[&Ring], width: f64) -> Result<Layer> {
        coverage(shaders, cv, rings, view, Brush::Stroke(width as f32))
    }

    /// The globe's land and border textures.
    pub fn equirect(&self) -> Result<&Equirect> {
        if let Some(e) = self.equirect.get() {
            return Ok(e);
        }
        let rings: Vec<&Ring> = self.geo()?.all_country_rings().collect();
        let built = Equirect { land: texture(&rings, Brush::Fill)?, border: texture(&rings, Brush::Stroke(2.0))? };
        Ok(self.equirect.get_or_init(|| built))
    }

    /// Texture of the named countries only (for highlights), cached by the name list.
    pub fn countries_texture(&self, names: &[String]) -> Result<Rc<Image>> {
        self.highlights.get_or_try_insert(names.to_vec(), || {
            let rings = self.geo()?.country_rings(names)?;
            texture(&rings, Brush::Fill)
        })
    }
}

/// Draw `mask`'s coverage in `color` at `alpha`.
pub fn tint(c: &Canvas, mask: &Layer, color: Rgb, alpha: f32) {
    let mut paint = Paint::default();
    paint.set_color_filter(solid_color(color, alpha));
    c.draw_image(mask.image(), (0, 0), Some(&paint));
}

fn polygon(ring: &Ring, project: impl Fn([f32; 2]) -> (f32, f32)) -> Path {
    let points: Vec<Point> = ring.iter().map(|&v| project(v).into()).collect();
    Path::polygon(&points, true, None, None)
}

/// A frame-sized white-on-transparent mask of the outlines as seen in `view`.
fn coverage(shaders: &Shaders, cv: &Canvas, rings: &[&Ring], view: &View, brush: Brush) -> Result<Layer> {
    let mut paint = fill([255, 255, 255]);
    paint.set_anti_alias(true);
    match brush {
        Brush::Fill => {
            paint.set_blender(shaders.union.blend(Uniforms::new())?);
        }
        Brush::Stroke(width) => {
            paint.set_style(PaintStyle::Stroke).set_stroke_width(width);
        }
    }
    let frame = Rect::from_wh(W as f32, H as f32);
    Layer::render(cv, W, H, |c| {
        for ring in rings {
            let path = polygon(ring, |v| view.vertex(v));
            if path.bounds().intersects(frame) {
                c.draw_path(&path, &paint);
            }
        }
    })
}

/// An equirectangular alpha mask of `rings`, rounded to whole texels like a raster polygon fill.
fn texture(rings: &[&Ring], brush: Brush) -> Result<Image> {
    let (w, h) = TEXTURE;
    let mut surface = surfaces::raster(&ImageInfo::new((w, h), ColorType::Alpha8, AlphaType::Premul, None), None, None).context("equirectangular surface")?;
    let mut paint = fill([255, 255, 255]);
    match brush {
        Brush::Fill => {
            paint.set_anti_alias(false);
        }
        Brush::Stroke(width) => {
            paint.set_anti_alias(true).set_style(PaintStyle::Stroke).set_stroke_width(width);
        }
    }
    let texel = |[lon, lat]: [f32; 2]| ((((f64::from(lon) + 180.0) / 360.0 * f64::from(w)).round() + 0.5) as f32, (((90.0 - f64::from(lat)) / 180.0 * f64::from(h)).round() + 0.5) as f32);
    for ring in rings {
        surface.canvas().draw_path(&polygon(ring, texel), &paint);
    }
    Ok(surface.image_snapshot())
}
