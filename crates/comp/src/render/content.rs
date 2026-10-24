//! Painting a layer's own content into a comp-sized layer, through the layer's screen matrix.

use anyhow::Result;
use skia_safe::{Canvas, Paint, Rect};

use super::{color4f, comp_layer, shape, Ctx};
use crate::{
    eval::{Content, Frame, LayerState},
    plan::{CompPlan, LayerPlan},
};

pub fn paint_content(ctx: &Ctx, c: &Canvas, _frame: &Frame, _comp: &CompPlan, st: &LayerState, lp: &LayerPlan) -> Result<()> {
    c.save();
    c.concat_44(&st.screen.to_m44());
    let r = match &st.content {
        Content::Solid { color, size } => {
            let mut p = Paint::default();
            p.set_color4f(color4f(*color), None).set_anti_alias(true);
            c.draw_rect(Rect::from_wh(size[0] as f32, size[1] as f32), &p);
            Ok(())
        }
        Content::Shape { items } => shape::paint(c, items, &lp.shapes),
        Content::Precomp { frame: Some(inner) } => {
            let layer = comp_layer(ctx, c, inner)?;
            let mut p = Paint::default();
            p.set_anti_alias(true);
            c.draw_image_with_sampling_options(layer.image(), (0, 0), turbomotion_core::layer::linear(), Some(&p));
            Ok(())
        }
        Content::Precomp { frame: None } | Content::None | Content::Adjustment => Ok(()),
    };
    c.restore();
    r
}
