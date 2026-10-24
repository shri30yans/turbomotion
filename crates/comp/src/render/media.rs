//! Painting pictures: still images and video frames into the layer's viewport, with sampling chosen by the scale
//! (mipmaps when shrinking, a cubic filter when enlarging) and optional depth parallax.

use anyhow::{anyhow, Context, Result};
use skia_safe::{canvas::SrcRectConstraint, CubicResampler, FilterMode, Image, Matrix, MipmapMode, Paint, Rect, SamplingOptions, TileMode};
use turbomotion_core::shader::Uniforms;

use super::Ctx;
use crate::{
    eval::media::{ParallaxState, View},
    plan::{LayerPlan, Media},
};

/// How a draw at the canvas's current transform should sample: mipmapped when it shrinks, a Mitchell cubic when it grows.
fn sampling(c: &skia_safe::Canvas, view: &View) -> SamplingOptions {
    let m: Matrix = c.local_to_device_as_3x3();
    let device = f64::from(m.scale_x()).hypot(f64::from(m.skew_y()));
    let into_viewport = view.dest[2] / view.src[2].max(1e-9);
    if device * into_viewport < 0.999 {
        SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear)
    } else {
        SamplingOptions::from(CubicResampler::mitchell())
    }
}

pub fn paint_image(ctx: &Ctx, c: &skia_safe::Canvas, lp: &LayerPlan, view: &View, parallax: &Option<ParallaxState>) -> Result<()> {
    let Media::Image { path, depth, .. } = &lp.media else { return Ok(()) };
    let img = ctx.res.images.get(path).context("loading the image")?;
    let depth_img = match (parallax, depth) {
        (Some(_), Some((dp, _))) => Some(ctx.res.images.get(dp).context("loading the depth map")?),
        _ => None,
    };
    draw(ctx, c, &img, view, parallax.as_ref().zip(depth_img.as_deref()))
}

pub fn draw(ctx: &Ctx, c: &skia_safe::Canvas, img: &Image, view: &View, parallax: Option<(&ParallaxState, &Image)>) -> Result<()> {
    let [sx, sy, sw, sh] = view.src.map(|v| v as f32);
    let [dx, dy, dw, dh] = view.dest.map(|v| v as f32);
    c.save();
    c.clip_rect(Rect::from_wh(view.viewport[0] as f32, view.viewport[1] as f32), None, Some(true));
    let opts = sampling(c, view);
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    let dest = Rect::from_xywh(dx, dy, dw, dh);
    match parallax {
        None => {
            c.draw_image_rect_with_sampling_options(img, Some((&Rect::from_xywh(sx, sy, sw, sh), SrcRectConstraint::Fast)), dest, opts, &paint);
        }
        Some((p, depth)) => {
            // picture pixels -> viewport pixels
            let (kx, ky) = (dw / sw, dh / sh);
            let mut to_view = Matrix::translate((dx - sx * kx, dy - sy * ky));
            to_view.pre_scale((kx, ky), None);
            let mut depth_to_view = to_view;
            depth_to_view.pre_scale((img.width() as f32 / depth.width() as f32, img.height() as f32 / depth.height() as f32), None);
            let s_img = img.to_shader((TileMode::Clamp, TileMode::Clamp), opts, Some(&to_view)).ok_or_else(|| anyhow!("could not sample the image"))?;
            let s_depth = depth.to_shader((TileMode::Clamp, TileMode::Clamp), SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear), Some(&depth_to_view)).ok_or_else(|| anyhow!("could not sample the depth map"))?;
            // sampling stays inside the part of the picture on screen, so edges smear instead of showing the background
            let (lo, hi) = (to_view.map_xy(0.0, 0.0), to_view.map_xy(img.width() as f32, img.height() as f32));
            let u = Uniforms::new().v2(p.shift[0], p.shift[1]).v2(f64::from(lo.x) + 0.5, f64::from(lo.y) + 0.5).v2(f64::from(hi.x) - 0.5, f64::from(hi.y) - 0.5);
            paint.set_shader(ctx.programs.parallax.instance(u, &[s_img, s_depth])?);
            c.draw_rect(dest, &paint);
        }
    }
    c.restore();
    Ok(())
}
