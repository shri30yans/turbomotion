//! Painting a layer's own content into a comp-sized layer, through the layer's screen matrix.

use anyhow::Result;
use skia_safe::{Canvas, Paint, Rect};

use super::{color4f, comp_layer, media, shape, text, Ctx};
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
        Content::Text(t) => text::paint(c, t),
        Content::Image { view, parallax } => media::paint_image(ctx, c, lp, view, parallax),
        Content::Video { view, src_time } => paint_video(ctx, c, lp, view, *src_time),
        Content::Precomp { frame: Some(inner) } => {
            let layer = comp_layer(ctx, c, inner)?;
            let mut p = Paint::default();
            p.set_anti_alias(true);
            c.draw_image_with_sampling_options(layer.image(), (0, 0), turbomotion_core::layer::linear(), Some(&p));
            Ok(())
        }
        Content::Texture { size, shader } => super::fx::generate(ctx, c, shader, *size, st.local_t),
        Content::Precomp { frame: None } | Content::None | Content::Adjustment => Ok(()),
    };
    c.restore();
    r
}

fn paint_video(ctx: &Ctx, c: &Canvas, lp: &LayerPlan, view: &crate::eval::media::View, src_time: f64) -> Result<()> {
    let crate::plan::Media::Video { path, info } = &lp.media else { return Ok(()) };
    let max_width = (ctx.plan.root().width as i32).max(1280) * 2;
    let frame = ctx.res.videos.frame(path, info, src_time, max_width)?;
    // the decoder may have scaled the frame down; keep the same fraction of it
    let k = f64::from(frame.width()) / f64::from(info.w);
    let v = crate::eval::media::View { src: view.src.map(|s| s * k), ..*view };
    media::draw(ctx, c, &frame, &v, None)
}
