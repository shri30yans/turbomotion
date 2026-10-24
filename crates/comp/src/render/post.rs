//! What happens to a painted layer before it lands: blur from transitions, effects, masks, track matte; and the
//! adjustment layer, which applies the same machinery to everything composited below it.

use anyhow::{Context, Result};
use skia_safe::{color_filters, image_filters, AlphaType, BlendMode, Canvas, ColorType, ISize, ImageInfo, Paint, PaintStyle, PathFillType, Surface, TileMode};
use turbomotion_core::{effects, layer::Layer};

use super::{content::paint_content, Ctx};
use crate::{
    eval::{Frame, LayerState},
    plan::CompPlan,
    schema::{MaskMode, MatteMode},
};

fn blank(like: &Canvas, w: i32, h: i32) -> Result<Surface> {
    let info = ImageInfo::new(ISize::new(w, h), ColorType::RGBAF16, AlphaType::Premul, None);
    let mut s = like.new_surface(&info, None).with_context(|| format!("no GPU memory for a {w}x{h} layer"))?;
    s.canvas().clear(skia_safe::Color::TRANSPARENT);
    Ok(s)
}

/// The layer fully processed except for its opacity and blend: content, effects, masks and its own matte.
pub fn layer_image(ctx: &Ctx, like: &Canvas, frame: &Frame, comp: &CompPlan, i: usize) -> Result<Layer> {
    let st = &frame.layers[i];
    let lp = &comp.layers[i];
    let mut img = Layer::try_render(like, frame.width as i32, frame.height as i32, |c| paint_content(ctx, c, frame, comp, st, lp))?;
    if st.extra_blur > 0.3 {
        img = img.blurred(like, st.extra_blur)?;
    }
    if st.extra_smear > 0.3 {
        let e: effects::Effect = serde_json::from_value(serde_json::json!({"effect": "directional_blur", "distance": st.extra_smear, "angle": 0}))?;
        img = effects::apply(ctx.shaders, like, img, &[e], st.local_t)?;
    }
    if !lp.fx.is_empty() {
        img = super::fx::chain(ctx, like, img, &lp.fx, &st.fx, st.local_t)?;
    }
    if !lp.masks.is_empty() {
        let cov = mask_coverage(like, frame, comp, st)?;
        img = multiply_by(like, frame, &img, &cov)?;
    }
    if let Some((src, mode)) = lp.matte {
        let m = layer_image(ctx, like, frame, comp, src)?;
        let cov = matte_coverage(like, frame, &m, mode, frame.layers[src].opacity)?;
        img = multiply_by(like, frame, &img, &cov)?;
    }
    Ok(img)
}

/// `img` where `cov` has alpha.
fn multiply_by(like: &Canvas, frame: &Frame, img: &Layer, cov: &Layer) -> Result<Layer> {
    let mut out = blank(like, frame.width as i32, frame.height as i32)?;
    out.canvas().draw_image(img.image(), (0, 0), None);
    let mut keep = Paint::default();
    keep.set_blend_mode(BlendMode::DstIn);
    out.canvas().draw_image(cov.image(), (0, 0), Some(&keep));
    Ok(Layer::from_image(out.image_snapshot()))
}

/// The union of a layer's masks as an alpha coverage layer.
fn mask_coverage(like: &Canvas, frame: &Frame, comp: &CompPlan, st: &LayerState) -> Result<Layer> {
    let lp = &comp.layers[st.index];
    let (w, h) = (frame.width as i32, frame.height as i32);
    let mut cov = blank(like, w, h)?;
    if matches!(lp.def.common.masks.first().map(|m| m.mode), Some(MaskMode::Subtract | MaskMode::Difference)) {
        cov.canvas().clear(skia_safe::Color::WHITE);
    }
    for ((m, geo), ms) in lp.def.common.masks.iter().zip(&lp.masks).zip(&st.masks) {
        let mut one = blank(like, w, h)?;
        let mut path = geo.at(ms.morph);
        if m.invert {
            path.set_fill_type(PathFillType::InverseWinding);
        }
        let mut paint = Paint::default();
        paint.set_anti_alias(true).set_color4f(skia_safe::Color4f::new(1.0, 1.0, 1.0, ms.opacity as f32), None);
        {
            let c = one.canvas();
            c.concat_44(&st.screen.to_m44());
            c.draw_path(&path, &paint);
            if ms.expansion > 0.0 {
                paint.set_style(PaintStyle::Stroke).set_stroke_width((ms.expansion * 2.0) as f32);
                c.draw_path(&path, &paint);
            } else if ms.expansion < 0.0 {
                paint.set_style(PaintStyle::Stroke).set_stroke_width((-ms.expansion * 2.0) as f32).set_blend_mode(BlendMode::DstOut).set_color(skia_safe::Color::WHITE);
                c.draw_path(&path, &paint);
            }
        }
        let mut combine = Paint::default();
        combine.set_blend_mode(match m.mode {
            MaskMode::Add => BlendMode::SrcOver,
            MaskMode::Subtract => BlendMode::DstOut,
            MaskMode::Intersect => BlendMode::DstIn,
            MaskMode::Difference => BlendMode::Xor,
        });
        if ms.feather > 0.2 {
            combine.set_image_filter(image_filters::blur((ms.feather as f32 / 2.0, ms.feather as f32 / 2.0), TileMode::Decal, None, None));
        }
        cov.canvas().draw_image(one.image_snapshot(), (0, 0), Some(&combine));
    }
    Ok(Layer::from_image(cov.image_snapshot()))
}

