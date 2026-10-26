//! Contact sheets: several frames on one labelled picture, optionally beside a reference video.

use anyhow::{Context, Result};
use skia_safe::{png_encoder, AlphaType, ColorType, ISize, ImageInfo, Paint, Pixmap};

use super::Renderer;
use crate::media::refframe::RefVideo;

impl Renderer<'_> {
    /// A labelled contact sheet of the frames at `times`.
    pub fn sheet(&mut self, times: &[f64], cols: u32, thumb: u32) -> Result<Vec<u8>> {
        self.sheet_against(times, cols, thumb, None)
    }

    /// A contact sheet; with a reference video, each cell holds the reference frame (left) beside ours (right) at the same time.
    pub fn sheet_against(&mut self, times: &[f64], cols: u32, thumb: u32, reference: Option<&RefVideo>) -> Result<Vec<u8>> {
        let root = self.plan.root();
        let tw = thumb as i32;
        let th = (f64::from(thumb) * f64::from(root.height) / f64::from(root.width)).round() as i32;
        let slots = if reference.is_some() { 2 } else { 1 };
        let cols = cols.min(times.len() as u32).max(1) as i32;
        let rows = (times.len() as i32 + cols - 1) / cols;
        let (gap, label, inner) = (10, 22, 4);
        let cell_w = slots * tw + (slots - 1) * inner;
        let (sw, sh) = (cols * cell_w + (cols + 1) * gap, rows * (th + label) + (rows + 1) * gap);
        let mut sheet = self.gpu.frame_surface(ISize::new(sw, sh), self.pipeline.output_space())?;
        sheet.canvas().clear(skia_safe::Color::from_rgb(24, 24, 28));
        let font = skia_safe::Font::from_typeface(skia_safe::FontMgr::new().legacy_make_typeface(None, skia_safe::FontStyle::normal()).context("no default font")?, 14.0);
        let mut ink = Paint::default();
        ink.set_color(skia_safe::Color::from_rgb(220, 220, 225)).set_anti_alias(true);
        let mut p = Paint::default();
        p.set_anti_alias(true);
        let sampling = skia_safe::SamplingOptions::new(skia_safe::FilterMode::Linear, skia_safe::MipmapMode::Linear);
        for (i, t) in times.iter().enumerate() {
            self.draw_at(*t)?;
            let (c, r) = (i as i32 % cols, i as i32 / cols);
            let (x, y) = (gap + c * (cell_w + gap), gap + r * (th + label + gap));
            let image = self.surface.image_snapshot();
            let frame = (t * f64::from(self.plan.fps)).round() as u32;
            let mut ours_x = x;
            if let Some(reference) = reference {
                let png = reference.frame_png(*t).with_context(|| format!("reference frame for {t:.2}s"))?;
                let theirs = skia_safe::Image::from_encoded(skia_safe::Data::new_copy(&png)).context("decoding the reference frame")?;
                // fit the reference inside the cell without stretching it, centred on a dark plate
                let k = (tw as f32 / theirs.width() as f32).min(th as f32 / theirs.height() as f32);
                let (dw, dh) = (theirs.width() as f32 * k, theirs.height() as f32 * k);
                let (ox, oy) = (x as f32 + (tw as f32 - dw) / 2.0, (y + label) as f32 + (th as f32 - dh) / 2.0);
                sheet.canvas().draw_image_rect_with_sampling_options(&theirs, None, skia_safe::Rect::from_xywh(ox, oy, dw, dh), sampling, &p);
                sheet.canvas().draw_str(format!("REF {t:.2}s"), (x as f32, (y + 15) as f32), &font, &ink);
                ours_x = x + tw + inner;
            }
            sheet.canvas().draw_image_rect_with_sampling_options(&image, None, skia_safe::Rect::from_xywh(ours_x as f32, (y + label) as f32, tw as f32, th as f32), sampling, &p);
            sheet.canvas().draw_str(format!("{}{t:.2}s  (frame {frame})", if reference.is_some() { "OURS " } else { "" }), (ours_x as f32, (y + 15) as f32), &font, &ink);
        }
        self.gpu.context.flush_and_submit();
        let info = ImageInfo::new(ISize::new(sw, sh), ColorType::BGRA8888, AlphaType::Premul, self.pipeline.output_space());
        let mut pixels = vec![0u8; info.compute_min_byte_size()];
        anyhow::ensure!(sheet.read_pixels(&info, &mut pixels, info.min_row_bytes(), (0, 0)), "pixel readback failed");
        let pixmap = Pixmap::new(&info, &mut pixels, info.min_row_bytes()).context("wrapping the sheet pixels")?;
        let mut png = Vec::new();
        anyhow::ensure!(png_encoder::encode(&pixmap, &mut png, &png_encoder::Options::default()), "PNG encoding failed");
        Ok(png)
    }
}
