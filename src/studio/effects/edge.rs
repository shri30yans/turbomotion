//! Effects that add something around a layer's silhouette: a drop shadow and an outline.

use anyhow::{Context, Result};
use serde::Deserialize;
use skia_safe::{image_filters, Paint, TileMode};

use super::{
    param::{value, Param},
    pass::Ctx,
};
use crate::studio::{
    layer::{solid_color, Layer},
    model::Rgb,
};

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Shadow {
    /// Offset in pixels.
    pub dx: Option<Param>,
    pub dy: Option<Param>,
    /// Gaussian sigma of the shadow's edge.
    pub blur: Option<Param>,
    pub color: Option<Rgb>,
    pub opacity: Option<Param>,
}

pub fn shadow(ctx: &Ctx, e: &Shadow, layer: &Layer) -> Result<Layer> {
    let blur = value(&e.blur, ctx.t, 16.0) as f32;
    let mut paint = Paint::default();
    paint.set_color_filter(solid_color(e.color.unwrap_or([0, 0, 0]), value(&e.opacity, ctx.t, 0.5) as f32));
    paint.set_image_filter(image_filters::blur((blur, blur), TileMode::Decal, None, None));
    let offset = (value(&e.dx, ctx.t, 0.0) as f32, value(&e.dy, ctx.t, 12.0) as f32);
    Layer::render(ctx.like, layer.width(), layer.height(), |c| {
        c.draw_image(layer.image(), offset, Some(&paint));
        c.draw_image(layer.image(), (0, 0), None);
    })
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Outline {
    /// Thickness in pixels.
    pub width: Option<Param>,
    pub color: Option<Rgb>,
    pub opacity: Option<Param>,
}

pub fn outline(ctx: &Ctx, e: &Outline, layer: &Layer) -> Result<Layer> {
    let width = value(&e.width, ctx.t, 4.0).max(0.0) as f32;
    let mut paint = Paint::default();
    paint.set_color_filter(solid_color(e.color.unwrap_or([255, 255, 255]), value(&e.opacity, ctx.t, 1.0) as f32));
    paint.set_image_filter(image_filters::dilate((width, width), None, None).context("outline filter")?);
    Layer::render(ctx.like, layer.width(), layer.height(), |c| {
        c.draw_image(layer.image(), (0, 0), Some(&paint));
        c.draw_image(layer.image(), (0, 0), None);
    })
}