/// A track matte's alpha or brightness as an alpha coverage layer.
fn matte_coverage(like: &Canvas, frame: &Frame, matte: &Layer, mode: MatteMode, matte_opacity: f64) -> Result<Layer> {
    let (w, h) = (frame.width as i32, frame.height as i32);
    let mut cov = blank(like, w, h)?;
    let mut paint = Paint::default();
    paint.set_alpha_f(matte_opacity as f32);
    match mode {
        MatteMode::Alpha => cov.canvas().draw_image(matte.image(), (0, 0), Some(&paint)),
        MatteMode::AlphaInverted => {
            paint.set_color_filter(color_filters::matrix_row_major(&[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 1.0], None));
            cov.canvas().draw_image(matte.image(), (0, 0), Some(&paint))
        }
        MatteMode::Luma | MatteMode::LumaInverted => {
            // Flatten against black so the premultiplied brightness is the brightness, then move it into alpha.
            let mut flat = blank(like, w, h)?;
            flat.canvas().clear(skia_safe::Color::BLACK);
            flat.canvas().draw_image(matte.image(), (0, 0), Some(&paint));
            let (k, off) = if mode == MatteMode::Luma { (1.0f32, 0.0f32) } else { (-1.0, 1.0) };
            let mut p = Paint::default();
            p.set_color_filter(color_filters::matrix_row_major(
                &[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.2126 * k, 0.7152 * k, 0.0722 * k, 0.0, off],
                None,
            ));
            cov.canvas().draw_image(flat.image_snapshot(), (0, 0), Some(&p))
        }
    };
    Ok(Layer::from_image(cov.image_snapshot()))
}

/// An adjustment layer: what is composited so far becomes `lerp(below, effects(below), coverage × opacity)`.
pub fn adjust(ctx: &Ctx, like: &Canvas, acc: &mut Surface, frame: &Frame, comp: &CompPlan, st: &LayerState) -> Result<()> {
    let lp = &comp.layers[st.index];
    let (w, h) = (frame.width as i32, frame.height as i32);
    let below = Layer::from_image(acc.image_snapshot());
    let changed = super::fx::chain(ctx, like, Layer::from_image(below.image().clone()), &lp.fx, &st.fx, st.local_t)?;
    // coverage: the masks (or everywhere) times the layer's opacity
    let mut cov = if lp.masks.is_empty() {
        let mut s = blank(like, w, h)?;
        s.canvas().clear(skia_safe::Color::WHITE);
        Layer::from_image(s.image_snapshot())
    } else {
        mask_coverage(like, frame, comp, st)?
    };
    if st.opacity < 0.999 {
        let mut s = blank(like, w, h)?;
        let mut p = Paint::default();
        p.set_alpha_f(st.opacity as f32);
        s.canvas().draw_image(cov.image(), (0, 0), Some(&p));
        cov = Layer::from_image(s.image_snapshot());
    }
    let mut keep = blank(like, w, h)?;
    keep.canvas().draw_image(below.image(), (0, 0), None);
    let mut out_of = Paint::default();
    out_of.set_blend_mode(BlendMode::DstOut);
    keep.canvas().draw_image(cov.image(), (0, 0), Some(&out_of));
    let inside = multiply_by(like, frame, &changed, &cov)?;
    let mut plus = Paint::default();
    plus.set_blend_mode(BlendMode::Plus);
    keep.canvas().draw_image(inside.image(), (0, 0), Some(&plus));
    acc.canvas().clear(skia_safe::Color::TRANSPARENT);
    acc.canvas().draw_image(keep.image_snapshot(), (0, 0), None);
    Ok(())
}
