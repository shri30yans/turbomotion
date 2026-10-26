//! The stages a layer's picture goes through, one function each: painting, masks, mattes, the wipe and the floor
//! reflection, and the two halves of an adjustment layer. The frame graph (`graph/`) decides the order and what feeds what.

use anyhow::{Context, Result};
use skia_safe::{color_filters, image_filters, AlphaType, BlendMode, Canvas, ColorType, ISize, ImageInfo, Paint, PaintStyle, Path, PathFillType, Rect, Surface, TileMode};
use turbomotion_core::{effects, layer::Layer};

use super::{content::paint_content, Ctx};
use crate::{
    eval::{transit::Clip, Frame, LayerState},
    plan::CompPlan,
    schema::{MaskMode, MatteMode},
};

pub fn blank(like: &Canvas, w: i32, h: i32) -> Result<Surface> {
    let info = ImageInfo::new(ISize::new(w, h), ColorType::RGBAF16, AlphaType::Premul, None);
    let mut s = like.new_surface(&info, None).with_context(|| format!("no GPU memory for a {w}x{h} layer"))?;
    s.canvas().clear(skia_safe::Color::TRANSPARENT);
    Ok(s)
}

/// A transition's motion smear runs along the frame's x axis (degrees).
const HORIZONTAL: f64 = 0.0;

/// A blur or smear of this many pixels or fewer is not applied.
const MIN_BLUR: f64 = 0.3;

/// The layer's own picture before its effects: content, lighting, and the blur or smear a transition adds.
pub fn paint_stage(ctx: &Ctx, like: &Canvas, frame: &Frame, comp: &CompPlan, i: usize) -> Result<Layer> {
    let st = &frame.layers[i];
    let lp = &comp.layers[i];
    let img = Layer::try_render(like, frame.width as i32, frame.height as i32, |c| paint_content(ctx, c, frame, comp, st, lp, true))?;
    finish(ctx, like, frame, st, img)
}

/// Whether anything of [`finish`] applies to the layer.
pub fn needs_finish(st: &LayerState) -> bool {
    st.lit.is_some() || st.extra_blur > MIN_BLUR || st.extra_smear > MIN_BLUR
}

/// What is done to a layer's picture in the frame once it is placed: lighting, and the blur or smear of a transition.
pub fn finish(ctx: &Ctx, like: &Canvas, frame: &Frame, st: &LayerState, mut img: Layer) -> Result<Layer> {
    if st.lit.is_some() {
        img = super::light::apply(ctx, like, img, frame, st)?;
    }
    if st.extra_blur > MIN_BLUR {
        img = img.blurred(like, st.extra_blur)?;
    }
    if st.extra_smear > MIN_BLUR {
        img = effects::smear(&effects::Ctx::new(ctx.shaders, like, st.local_t), &img, st.extra_smear, HORIZONTAL)?;
    }
    Ok(img)
}

/// A wipe / iris, in the layer's screen-space box.
pub fn apply_clip(like: &Canvas, frame: &Frame, img: &Layer, bounds: [f64; 4], clip: Clip) -> Result<Layer> {
    let (x0, y0, x1, y1) = (bounds[0], bounds[1], bounds[2], bounds[3]);
    let (w, h) = ((x1 - x0).max(1.0), (y1 - y0).max(1.0));
    Layer::render(like, frame.width as i32, frame.height as i32, |c| {
        c.save();
        match clip {
            Clip::Rect(r) => {
                let rect = Rect::from_ltrb((x0 + r[0] * w) as f32, (y0 + r[1] * h) as f32, (x0 + r[2] * w) as f32, (y0 + r[3] * h) as f32);
                c.clip_rect(rect, None, Some(true));
            }
            Clip::Round { rect: r, radius } => {
                let rect = Rect::from_ltrb((x0 + r[0] * w) as f32, (y0 + r[1] * h) as f32, (x0 + r[2] * w) as f32, (y0 + r[3] * h) as f32);
                let rr = skia_safe::RRect::new_rect_xy(rect, radius as f32, radius as f32);
                c.clip_rrect(rr, None, Some(true));
            }
            Clip::Circle(k) => {
                let rad = (k * 0.5 * (w * w + h * h).sqrt()) as f32;
                let mut path = Path::new();
                path.add_circle((((x0 + x1) * 0.5) as f32, ((y0 + y1) * 0.5) as f32), rad, None);
                c.clip_path(&path, None, Some(true));
            }
        }
        c.draw_image(img.image(), (0, 0), None);
        c.restore();
    })
}

