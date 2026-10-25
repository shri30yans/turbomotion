//! Effects that move or fade a whole layer: opacity and a general 2D transform.

use anyhow::Result;
use serde::Deserialize;

use super::{param::Param, pass::Ctx};
use crate::layer::{place, Layer, Place};

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
    let (w, h) = (f64::from(layer.width()), f64::from(layer.height()));
    let (ax, ay) = (e.anchor_x.at(ctx.t), e.anchor_y.at(ctx.t));
    let mut at = Place::at(ax * w + e.x.at(ctx.t), ay * h + e.y.at(ctx.t)).scale(e.scale.at(ctx.t)).rot(e.rotation.at(ctx.t));
    at.anchor = (ax, ay);
    Layer::render(ctx.like, layer.width(), layer.height(), |c| place(c, layer, &at))
}
