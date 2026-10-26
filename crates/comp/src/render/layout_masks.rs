//! Where each foreground layer really is on screen, as a coarse coverage mask: what `check --layout` compares.

use anyhow::{Context, Result};
use skia_safe::{AlphaType, Canvas, ColorType, ISize, ImageInfo, Paint};

use super::{graph, Ctx};
use crate::eval::{Content, Frame, LayerState};

/// One foreground layer's footprint on screen.
pub struct LayoutMask {
    pub comp: String,
    pub layer: String,
    /// Layers of one group (an arrow and its head and label) never clash with each other.
    pub group: Option<String>,
    pub w: usize,
    pub h: usize,
    /// One byte per cell: 1 where the layer is drawn (alpha above a quarter).
    pub cells: Vec<u8>,
}

const MASK_SCALE: f32 = 0.25;

fn is_foreground(st: &LayerState, role: Option<&str>) -> bool {
    match role {
        Some("ignore" | "backdrop") => false,
        Some("foreground") => true,
        Some(r) if r.starts_with("group:") => true,
        _ => matches!(st.content, Content::Text(_) | Content::Mesh(_) | Content::Precomp { .. } | Content::Shape { .. } | Content::Pack { .. }),
    }
}

pub(super) fn collect_masks(ctx: &Ctx, like: &Canvas, frame: &Frame, out: &mut Vec<LayoutMask>) -> Result<()> {
    let comp = &ctx.plan.comps[&frame.comp];
    for (i, st) in frame.layers.iter().enumerate() {
        if let Content::Precomp { frame: Some(inner) } = &st.content {
            if st.visible && st.opacity > 0.0 {
                collect_masks(ctx, like, inner, out)?;
            }
        }
        if !comp.layout || !st.active || !crate::check::shows(st) || !is_foreground(st, comp.layers[i].def.common.layout.as_deref()) {
            continue;
        }
        let img = graph::layer_picture(ctx, like, frame, i).with_context(|| comp.layers[i].label.clone())?;
        let (w, h) = ((frame.width as f32 * MASK_SCALE) as i32, (frame.height as f32 * MASK_SCALE) as i32);
        let info = ImageInfo::new(ISize::new(w, h), ColorType::RGBA8888, AlphaType::Premul, None);
        let mut small = like.new_surface(&info, None).context("no GPU memory for a layout mask")?;
        small.canvas().clear(skia_safe::Color::TRANSPARENT);
        let mut paint = Paint::default();
        paint.set_alpha_f(st.opacity as f32);
        small.canvas().draw_image_rect_with_sampling_options(
            img.image(),
            None,
            skia_safe::Rect::from_xywh(0.0, 0.0, w as f32, h as f32),
            skia_safe::SamplingOptions::new(skia_safe::FilterMode::Linear, skia_safe::MipmapMode::Linear),
            &paint,
        );
        let mut px = vec![0u8; info.compute_min_byte_size()];
        anyhow::ensure!(small.read_pixels(&info, &mut px, info.min_row_bytes(), (0, 0)), "layout mask readback failed");
        let cells = px.chunks_exact(4).map(|p| u8::from(p[3] > 115)).collect();
        out.push(LayoutMask { comp: frame.comp.clone(), layer: comp.layers[i].label.clone(), group: comp.layers[i].def.common.layout.clone().filter(|r| r.starts_with("group:")), w: w as usize, h: h as usize, cells });
    }
    Ok(())
}
