//! Frosted glass: blur what is already composited behind a layer and draw the layer on top.

use anyhow::{ensure, Result};
use serde::Deserialize;

use super::{
    param::Param,
    pass::Ctx,
    quality::{soften, Quality},
};
use crate::layer::Layer;

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Backdrop {
    pub blur: Param,
    pub quality: Quality,
}

/// Gaussian-blur `behind` and lay `painted` over it (standard src-over). Both pictures are comp-sized.
pub fn over(ctx: &Ctx, painted: &Layer, behind: &Layer, e: &Backdrop) -> Result<Layer> {
    ensure!(
        painted.width() == behind.width() && painted.height() == behind.height(),
        "the backdrop and the layer must be the same size ({}x{} vs {}x{})",
        behind.width(),
        behind.height(),
        painted.width(),
        painted.height()
    );
    let blurred = soften(ctx, behind, e.blur.at(ctx.t), e.quality)?;
    Layer::try_render(ctx.like, painted.width(), painted.height(), |c| {
        c.draw_image(blurred.image(), (0, 0), None);
        c.draw_image(painted.image(), (0, 0), None);
        Ok(())
    })
}

pub fn backdrop(_ctx: &super::pass::Ctx, _e: &Backdrop, _layer: &Layer) -> Result<Layer> {
    anyhow::bail!("the backdrop effect is applied by the composition renderer with a snapshot of what is behind the layer")
}
