//! Spotlight: dim everything but one region of the picture, ring it, and name it. For "look here".
//!
//! `spot=x,y,rx,ry,Label` in image-normalised coordinates (`rx` is a share of the picture's width,
//! `ry` of its height). It lands on the shot's first cue (`spotCue` picks another).

use anyhow::{bail, Context, Result};
use skia_safe::{BlurStyle, Canvas, MaskFilter, Paint, PaintCap, PaintStyle, Path, PathFillType, PathMeasure, Point, RRect, Rect};

use crate::{
    camera,
    ease::Ease,
    film::{Film, Shot},
    layer::with_opacity,
    motion,
    palette::{black, NIGHT, OCHRE, WHITE},
    text::{Family, Fonts, Style, TextBlock, UNBOUNDED},
};

const SCRIM: f32 = 0.58;

pub struct Spot {
    x: f32,
    y: f32,
    rx: f32,
    ry: f32,
    label: Option<TextBlock>,
    at: f32,
    size: (f32, f32),
}

impl Spot {
    /// `None` when the shot has no spotlight.
    pub fn prepare(fonts: &Fonts, film: &Film, shot: &Shot) -> Result<Option<Self>> {
        let Some(spec) = shot.param("spot") else { return Ok(None) };
        let mut parts = spec.splitn(5, ',');
        let mut number = |name: &str| -> Result<f32> {
            parts.next().with_context(|| format!("{}: spot has no {name}", shot.id))?.trim().parse().with_context(|| format!("{}: spot {name} is not a number", shot.id))
        };
        let (x, y, rx, ry) = (number("x")?, number("y")?, number("rx")?, number("ry")?);
        if rx <= 0.0 || ry <= 0.0 {
            bail!("{}: spot radii must be positive", shot.id);
        }
        let label = parts.next().map(str::trim).filter(|l| !l.is_empty());
        let style = Style { letter_spacing: -0.3, ..Style::new(Family::Manrope, 32.0, 700.0, NIGHT) };
        let cue = shot.param("spotCue").map(str::parse::<usize>).transpose().context("spotCue is not a number")?.unwrap_or(0);
        Ok(Some(Self {
            x,
            y,
            rx,
            ry,
            label: label.map(|l| fonts.typeset(l, &style, UNBOUNDED, false)).transpose()?,
            at: shot.cue_at(cue, Some(0.9)),
            size: (film.width as f32, film.height as f32),
        }))
    }

    pub fn draw(&self, canvas: &Canvas, view: camera::Rect, seconds: f32) {
        let centre = Point::new(view.x + self.x * view.w, view.y + self.y * view.h);
        let oval = Rect::from_xywh(centre.x - self.rx * view.w, centre.y - self.ry * view.h, 2.0 * self.rx * view.w, 2.0 * self.ry * view.h);
        let shown = motion::show(seconds, self.at, 0.5);
        if shown <= 0.0 {
            return;
        }
        let mut hole = Path::new();
        hole.add_rect(Rect::from_wh(self.size.0, self.size.1), None);
        hole.add_oval(oval, None);
        hole.set_fill_type(PathFillType::EvenOdd);
        let mut scrim = Paint::default();
        scrim.set_anti_alias(true).set_color(black((255.0 * SCRIM * shown) as u8));
        canvas.draw_path(&hole, &scrim);

        let mut ring = Path::new();
        ring.add_oval(oval, None);
        let mut measure = PathMeasure::new(&ring, false, None);
        let length = measure.length();
        let drawn = motion::draw(seconds, self.at + 0.1, 0.8, Ease::PowerInOut(2));
        if let Some(visible) = measure.segment(0.0, length * drawn, true) {
            for (width, color) in [(9.0, black(70)), (4.5, WHITE)] {
                let mut paint = Paint::default();
                paint.set_anti_alias(true).set_style(PaintStyle::Stroke).set_stroke_width(width).set_stroke_cap(PaintCap::Round).set_color(color);
                canvas.draw_path(&visible, &paint);
            }
        }
        if let Some(label) = &self.label {
            let p = motion::fade_up(seconds, self.at + 0.55, 10.0, 0.45);
            let (w, h) = (label.content_width() + 36.0, label.height + 18.0);
            let right = oval.right + 40.0 + w < self.size.0 - 40.0;
            let left = if right { oval.right + 40.0 } else { oval.left - 40.0 - w };
            let top = centre.y - h / 2.0 + p.dy;
            let pill = RRect::new_rect_xy(Rect::from_xywh(left, top, w, h), 8.0, 8.0);
            with_opacity(canvas, p.opacity, || {
                let mut shadow = Paint::default();
                shadow.set_anti_alias(true).set_color(black(89)).set_mask_filter(MaskFilter::blur(BlurStyle::Normal, 12.0, false));
                canvas.draw_rrect(pill.with_offset((0.0, 6.0)), &shadow);
                let mut fill = Paint::default();
                fill.set_anti_alias(true).set_color(WHITE);
                canvas.draw_rrect(pill, &fill);
                let mut dot = Paint::default();
                dot.set_anti_alias(true).set_color(OCHRE);
                canvas.draw_rect(Rect::from_xywh(left, top + 8.0, 5.0, h - 16.0), &dot);
                label.draw(canvas, Point::new(left + 20.0, top + 9.0));
            });
        }
    }
}
