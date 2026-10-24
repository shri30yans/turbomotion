//! Photo overlays: scrim, tag, badge, hero word, title and note.

use anyhow::Result;
use skia_safe::{
    gradient_shader, Canvas, Color, Paint, PaintStyle, Point, RRect, Rect, TileMode,
};

use crate::{
    film::{Film, Shot},
    layer::{with_backdrop_blur, with_opacity},
    motion,
    palette::{black, OCHRE, TERRACOTTA, WHITE},
    text::{Family, Fonts, Shadow, Style, TextBlock, UNBOUNDED},
};

const NOTE_BAR: f32 = 5.0;
const NOTE_GAP: f32 = 18.0;
const NOTE_MAX_WIDTH: f32 = 1100.0;

struct Tag {
    text: TextBlock,
}

struct Badge {
    text: TextBlock,
    fill: Color,
}

struct Heading {
    text: TextBlock,
}

struct Title {
    text: TextBlock,
}

struct Note {
    text: TextBlock,
    bar: f32,
    /// Whether a title precedes it, which delays the note behind the title.
    after_title: bool,
}

pub struct Overlay {
    tag: Option<Tag>,
    badge: Option<Badge>,
    hero: Option<Heading>,
    title: Option<Title>,
    note: Option<Note>,
    /// Seconds into the shot at which the first cue lands (`cueAt(shot, 0, 0.6)`).
    title_at: f32,
    size: (f32, f32),
}

fn badge_fill(label: &str) -> Color {
    if label.contains("LEGEND") || label.contains("STORY") {
        TERRACOTTA
    } else if label.contains("RECONSTRUCTION") || label.contains("ILLUSTRAT") {
        OCHRE
    } else {
        Color::from_argb(157, 20, 14, 10)
    }
}

impl Overlay {
    /// `None` when the shot carries no overlay parameters.
    pub fn prepare(fonts: &Fonts, film: &Film, shot: &Shot) -> Result<Option<Self>> {
        let get = |k| shot.param(k);
        let mono = |size, spacing, color| Style { letter_spacing: spacing, ..Style::new(Family::Plex, size, 500.0, color) };
        let tag = get("tag")
            .map(|t| {
                let style = Style { shadow: Some(Shadow { color: black(140), dx: 0.0, dy: 2.0, blur: 12.0 }), ..mono(21.0, 2.2, WHITE) };
                fonts.typeset(t, &style, UNBOUNDED, false).map(|text| Tag { text })
            })
            .transpose()?;
        let badge = get("badge")
            .map(|b| fonts.typeset(b, &mono(19.0, 1.6, WHITE), UNBOUNDED, false).map(|text| Badge { text, fill: badge_fill(b) }))
            .transpose()?;
        let hero = get("hero")
            .map(|h| {
                let style = Style {
                    letter_spacing: -6.0,
                    line_height: Some(0.95),
                    shadow: Some(Shadow { color: black(115), dx: 0.0, dy: 10.0, blur: 50.0 }),
                    opsz: Some(144.0),
                    soft: Some(50.0),
                    ..Style::new(Family::Fraunces, 220.0, 600.0, WHITE)
                };
                fonts.typeset(h, &style, film.width as f32 - 110.0, true).map(|text| Heading { text })
            })
            .transpose()?;
        let title = get("title")
            .map(|t| {
                let style = Style {
                    letter_spacing: -2.0,
                    line_height: Some(1.04),
                    shadow: Some(Shadow { color: black(102), dx: 0.0, dy: 6.0, blur: 30.0 }),
                    opsz: Some(120.0),
                    ..Style::new(Family::Fraunces, 84.0, 500.0, WHITE)
                };
                fonts.typeset(t, &style, 1250.0, false).map(|text| Title { text })
            })
            .transpose()?;
        let note = get("note")
            .map(|n| {
                let style = Style {
                    letter_spacing: -0.3,
                    line_height: Some(1.35),
                    ..Style::new(Family::Manrope, 32.0, 500.0, Color::from_argb(235, 255, 248, 236))
                };
                // The bar and the text are flex items in a 1100px box; when the text's
                // max-content width overflows it, both shrink in proportion to their base size.
                let base = fonts.measure(n, &style);
                let overflow = (NOTE_BAR + base - (NOTE_MAX_WIDTH - NOTE_GAP)).max(0.0);
                let bar = NOTE_BAR - overflow * NOTE_BAR / (NOTE_BAR + base);
                // Rounded up like every shrink-to-fit box: a line laid out in exactly its own measured
                // width can wrap on rounding.
                let width = (base - overflow * base / (NOTE_BAR + base)).ceil();
                fonts.typeset(n, &style, width, false).map(|text| Note { text, bar, after_title: title.is_some() })
            })
            .transpose()?;
        if tag.is_none() && badge.is_none() && hero.is_none() && title.is_none() && note.is_none() {
            return Ok(None);
        }
        Ok(Some(Self { tag, badge, hero, title, note, title_at: shot.cue_at(0, Some(0.6)), size: (film.width as f32, film.height as f32) }))
    }

