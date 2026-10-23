//! Historical artwork shown whole on dark paper, then explored by a slow camera move.
//! Mirrors `PaintingShot` in `bench/remotion/src/shots/Typography.tsx`.

use anyhow::{Context, Result};
use skia_safe::{gpu::Mipmapped, BlurStyle, Canvas, Color, Image, MaskFilter, Paint, PaintStyle, Point, RRect, Rect};

use super::{Loader, Moment, Scene, Stage};
use crate::{
    camera::{ease_cam, parse_view, View},
    css_filter::Matrix,
    ease::SETTLE,
    film::Shot,
    image::{draw_fit, load_texture},
    layer::with_opacity,
    motion::{self, Tween},
    paper::{Backdrop, Paper},
    palette::{black, white, WHITE},
    text::{Align, Family, Shadow, Style, TextBlock, UNBOUNDED},
};

/// `tag` is accepted but never shown: the reference fades `.tag` in nowhere in this shot type.
pub const PARAMS: &[&str] = &["from", "to", "badge", "note", "tag"];

const BOX_HEIGHT: f32 = 880.0;
const BOX_MAX_WIDTH: f32 = 1600.0;
const NOTE_MAX_WIDTH: f32 = 1100.0;
const OUTLINE_OFFSET: f32 = 14.0;

struct Badge {
    text: TextBlock,
}

struct Note {
    text: TextBlock,
    width: f32,
}

struct Painting {
    paper: Backdrop,
    art: Image,
    from: View,
    to: View,
    duration: f32,
    badge: Option<Badge>,
    note: Option<Note>,
}

pub fn prepare(loader: &mut Loader, shot: &Shot) -> Result<Box<dyn Scene>> {
    let id = shot.args.first().with_context(|| format!("{}: painting shot without an asset", shot.id))?;
    let asset = loader.film.assets.get(id).with_context(|| format!("{}: unknown asset {id:?}", shot.id))?;
    let view = |key: &str, default: View| shot.param(key).map(parse_view).unwrap_or(Ok(default));
    let fonts = loader.fonts;
    let badge = shot
        .param("badge")
        .map(|b| fonts.typeset(b, &Style::mono(19.0, 1.6, WHITE), UNBOUNDED, false).map(|text| Badge { text }))
        .transpose()?;
    let note = shot
        .param("note")
        .map(|n| {
            let style = Style {
                shadow: Some(Shadow { color: black(204), dx: 0.0, dy: 2.0, blur: 14.0 }),
                align: Align::Right,
                ..Style::new(Family::Manrope, 27.0, 500.0, white(230))
            };
            // An absolutely positioned box shrinks to its text, up to the maximum width.
            let width = fonts.shrink_to_fit(n, &style, NOTE_MAX_WIDTH);
            fonts.typeset(n, &style, width, false).map(|text| Note { text, width })
        })
        .transpose()?;
    Ok(Box::new(Painting {
        paper: Backdrop::prepare(loader.context, loader.assets, Paper::Sheet { dark: true }, (loader.film.width as f32, loader.film.height as f32))?,
        art: load_texture(loader.context, &loader.public.join(&asset.file), Mipmapped::Yes)?,
        from: view("from", View { cx: 0.5, cy: 0.5, zoom: 1.0 })?,
        to: view("to", View { cx: 0.5, cy: 0.5, zoom: 1.15 })?,
        duration: shot.duration,
        badge,
        note,
    }))
}

impl Scene for Painting {
    fn draw(&self, canvas: &Canvas, stage: &Stage, at: Moment) -> Result<()> {
        let (w, h) = (stage.film.width as f32, stage.film.height as f32);
        self.paper.draw(canvas, at.frame)?;

        // The artwork is centred at a fixed height and the camera is a zoom about a point on it.
        let aspect = self.art.width() as f32 / self.art.height() as f32;
        let (box_w, box_h) = ((BOX_HEIGHT * aspect).min(BOX_MAX_WIDTH), (BOX_HEIGHT * aspect).min(BOX_MAX_WIDTH) / aspect);
        let frame = Rect::from_xywh((w - box_w) / 2.0, (h - box_h) / 2.0, box_w, box_h);
        let k = ease_cam((f64::from(at.seconds) / f64::from(self.duration)).min(1.0));
        let lerp = |a: f64, b: f64| (a + (b - a) * k) as f32;
        let (cx, cy, zoom) = (lerp(self.from.cx, self.to.cx), lerp(self.from.cy, self.to.cy), lerp(self.from.zoom, self.to.zoom));
        let pivot = Point::new(frame.left + cx * box_w, frame.top + cy * box_h);

        canvas.save();
        canvas.translate(pivot);
        canvas.scale((zoom, zoom));
        canvas.translate((-pivot.x, -pivot.y));
        // The frame fades, rises and settles in around its own centre.
        let enter = Tween::new(0.05, 0.9, SETTLE).progress(at.seconds);
        let scale = 0.97 + 0.03 * enter;
        let centre = frame.center();
        canvas.translate((0.0, 30.0 * (1.0 - enter)));
        canvas.translate(centre);
        canvas.scale((scale, scale));
        canvas.translate((-centre.x, -centre.y));
        with_opacity(canvas, enter, || self.draw_frame(canvas, frame));
        canvas.restore();

        if let Some(badge) = &self.badge {
            let p = motion::fade_up(at.seconds, 0.5, -10.0, 0.6);
            let (bw, bh) = (badge.text.content_width() + 32.0 + 2.0, badge.text.height + 18.0 + 2.0);
            let rect = Rect::from_xywh(w - 72.0 - bw, 64.0 + p.dy, bw, bh);
            with_opacity(canvas, p.opacity, || {
                let rrect = RRect::new_rect_xy(rect, 4.0, 4.0);
                let mut fill = Paint::default();
                fill.set_color(white(31));
                canvas.draw_rrect(rrect, &fill);
                let mut border = Paint::default();
                border.set_style(PaintStyle::Stroke).set_stroke_width(1.0).set_anti_alias(true).set_color(white(77));
                canvas.draw_rrect(RRect::new_rect_xy(rect.with_inset((0.5, 0.5)), 3.5, 3.5), &border);
                badge.text.draw(canvas, Point::new(rect.left + 17.0, rect.top + 10.0));
            });
        }
        if let Some(note) = &self.note {
            let p = motion::fade_up(at.seconds, 0.8, 12.0, 0.6);
            with_opacity(canvas, p.opacity, || {
                note.text.draw(canvas, Point::new(w - 72.0 - note.width, h - 40.0 - note.text.height + p.dy));
            });
        }
        Ok(())
    }
}

impl Painting {
    /// Shadow, hairline outline and the picture itself.
    fn draw_frame(&self, canvas: &Canvas, frame: Rect) {
        let mut shadow = Paint::default();
        shadow.set_color(black(153)).set_mask_filter(MaskFilter::blur(BlurStyle::Normal, 45.0, false));
        canvas.draw_rect(frame.with_offset((0.0, 30.0)), &shadow);

        let mut outline = Paint::default();
        outline.set_style(PaintStyle::Stroke).set_stroke_width(1.0).set_color(Color::from_argb(89, 224, 169, 59));
        canvas.draw_rect(frame.with_outset((OUTLINE_OFFSET + 0.5, OUTLINE_OFFSET + 0.5)), &outline);

        let mut picture = Paint::default();
        picture.set_color_filter(Matrix::contrast(1.03).then(Matrix::saturate(1.04)).filter());
        draw_fit(canvas, &self.art, frame, &picture);
    }
}
