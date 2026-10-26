//! Frosted glass: blur what is already composited behind a layer and draw the layer on top.

use anyhow::{ensure, Result};
use serde::Deserialize;
use skia_safe::Canvas;

use super::param::Param;
use crate::layer::Layer;

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Backdrop {
    pub blur: Param,
}

/// Gaussian-blur `behind` and lay `painted` over it (standard src-over). Both pictures are comp-sized.
pub fn over(like: &Canvas, painted: &Layer, behind: &Layer, sigma: f64) -> Result<Layer> {
    ensure!(
        painted.width() == behind.width() && painted.height() == behind.height(),
        "the backdrop and the layer must be the same size ({}x{} vs {}x{})",
        behind.width(),
        behind.height(),
        painted.width(),
        painted.height()
    );
    let blurred = behind.blurred(like, sigma)?;
    Layer::try_render(like, painted.width(), painted.height(), |c| {
        c.draw_image(blurred.image(), (0, 0), None);
        c.draw_image(painted.image(), (0, 0), None);
        Ok(())
    })
}

pub fn backdrop(_ctx: &super::pass::Ctx, e: &Backdrop, _layer: &Layer) -> Result<Layer> {
    let _ = e.blur;
    anyhow::bail!("the backdrop effect is applied by the composition renderer with a snapshot of what is behind the layer")
}
