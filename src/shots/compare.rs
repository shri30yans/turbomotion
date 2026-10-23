//! A comparison on one shared scale: bars start at the same baseline and share one
//! pixels-per-unit, so length is the argument. Bar `i` lands on cue `i`.
//! Mirrors `CompareShot` in `bench/remotion/src/shots/Compare.tsx`.

use anyhow::{Context, Result};
use skia_safe::{Canvas, Color, Paint, PaintStyle, Path, Point, RRect, Rect};

use super::{Loader, Moment, Scene, Stage};
use crate::{
    ease::Ease,
    film::Shot,
    layer::with_opacity,
    motion,
    paper::{Backdrop, Paper},
    palette::{INK, INK_SOFT, OCHRE, PAPER, TEAL, TERRACOTTA},
    text::{Family, Fonts, Halo, Style, TextBlock, UNBOUNDED},
};

pub const PARAMS: &[&str] = &["items", "title", "unit", "kicker", "ref", "note", "tag", "legend"];

const LEFT: f32 = 470.0;
const RIGHT: f32 = 1700.0;
const TOP: f32 = 330.0;
const MAX_ROW: f32 = 190.0;
const ROWS_HEIGHT: f32 = 560.0;
const TEXT_LEFT: f32 = 150.0;
const DEFAULT_LEGEND: &str = "SAME SCALE · BARS START AT ZERO";
const LEGEND_INK: Color = Color::from_rgb(0x8C, 0x7A, 0x60);

struct Row {
    label: TextBlock,
    value: TextBlock,
    length: f32,
    /// The last bar is the one the argument lands on.
    hot: bool,
    /// Seconds into the shot at which the bar starts to grow.
    at: f32,
}

struct Reference {
    x: f32,
    label: TextBlock,
    at: f32,
}

struct Compare {
    paper: Backdrop,
    rows: Vec<Row>,
    row_height: f32,
    reference: Option<Reference>,
    kicker: Option<TextBlock>,
    title: TextBlock,
    note: Option<TextBlock>,
    tag: Option<TextBlock>,
    legend: TextBlock,
}

fn number(text: &str) -> Result<f64> {
    text.trim().parse().with_context(|| format!("{text:?} is not a number"))
}

