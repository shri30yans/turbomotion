//! Effects that add something around a layer's silhouette: a drop shadow and an outline.

use anyhow::{Context, Result};
use serde::Deserialize;
use skia_safe::{image_filters, Paint, TileMode};

use super::{param::Param, pass::Ctx};
use crate::{
    color::Rgb,
    layer::{solid_color, Layer},
};

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Shadow {
    pub dx: Param,
    pub dy: Param,
    pub blur: Param,
    pub color: Rgb,
    pub opacity: Param,
}

pub fn shadow(ctx: &Ctx, e: &Shadow, layer: &Layer) -> Result<Layer> {
    let blur = e.blur.at(ctx.t) as f32;
    let mut paint = Paint::default();
    paint.set_color_filter(solid_color(ctx.like, e.color, e.opacity.at(ctx.t) as f32)?);
    paint.set_image_filter(image_filters::blur((blur, blur), TileMode::Decal, None, None));
    let offset = (e.dx.at(ctx.t) as f32, e.dy.at(ctx.t) as f32);
    Layer::render(ctx.like, layer.width(), layer.height(), |c| {
        c.draw_image(layer.image(), offset, Some(&paint));
        c.draw_image(layer.image(), (0, 0), None);
    })
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Outline {
    pub width: Param,
    pub color: Rgb,
    pub opacity: Param,
}

pub fn outline(ctx: &Ctx, e: &Outline, layer: &Layer) -> Result<Layer> {
    let width = e.width.at(ctx.t) as f32;
    let mut paint = Paint::default();
    paint.set_color_filter(solid_color(ctx.like, e.color, e.opacity.at(ctx.t) as f32)?);
    paint.set_image_filter(image_filters::dilate((width, width), None, None).context("outline filter")?);
    Layer::render(ctx.like, layer.width(), layer.height(), |c| {
        c.draw_image(layer.image(), (0, 0), Some(&paint));
        c.draw_image(layer.image(), (0, 0), None);
    })
}
