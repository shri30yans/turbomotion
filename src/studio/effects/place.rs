//! Effects that move or fade a whole layer: opacity and a general 2D transform.

use anyhow::Result;
use serde::Deserialize;

use super::{
    param::{value, Param},
    pass::Ctx,
};
use crate::studio::layer::{place, Layer, Place};

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Opacity {
    /// 0 = invisible, 1 = unchanged.
    pub value: Option<Param>,
}

pub fn opacity(ctx: &Ctx, e: &Opacity, layer: &Layer) -> Result<Layer> {
    Layer::render(ctx.like, layer.width(), layer.height(), |c| place(c, layer, &Place::at(f64::from(layer.width()) / 2.0, f64::from(layer.height()) / 2.0).opacity(value(&e.value, ctx.t, 1.0))))
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    /// Move by this many pixels.
    pub x: Option<Param>,
    pub y: Option<Param>,
    /// Uniform scale (1 = unchanged).
    pub scale: Option<Param>,
    /// Clockwise degrees.
    pub rotation: Option<Param>,
    /// The point scale and rotation pivot about, as fractions of the frame (default centre).
    pub anchor_x: Option<Param>,
    pub anchor_y: Option<Param>,
}

pub fn transform(ctx: &Ctx, e: &Transform, layer: &Layer) -> Result<Layer> {
    let (w, h) = (f64::from(layer.width()), f64::from(layer.height()));
    let (ax, ay) = (value(&e.anchor_x, ctx.t, 0.5), value(&e.anchor_y, ctx.t, 0.5));
    let mut at = Place::at(ax * w + value(&e.x, ctx.t, 0.0), ay * h + value(&e.y, ctx.t, 0.0)).scale(value(&e.scale, ctx.t, 1.0)).rot(value(&e.rotation, ctx.t, 0.0));
    at.anchor = (ax, ay);
    Layer::render(ctx.like, layer.width(), layer.height(), |c| place(c, layer, &at))
}