    pub fn draw(&self, canvas: &Canvas, seconds: f32) {
        let (w, h) = self.size;
        if self.hero.is_some() || self.title.is_some() || self.note.is_some() {
            self.scrim(canvas, motion::show(seconds, 0.0, 0.8));
        }
        if let Some(tag) = &self.tag {
            let p = motion::fade_up(seconds, 0.25, 12.0, 0.6);
            let row = tag.text.height.max(12.0);
            let top = 64.0 + p.dy;
            with_opacity(canvas, p.opacity, || {
                let mut square = Paint::default();
                square.set_color(TERRACOTTA);
                canvas.draw_rect(Rect::from_xywh(72.0, top + (row - 12.0) / 2.0, 12.0, 12.0), &square);
                tag.text.draw(canvas, Point::new(72.0 + 12.0 + 14.0, top + (row - tag.text.height) / 2.0));
            });
        }
        if let Some(badge) = &self.badge {
            let p = motion::fade_up(seconds, 0.4, -10.0, 0.6);
            let (bw, bh) = (badge.text.content_width() + 32.0 + 2.0, badge.text.height + 18.0 + 2.0);
            let rect = Rect::from_xywh(w - 72.0 - bw, 64.0 + p.dy, bw, bh);
            let rrect = RRect::new_rect_xy(rect, 4.0, 4.0);
            with_backdrop_blur(canvas, rrect, 6.0, p.opacity, || {
                let mut fill = Paint::default();
                fill.set_color(badge.fill);
                canvas.draw_rrect(rrect, &fill);
                let mut border = Paint::default();
                border.set_style(PaintStyle::Stroke).set_stroke_width(1.0).set_anti_alias(true).set_color(Color::from_argb(89, 255, 248, 236));
                canvas.draw_rrect(RRect::new_rect_xy(rect.with_inset((0.5, 0.5)), 3.5, 3.5), &border);
                badge.text.draw(canvas, Point::new(rect.left + 1.0 + 16.0, rect.top + 1.0 + 9.0));
            });
        }
        if let Some(hero) = &self.hero {
            let at = Point::new(110.0, h - 150.0 - hero.text.height);
            let start = self.title_at - 0.15;
            hero.text.draw_chars(canvas, at, |n| motion::mask_slide(seconds, start, 0.04, n, 0.6, 105.0));
        }
        if self.title.is_some() || self.note.is_some() {
            self.caption(canvas, seconds);
        }
    }

    /// Title and note stack from the container's bottom edge (110px in, 104px up).
    fn caption(&self, canvas: &Canvas, seconds: f32) {
        let h = self.size.1;
        let title_h = self.title.as_ref().map_or(0.0, |t| t.text.height);
        let note_h = self.note.as_ref().map_or(0.0, |n| 22.0 + n.text.height);
        let top = h - 104.0 - title_h - note_h;
        if let Some(title) = &self.title {
            let start = self.title_at - 0.1;
            title.text.draw_lines(canvas, Point::new(110.0, top), |i| motion::mask_slide(seconds, start, 0.08, i, 0.8, 110.0));
        }
        if let Some(note) = &self.note {
            let at = if note.after_title { self.title_at + 0.45 } else { self.title_at.max(0.6) };
            let p = motion::fade_up(seconds, at, 14.0, 0.6);
            let y = top + title_h + 22.0 + p.dy;
            with_opacity(canvas, p.opacity, || {
                let mut bar = Paint::default();
                bar.set_color(OCHRE).set_anti_alias(true);
                canvas.draw_rrect(RRect::new_rect_xy(Rect::from_xywh(110.0, y, note.bar, note.text.height), 2.0, 2.0), &bar);
                note.text.draw(canvas, Point::new(110.0 + note.bar + NOTE_GAP, y));
            });
        }
    }

    fn scrim(&self, canvas: &Canvas, opacity: f32) {
        let (w, h) = self.size;
        let stop = |a: f32| Color::from_argb((a * opacity * 255.0).round() as u8, 14, 9, 6);
        let shader = gradient_shader::linear(
            (Point::new(0.0, h), Point::new(0.0, 0.0)),
            [stop(0.78), stop(0.35), stop(0.0)].as_slice(),
            [0.0, 0.32, 0.55].as_slice(),
            TileMode::Clamp,
            None,
            None,
        );
        let mut paint = Paint::default();
        paint.set_shader(shader);
        canvas.draw_rect(Rect::from_wh(w, h), &paint);
    }
}