pub fn prepare(loader: &mut Loader, shot: &Shot) -> Result<Box<dyn Scene>> {
    let fonts = loader.fonts;
    let unit = shot.param("unit").unwrap_or_default();
    let items = super::required(shot, "items")?
        .split(';')
        .map(|item| {
            let mut parts = item.split(':');
            let label = parts.next().unwrap_or_default().trim();
            let value = number(parts.next().with_context(|| format!("{}: item {item:?} has no value", shot.id))?)?;
            Ok((label, value))
        })
        .collect::<Result<Vec<_>>>()?;
    let max = items.iter().map(|(_, v)| *v).fold(f64::MIN, f64::max);
    let per_unit = (RIGHT - LEFT) / max as f32;
    let row_height = MAX_ROW.min(ROWS_HEIGHT / items.len() as f32);

    let rows = items
        .iter()
        .enumerate()
        .map(|(i, (label, value))| {
            let hot = i == items.len() - 1;
            Ok(Row {
                label: fonts.typeset(label, &Style { letter_spacing: -0.5, ..Style::new(Family::Manrope, 38.0, 600.0, INK) }, UNBOUNDED, false)?,
                value: value_text(fonts, *value, unit, hot)?,
                length: *value as f32 * per_unit,
                hot,
                at: (shot.cue_at(i, Some(0.6 + 1.2 * i as f32)) - 0.1).max(0.3),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let reference = shot
        .param("ref")
        .map(|spec| {
            let (name, value) = spec.split_once(':').with_context(|| format!("{}: ref {spec:?} needs label:value", shot.id))?;
            let value = number(value)?;
            let style = Style { halo: Some(Halo { color: PAPER, width: 8.0 }), ..Style::mono(24.0, 2.0, TEAL) };
            Ok::<_, anyhow::Error>(Reference {
                x: LEFT + value as f32 * per_unit,
                label: fonts.typeset(&format!("{name} · {value} {unit}"), &style, UNBOUNDED, false)?,
                at: (shot.cue_at(items.len(), Some(0.6 + 1.2 * items.len() as f32)) - 0.1).max(0.3),
            })
        })
        .transpose()?;

    let title_style = Style { letter_spacing: -2.0, line_height: Some(1.04), opsz: Some(144.0), soft: Some(40.0), ..Style::new(Family::Fraunces, 84.0, 500.0, INK) };
    let title_text = shot.param("title").unwrap_or_default();
    let width = loader.film.width as f32 - TEXT_LEFT;
    let note = shot
        .param("note")
        .map(|n| {
            let style = Style::new(Family::Manrope, 28.0, 500.0, INK_SOFT);
            fonts.typeset(n, &style, fonts.shrink_to_fit(n, &style, 1200.0), false)
        })
        .transpose()?;
    let tag = shot.param("tag").map(|t| fonts.typeset(t, &Style::mono(21.0, 2.2, INK), UNBOUNDED, false)).transpose()?;
    let size = (loader.film.width as f32, loader.film.height as f32);
    Ok(Box::new(Compare {
        paper: Backdrop::prepare(loader.context, loader.assets, Paper::Board { dark: false }, size)?,
        rows,
        row_height,
        reference,
        kicker: shot.param("kicker").map(|k| fonts.typeset(k, &Style::mono(26.0, 4.0, TERRACOTTA), width, true)).transpose()?,
        title: fonts.typeset(title_text, &title_style, fonts.shrink_to_fit(title_text, &title_style, width), false)?,
        note,
        tag,
        legend: fonts.typeset(shot.param("legend").unwrap_or(DEFAULT_LEGEND), &Style::mono(17.0, 2.0, LEGEND_INK), UNBOUNDED, false)?,
    }))
}

/// The value in display type, with its unit beside it; both outlined in paper so they read over the bars.
fn value_text(fonts: &Fonts, value: f64, unit: &str, hot: bool) -> Result<TextBlock> {
    let halo = Some(Halo { color: PAPER, width: 8.0 });
    let figure = Style { halo, ..Style::new(Family::Fraunces, 56.0, 700.0, if hot { TERRACOTTA } else { INK }) };
    let suffix = Style { halo, ..Style::new(Family::Manrope, 30.0, 600.0, INK_SOFT) };
    fonts.typeset_inline(&[(&value.to_string(), figure), (&format!(" {unit}"), suffix)])
}

impl Scene for Compare {
    fn draw(&self, canvas: &Canvas, stage: &Stage, at: Moment) -> Result<()> {
        let (w, h) = (stage.film.width as f32, stage.film.height as f32);
        let s = at.seconds;
        self.paper.draw(canvas, at.frame)?;

        if let Some(tag) = &self.tag {
            let mut square = Paint::default();
            square.set_color(TERRACOTTA);
            let row = tag.height.max(12.0);
            canvas.draw_rect(Rect::from_xywh(72.0, 60.0 + (row - 12.0) / 2.0, 12.0, 12.0), &square);
            tag.draw(canvas, Point::new(72.0 + 12.0 + 14.0, 60.0 + (row - tag.height) / 2.0));
        }
        self.legend.draw(canvas, Point::new(w - 72.0 - self.legend.content_width(), h - 56.0 - self.legend.height));

        if let Some(kicker) = &self.kicker {
            kicker.draw_chars(canvas, Point::new(TEXT_LEFT, 150.0), |n| motion::mask_slide(s, 0.15, 0.018, n, 0.6, 105.0));
        }
        self.title.draw_lines(canvas, Point::new(TEXT_LEFT, 195.0), |i| motion::mask_slide(s, 0.25, 0.07, i, 0.8, 110.0));

        let mut axis = Paint::default();
        axis.set_style(PaintStyle::Stroke).set_stroke_width(4.0).set_color(INK);
        let bottom = TOP + self.rows.len() as f32 * self.row_height;
        canvas.draw_line((LEFT, TOP - 30.0), (LEFT, bottom), &axis);

        for (i, row) in self.rows.iter().enumerate() {
            self.draw_row(canvas, row, TOP + i as f32 * self.row_height, s);
        }
        if let Some(reference) = &self.reference {
            self.draw_reference(canvas, reference, bottom, s);
        }
        if let Some(note) = &self.note {
            note.draw(canvas, Point::new(TEXT_LEFT, h - 70.0 - note.height));
        }
        Ok(())
    }
}

impl Compare {
    fn draw_row(&self, canvas: &Canvas, row: &Row, y: f32, s: f32) {
        let baseline = y + self.row_height * 0.42;

        let label = motion::fade_up(s, row.at - 0.1, 10.0, 0.4);
        with_opacity(canvas, label.opacity, || {
            row.label.draw_on_baseline(canvas, LEFT - 28.0 - row.label.content_width(), baseline + label.dy);
        });

        // The bar grows from its left edge; its outline stretches with it.
        let grow = motion::Tween::new(row.at, 1.0, Ease::PowerOut(3)).progress(s);
        if grow > 0.0 {
            canvas.save();
            canvas.translate((LEFT, 0.0));
            canvas.scale((grow, 1.0));
            canvas.translate((-LEFT, 0.0));
            let bar = RRect::new_rect_xy(Rect::from_xywh(LEFT, y + 14.0, row.length, self.row_height - 56.0), 4.0, 4.0);
            let mut fill = Paint::default();
            fill.set_anti_alias(true).set_color(if row.hot { TERRACOTTA } else { OCHRE });
            canvas.draw_rrect(bar, &fill);
            let mut outline = Paint::default();
            outline.set_style(PaintStyle::Stroke).set_stroke_width(3.0).set_anti_alias(true).set_color(INK);
            canvas.draw_rrect(bar, &outline);
            canvas.restore();
        }

        let value = motion::fade_up(s, row.at + 0.7, 8.0, 0.4);
        with_opacity(canvas, value.opacity, || {
            row.value.draw_on_baseline(canvas, LEFT + row.length + 22.0, baseline + value.dy);
        });
    }

    /// The guide line is declared dashed, but the reference reveals it with `drawSVG`, which
    /// replaces the dash pattern, so it is always solid.
    fn draw_reference(&self, canvas: &Canvas, reference: &Reference, bottom: f32, s: f32) {
        let drawn = motion::draw(s, reference.at, 0.8, Ease::PowerInOut(2));
        if drawn > 0.0 {
            let mut line = Paint::default();
            line.set_style(PaintStyle::Stroke).set_stroke_width(4.0).set_color(TEAL);
            let mut path = Path::new();
            path.move_to((reference.x, TOP - 30.0));
            path.line_to((reference.x, TOP - 30.0 + (bottom - TOP + 30.0) * drawn));
            canvas.draw_path(&path, &line);
        }
        reference.label.draw_on_baseline(canvas, reference.x + 14.0, TOP - 40.0);
    }
}