/// A faded, softened mirror of the layer's pixels, drawn on the floor beneath its bounding box.
pub fn floor_reflect(like: &Canvas, img: Layer, bounds: [f64; 4], r: crate::eval::ReflectState) -> Result<Layer> {
    if r.strength <= 0.01 {
        return Ok(img);
    }
    let (x0, y0, x1, y1) = (bounds[0], bounds[1], bounds[2], bounds[3]);
    let content_h = (y1 - y0 - r.gap).max(1.0) / (1.0 + r.fade);
    let floor = y0 + content_h;
    let n = (content_h * r.fade).max(1.0);
    let (fw, fh) = (img.width(), img.height());
    let dest = Rect::from_xywh(x0 as f32, (floor + r.gap) as f32, (x1 - x0).max(1.0) as f32, n as f32);
    let strip = Layer::render(like, fw, fh, |c| {
        c.save();
        c.clip_rect(dest, None, Some(true));
        // y_dest = (floor + gap) + (floor - y_src)  =>  y' = -y + 2*floor + gap
        c.concat(&skia_safe::Matrix::new_all(1.0, 0.0, 0.0, 0.0, -1.0, (2.0 * floor + r.gap) as f32, 0.0, 0.0, 1.0));
        c.draw_image(img.image(), (0, 0), None);
        c.restore();
        let fade = skia_safe::gradient_shader::linear(
            (skia_safe::Point::new(0.0, dest.top), skia_safe::Point::new(0.0, dest.bottom)),
            skia_safe::gradient_shader::GradientShaderColors::ColorsInSpace(
                &[skia_safe::Color4f::new(0.0, 0.0, 0.0, r.strength as f32), skia_safe::Color4f::new(0.0, 0.0, 0.0, 0.0)],
                None,
            ),
            None,
            skia_safe::TileMode::Clamp,
            None,
            None,
        );
        let mut mask = Paint::default();
        mask.set_shader(fade).set_blend_mode(BlendMode::DstIn);
        c.draw_rect(dest, &mask);
    })?;
    let soft = strip.blurred(like, r.blur)?;
    Layer::render(like, fw, fh, |c| {
        c.draw_image(img.image(), (0, 0), None);
        c.draw_image(soft.image(), (0, 0), None);
    })
}

/// `img` where `cov` has alpha.
pub fn multiply_by(like: &Canvas, img: &Layer, cov: &Layer) -> Result<Layer> {
    let mut out = blank(like, img.width(), img.height())?;
    out.canvas().draw_image(img.image(), (0, 0), None);
    let mut keep = Paint::default();
    keep.set_blend_mode(BlendMode::DstIn);
    out.canvas().draw_image(cov.image(), (0, 0), Some(&keep));
    Ok(Layer::from_image(out.image_snapshot()))
}

/// The union of a layer's masks as an alpha coverage layer.
pub fn mask_coverage(like: &Canvas, frame: &Frame, comp: &CompPlan, st: &LayerState) -> Result<Layer> {
    mask_coverage_in(like, (frame.width as i32, frame.height as i32), comp, st, |c| {
        c.concat_44(&st.screen.to_m44());
    })
}

/// The masks as coverage in a `w` x `h` picture; `place` sets the canvas up so the mask paths (layer pixels) land where
/// the picture is: through the screen matrix, or shifted to the layer's own buffer.
pub fn mask_coverage_in(like: &Canvas, (w, h): (i32, i32), comp: &CompPlan, st: &LayerState, place: impl Fn(&Canvas)) -> Result<Layer> {
    let lp = &comp.layers[st.index];
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
            place(c);
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
pub fn matte_coverage(like: &Canvas, frame: &Frame, matte: &Layer, mode: MatteMode, matte_opacity: f64) -> Result<Layer> {
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

/// What an adjustment layer reaches: its masks (or everywhere) times its opacity.
pub fn adjust_coverage(like: &Canvas, frame: &Frame, comp: &CompPlan, st: &LayerState) -> Result<Layer> {
    let lp = &comp.layers[st.index];
    let (w, h) = (frame.width as i32, frame.height as i32);
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
    Ok(cov)
}

/// An adjustment layer's result: `acc` becomes `lerp(below, changed, coverage)`.
pub fn adjust_mix(like: &Canvas, frame: &Frame, acc: &mut Surface, below: &Layer, changed: &Layer, cov: &Layer) -> Result<()> {
    let mut keep = blank(like, frame.width as i32, frame.height as i32)?;
    keep.canvas().draw_image(below.image(), (0, 0), None);
    let mut out_of = Paint::default();
    out_of.set_blend_mode(BlendMode::DstOut);
    keep.canvas().draw_image(cov.image(), (0, 0), Some(&out_of));
    let inside = multiply_by(like, changed, cov)?;
    let mut plus = Paint::default();
    plus.set_blend_mode(BlendMode::Plus);
    keep.canvas().draw_image(inside.image(), (0, 0), Some(&plus));
    acc.canvas().clear(skia_safe::Color::TRANSPARENT);
    acc.canvas().draw_image(keep.image_snapshot(), (0, 0), None);
    Ok(())
}
