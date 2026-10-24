//! Paper-puppet figure: a cut-out portrait of a person (`engine/tools/figures.py`) that pops up on a cue,
//! sways gently and carries a name plate. For "meet the king" moments over a place or an object.
//!
//! `figure=<asset-id>,Name,Role` reads `public/figures/<asset-id>.png`. `figureAt=left|right` (default left),
//! `figureCue=n` (default 0), `figureH=0.5` (share of the frame height). Pure function of time: the sway is a sine.

use std::{f32::consts::TAU, fs};

use anyhow::{Context, Result};
use skia_safe::{BlurStyle, Canvas, Data, FilterMode, Image, MaskFilter, MipmapMode, Paint, Point, RRect, Rect, SamplingOptions};

use crate::{
    ease::Ease,
    film::{Film, Shot},
    layer::with_opacity,
    motion,
    palette::{black, INK, INK_SOFT, OCHRE, WHITE},
    text::{Family, Fonts, Style, TextBlock, UNBOUNDED},
};

pub struct Figure {
    image: Image,
    name: TextBlock,
    role: Option<TextBlock>,
    right: bool,
    height: f32,
    at: f32,
    size: (f32, f32),
}

impl Figure {
    pub fn prepare(fonts: &Fonts, film: &Film, public: &std::path::Path, shot: &Shot) -> Result<Option<Self>> {
        let Some(spec) = shot.param("figure") else { return Ok(None) };
        let mut parts = spec.splitn(3, ',').map(str::trim);
        let id = parts.next().filter(|s| !s.is_empty()).with_context(|| format!("{}: figure needs an asset id", shot.id))?;
        let name = parts.next().filter(|s| !s.is_empty()).with_context(|| format!("{}: figure={id} needs a name", shot.id))?;
        let role = parts.next().filter(|s| !s.is_empty());
        let path = public.join("figures").join(format!("{id}.png"));
        let bytes = fs::read(&path).with_context(|| format!("{}: {} (run engine/tools/figures.py)", shot.id, path.display()))?;
        let image = Image::from_encoded(Data::new_copy(&bytes)).with_context(|| format!("decoding {}", path.display()))?;
        let name_style = Style { letter_spacing: -0.5, ..Style::new(Family::Fraunces, 40.0, 600.0, INK) };
        let role_style = Style::mono(19.0, 1.2, INK_SOFT);
        let cue = shot.param("figureCue").map(str::parse::<usize>).transpose().context("figureCue is not a number")?.unwrap_or(0);
        let height = shot.param("figureH").map(str::parse::<f32>).transpose().context("figureH is not a number")?.unwrap_or(0.5);
        Ok(Some(Self {
            image,
            name: fonts.typeset(name, &name_style, UNBOUNDED, false)?,
            role: role.map(|r| fonts.typeset(&r.to_uppercase(), &role_style, UNBOUNDED, false)).transpose()?,
            right: shot.param("figureAt") == Some("right"),
            height,
            at: shot.cue_at(cue, Some(0.6)),
            size: (film.width as f32, film.height as f32),
        }))
    }

    pub fn draw(&self, canvas: &Canvas, seconds: f32) {
        let enter = motion::Tween::new(self.at, 0.7, Ease::BackOut(1.6)).progress(seconds);
        if enter <= 0.0 {
            return;
        }
        let (w, h) = self.size;
        let fh = self.height * h;
        let fw = fh * self.image.width() as f32 / self.image.height() as f32;
        let margin = 70.0;
        let left = if self.right { w - margin - fw } else { margin };
        let rest_bottom = h - 24.0;
        // idle sway about the feet, and a small bob; both periodic in time since the pop
        let t = (seconds - self.at).max(0.0);
        let sway = (t * TAU / 3.6).sin() * 1.4;
        let bob = ((t * TAU / 2.0).sin() * 0.5 + 0.5) * -6.0;
        let rise = (1.0 - enter) * (fh * 0.55);
        let opacity = motion::show(seconds, self.at, 0.25);
        let dst = Rect::from_xywh(left, rest_bottom - fh + rise + bob, fw, fh);
        with_opacity(canvas, opacity, || {
            canvas.save();
            canvas.rotate(sway, Some(Point::new(left + fw / 2.0, rest_bottom + rise)));
            let mut paint = Paint::default();
            paint.set_anti_alias(true);
            canvas.draw_image_rect_with_sampling_options(&self.image, None, dst, SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear), &paint);
            canvas.restore();
        });
        // name plate, sliding in a beat after the figure lands
        let p = motion::fade_up(seconds, self.at + 0.45, 14.0, 0.5);
        let role_w = self.role.as_ref().map_or(0.0, TextBlock::content_width);
        let (pw, ph) = (self.name.content_width().max(role_w) + 52.0, self.name.height + self.role.as_ref().map_or(0.0, |r| r.height + 4.0) + 26.0);
        let px = if self.right { left + fw - pw + 10.0 } else { left - 10.0 };
        let py = rest_bottom - ph - 6.0 + p.dy;
        with_opacity(canvas, p.opacity, || {
            let pill = RRect::new_rect_xy(Rect::from_xywh(px, py, pw, ph), 10.0, 10.0);
            let mut shadow = Paint::default();
            shadow.set_anti_alias(true).set_color(black(100)).set_mask_filter(MaskFilter::blur(BlurStyle::Normal, 14.0, false));
            canvas.draw_rrect(pill.with_offset((0.0, 8.0)), &shadow);
            let mut fill = Paint::default();
            fill.set_anti_alias(true).set_color(WHITE);
            canvas.draw_rrect(pill, &fill);
            let mut bar = Paint::default();
            bar.set_anti_alias(true).set_color(OCHRE);
            canvas.draw_rect(Rect::from_xywh(px, py + 10.0, 6.0, ph - 20.0), &bar);
            self.name.draw(canvas, Point::new(px + 28.0, py + 12.0));
            if let Some(role) = &self.role {
                role.draw(canvas, Point::new(px + 28.0, py + 12.0 + self.name.height + 2.0));
            }
        });
    }
}
