//! Kinetic typography card: a kicker, lines of display type (the later lines italic in the
//! accent colour) and a rule, optionally beside a photograph.
//! Mirrors `CardShot` in `bench/remotion/src/shots/Typography.tsx`.

use anyhow::{Context, Result};
use skia_safe::{gpu::Mipmapped, Canvas, Image, Paint, PaintCap, PaintStyle, Point, RRect, Rect};

use super::{Loader, Moment, Scene, Stage};
use crate::{
    css_filter::Matrix,
    ease::Ease,
    film::Shot,
    image::{draw_cover, load_texture},
    motion::{self, Tween},
    paper::{Backdrop, Paper},
    palette::{INK, OCHRE, TERRACOTTA, WHITE},
    text::{Family, Style, TextBlock},
};

pub const PARAMS: &[&str] = &["text", "kicker", "tone", "image", "focus", "zoom"];

const KICKER_MARGIN: f32 = 40.0;
const RULE_MARGIN: f32 = 56.0;
const RULE: (f32, f32) = (260.0, 10.0);
const MAX_TEXT_WIDTH: f32 = 1580.0;
/// The photograph's box, to the right of the text.
const PHOTO_BOX: Rect = Rect { left: 1010.0, top: 110.0, right: 1010.0 + 838.0, bottom: 110.0 + 860.0 };

struct Illustration {
    image: Image,
    /// Normalised crop and zoom origin.
    focus: (f32, f32),
    zoom: f32,
}

struct Card {
    paper: Backdrop,
    illustration: Option<Illustration>,
    kicker: Option<TextBlock>,
    lines: Vec<TextBlock>,
    /// Seconds at which each line's reveal begins.
    line_starts: Vec<f32>,
    left: f32,
    top: f32,
    duration: f32,
}

pub fn prepare(loader: &mut Loader, shot: &Shot) -> Result<Box<dyn Scene>> {
    let (film, fonts) = (loader.film, loader.fonts);
    let source = shot.param("text").unwrap_or_default();
    let texts: Vec<&str> = source.split('|').collect();
    let dark = shot.param("tone") == Some("dark");
    let focus = shot.param("focus").map_or(Ok((0.5, 0.5)), parse_pair)?;
    let illustration = shot
        .param("image")
        .map(|id| {
            let asset = film.assets.get(id).with_context(|| format!("{}: unknown image asset {id:?}", shot.id))?;
            Ok::<_, anyhow::Error>(Illustration {
                image: load_texture(loader.context, &loader.public.join(&asset.file), Mipmapped::Yes)?,
                focus,
                zoom: shot.param("zoom").map(str::parse).transpose().context("zoom is not a number")?.unwrap_or(1.06),
            })
        })
        .transpose()?;

    let beside_photo = illustration.is_some();
    let (left, right) = if beside_photo { (150.0, 980.0) } else { (170.0, 170.0) };
    let width = (film.width as f32 - left - right).min(MAX_TEXT_WIDTH);
    let (size, spacing) = if beside_photo { (92.0, -2.5) } else if texts.len() > 2 { (104.0, -3.5) } else { (128.0, -3.5) };
    let lines = texts
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let style = Style {
                letter_spacing: spacing,
                line_height: Some(1.04),
                opsz: Some(144.0),
                soft: Some(40.0),
                italic: i > 0,
                ..Style::new(Family::Fraunces, size, if i == 0 { 500.0 } else { 400.0 }, if i > 0 { TERRACOTTA } else if dark { WHITE } else { INK })
            };
            fonts.typeset(text, &style, width, false)
        })
        .collect::<Result<Vec<_>>>()?;
    let kicker = shot
        .param("kicker")
        .map(|k| fonts.typeset(k, &Style::mono(26.0, 4.0, TERRACOTTA), width, true))
        .transpose()?;
    // Line 0 lands at once; line i lands on cue i-1, the beat that earns it.
    let line_starts = (0..lines.len())
        .map(|i| {
            let at = match (i, shot.cue_times.get(i.wrapping_sub(1))) {
                (0, _) => 0.25,
                (_, Some(cue)) => cue - 0.1,
                (_, None) => 0.25 + 0.85 * i as f32,
            };
            at.max(0.1)
        })
        .collect();

    let content = kicker.as_ref().map_or(0.0, |k| k.height + KICKER_MARGIN) + lines.iter().map(|l| l.height).sum::<f32>() + RULE_MARGIN + RULE.1;
    Ok(Box::new(Card {
        paper: Backdrop::prepare(loader.context, loader.assets, Paper::Sheet { dark }, (film.width as f32, film.height as f32))?,
        illustration,
        kicker,
        lines,
        line_starts,
        left,
        top: (film.height as f32 - content) / 2.0,
        duration: shot.duration,
    }))
}

