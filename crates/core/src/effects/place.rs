//! Effects that move or fade a whole layer: opacity and a general 2D transform.

use anyhow::Result;
use serde::Deserialize;

use super::{param::Param, pass::Ctx};
use crate::layer::{place, place_about, Layer, Place};

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Opacity {
    pub value: Param,
}

pub fn opacity(ctx: &Ctx, e: &Opacity, layer: &Layer) -> Result<Layer> {
    Layer::render(ctx.like, layer.width(), layer.height(), |c| place(c, layer, &Place::at(f64::from(layer.width()) / 2.0, f64::from(layer.height()) / 2.0).opacity(e.value.at(ctx.t))))
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    pub x: Param,
    pub y: Param,
    pub scale: Param,
    pub rotation: Param,
    pub anchor_x: Param,
    pub anchor_y: Param,
}

pub fn transform(ctx: &Ctx, e: &Transform, layer: &Layer) -> Result<Layer> {
    let space = ctx.space_of(layer);
    // the pivot is a fraction of the layer's content, found in the picture's own pixels
    let (px, py) = space.at(e.anchor_x.at(ctx.t), e.anchor_y.at(ctx.t));
    let pivot = (px - space.origin[0], py - space.origin[1]);
    let at = (pivot.0 + e.x.at(ctx.t), pivot.1 + e.y.at(ctx.t));
    Layer::render(ctx.like, layer.width(), layer.height(), |c| place_about(c, layer, pivot, at, e.scale.at(ctx.t), e.rotation.at(ctx.t), 1.0))
}