fn parse_pair(spec: &str) -> Result<(f32, f32)> {
    let (x, y) = spec.split_once(',').with_context(|| format!("{spec:?} is not x,y"))?;
    Ok((x.trim().parse()?, y.trim().parse()?))
}

impl Card {
    fn draw_photo(&self, canvas: &Canvas, art: &Illustration, at: Moment) {
        // The photograph is wiped in from the bottom (`clip-path: inset`), then pushed in slowly.
        let wipe = Tween::new(0.05, 1.0, Ease::PowerInOut(3)).progress(at.seconds);
        if wipe <= 0.0 {
            return;
        }
        let zoom = art.zoom + 0.05 * (at.seconds / self.duration).min(1.0);
        let origin = Point::new(PHOTO_BOX.left + art.focus.0 * PHOTO_BOX.width(), PHOTO_BOX.top + art.focus.1 * PHOTO_BOX.height());
        let revealed = Rect::new(PHOTO_BOX.left, PHOTO_BOX.top + (1.0 - wipe) * PHOTO_BOX.height(), PHOTO_BOX.right, PHOTO_BOX.bottom);

        let mut paint = Paint::default();
        paint.set_color_filter(Matrix::contrast(1.04).then(Matrix::saturate(1.05)).then(Matrix::sepia(0.08)).filter());
        canvas.save();
        canvas.clip_rrect(RRect::new_rect_xy(PHOTO_BOX, 4.0, 4.0), None, true);
        canvas.clip_rect(revealed, None, false);
        canvas.translate(origin);
        canvas.scale((zoom, zoom));
        canvas.translate((-origin.x, -origin.y));
        draw_cover(canvas, &art.image, PHOTO_BOX, art.focus, &paint);
        canvas.restore();
    }
}

impl Scene for Card {
    fn draw(&self, canvas: &Canvas, _: &Stage, at: Moment) -> Result<()> {
        self.paper.draw(canvas, at.frame)?;
        if let Some(art) = &self.illustration {
            self.draw_photo(canvas, art, at);
        }

        let mut y = self.top;
        if let Some(kicker) = &self.kicker {
            kicker.draw_chars(canvas, Point::new(self.left, y), |n| motion::mask_slide(at.seconds, 0.15, 0.018, n, 0.6, 105.0));
            y += kicker.height + KICKER_MARGIN;
        }
        for (line, start) in self.lines.iter().zip(&self.line_starts) {
            line.draw_lines(canvas, Point::new(self.left, y), |i| motion::mask_slide(at.seconds, *start, 0.07, i, 0.8, 110.0));
            y += line.height;
        }

        let drawn = motion::draw(at.seconds, 0.2, 1.0, Ease::PowerInOut(2));
        if drawn > 0.0 {
            let mut rule = Paint::default();
            rule.set_style(PaintStyle::Stroke).set_stroke_width(6.0).set_stroke_cap(PaintCap::Round).set_anti_alias(true).set_color(OCHRE);
            let rule_y = y + RULE_MARGIN + RULE.1 / 2.0;
            canvas.draw_line((self.left + 2.0, rule_y), (self.left + 2.0 + 256.0 * drawn, rule_y), &rule);
        }
        Ok(())
    }
}

