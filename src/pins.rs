//! Photo pins: a pulsing dot on a point of the image, a leader line and a label, each
//! revealed on a script cue.

use anyhow::{bail, Context, Result};
use skia_safe::{BlurStyle, Canvas, MaskFilter, Paint, PaintCap, PaintStyle, Path, PathMeasure, Point, RRect, Rect};

use crate::{
    camera,
    ease::Ease,
    film::{Film, Shot},
    layer::with_opacity,
    motion::{self, Tween},
    palette::{black, NIGHT, OCHRE, WHITE},
    text::{Family, Fonts, Style, TextBlock, UNBOUNDED},
};

const RING_RADIUS: f32 = 14.0;
const DOT_RADIUS: f32 = 9.0;
const LEAD_RUN: f32 = 120.0;
const LEAD_ABOVE: f32 = 90.0;
const LEAD_BELOW: f32 = 100.0;
const LEAD_FOOT: f32 = 40.0;
const LABEL_GAP: f32 = 48.0;
const LABEL_PADDING: (f32, f32) = (16.0, 8.0);
const LABEL_RADIUS: f32 = 6.0;

struct Pin {
    /// Image-normalised position.
    x: f32,
    y: f32,
    label: TextBlock,
    /// Seconds into the shot at which the pin lands.
    at: f32,
}

pub struct Pins {
    pins: Vec<Pin>,
    size: (f32, f32),
}

/// `"x,y,Label;x,y,Label"`: pins in image-normalised coordinates.
fn parse(spec: &str) -> Result<Vec<(f32, f32, String)>> {
    spec.split(';')
        .map(|pin| {
            let mut parts = pin.splitn(3, ',');
            let mut coordinate = |name| {
                let value = parts.next().with_context(|| format!("pin {pin:?} has no {name}"))?;
                value.trim().parse::<f32>().with_context(|| format!("pin {pin:?}: {name} is not a number"))
            };
            let (x, y) = (coordinate("x")?, coordinate("y")?);
            let label = parts.next().map(str::trim).unwrap_or_default();
            Ok((x, y, label.to_owned()))
        })
        .collect()
}

impl Pins {
    /// `None` when the shot has no pins.
    pub fn prepare(fonts: &Fonts, film: &Film, shot: &Shot) -> Result<Option<Self>> {
        let Some(spec) = shot.param("pins") else { return Ok(None) };
        let specs = parse(spec)?;
        // Pins consume the shot's last cues unless `pinsFrom` names the first.
        let first = match shot.param("pinsFrom") {
            Some(from) => from.parse::<usize>().context("pinsFrom is not a number")?,
            None => shot.cues.saturating_sub(specs.len()),
        };
        let style = Style { letter_spacing: -0.3, ..Style::new(Family::Manrope, 30.0, 600.0, NIGHT) };
        let pins = specs
            .into_iter()
            .enumerate()
            .map(|(i, (x, y, label))| {
                if label.is_empty() {
                    bail!("{}: pin {i} has no label", shot.id);
                }
                Ok(Pin { x, y, label: fonts.typeset(&label, &style, UNBOUNDED, false)?, at: shot.cue_at(first + i, None) })
            })
            .collect::<Result<_>>()?;
        Ok(Some(Self { pins, size: (film.width as f32, film.height as f32) }))
    }

    /// Draw every pin over a photograph placed at `view`.
    pub fn draw(&self, canvas: &Canvas, view: camera::Rect, seconds: f32) {
        for (i, pin) in self.pins.iter().enumerate() {
            let anchor = Point::new(view.x + pin.x * view.w, view.y + pin.y * view.h);
            // Labels sit beside the pin on the side with room, alternating above and below
            // so neighbouring pins never collide.
            let right = anchor.x < self.size.0 * 0.62;
            let side = if right { 1.0 } else { -1.0 };
            let elbow = Point::new(anchor.x + side * LEAD_RUN, anchor.y + if i % 2 == 0 { -LEAD_ABOVE } else { LEAD_BELOW });
            self.ring(canvas, anchor, pin.at, seconds);
            lead(canvas, anchor, elbow, side, pin.at + 0.08, seconds);
            dot(canvas, anchor, pin.at, seconds);
            self.label(canvas, pin, elbow, side, seconds);
        }
    }

    fn ring(&self, canvas: &Canvas, centre: Point, at: f32, seconds: f32) {
        let p = Tween::new(at, 1.2, Ease::PowerOut(2)).progress(seconds);
        let scale = 0.4 + 2.2 * p;
        let mut paint = stroke(WHITE, 3.0 * scale);
        paint.set_alpha_f(0.9 * (1.0 - p));
        canvas.draw_circle(centre, RING_RADIUS * scale, &paint);
    }

    fn label(&self, canvas: &Canvas, pin: &Pin, elbow: Point, side: f32, seconds: f32) {
        let p = motion::fade_up(seconds, pin.at + 0.3, 10.0, 0.45);
        let (pad_x, pad_y) = LABEL_PADDING;
        let (w, h) = (pin.label.content_width() + 2.0 * pad_x, pin.label.height + 2.0 * pad_y);
        let left = if side > 0.0 { elbow.x + LABEL_GAP } else { elbow.x - LABEL_GAP - w };
        let top = elbow.y - 30.0 + p.dy;
        let pill = RRect::new_rect_xy(Rect::from_xywh(left, top, w, h), LABEL_RADIUS, LABEL_RADIUS);
        with_opacity(canvas, p.opacity, || {
            let mut shadow = Paint::default();
            shadow.set_anti_alias(true).set_color(black(89)).set_mask_filter(MaskFilter::blur(BlurStyle::Normal, 12.0, false));
            canvas.draw_rrect(pill.with_offset((0.0, 6.0)), &shadow);
            let mut fill = Paint::default();
            fill.set_anti_alias(true).set_color(WHITE);
            canvas.draw_rrect(pill, &fill);
            pin.label.draw(canvas, Point::new(left + pad_x, top + pad_y));
        });
    }
}

/// The leader line grows from the dot to the elbow, then out along the label's underline.
fn lead(canvas: &Canvas, from: Point, elbow: Point, side: f32, at: f32, seconds: f32) {
    let shown = motion::draw(seconds, at, 0.45, Ease::PowerInOut(2));
    if shown <= 0.0 {
        return;
    }
    let mut path = Path::new();
    path.move_to(from).line_to(elbow).line_to((elbow.x + side * LEAD_FOOT, elbow.y));
    let mut measure = PathMeasure::new(&path, false, None);
    let length = measure.length();
    let Some(visible) = measure.segment(0.0, length * shown, true) else { return };
    let mut paint = stroke(WHITE, 2.5);
    paint.set_stroke_cap(PaintCap::Butt);
    canvas.draw_path(&visible, &paint);
}

fn dot(canvas: &Canvas, centre: Point, at: f32, seconds: f32) {
    let p = Tween::new(at, 0.45, Ease::BackOut(2.2)).progress(seconds);
    // The overshoot of the back ease scales the dot past full size; opacity saturates.
    let scale = 0.6 + 0.4 * p;
    // The dot is a 2 * DOT_RADIUS box; see `pop_residual` for why it sits off its anchor.
    let (dx, dy) = motion::pop_residual(0.6, (2.0 * DOT_RADIUS, 2.0 * DOT_RADIUS));
    let centre = Point::new(centre.x + dx, centre.y + dy);
    let mut fill = Paint::default();
    fill.set_anti_alias(true).set_color(OCHRE).set_alpha_f(p.min(1.0));
    let mut rim = stroke(NIGHT, 3.0 * scale);
    rim.set_alpha_f(p.min(1.0));
    canvas.draw_circle(centre, DOT_RADIUS * scale, &fill);
    canvas.draw_circle(centre, DOT_RADIUS * scale, &rim);
}

fn stroke(color: skia_safe::Color, width: f32) -> Paint {
    let mut paint = Paint::default();
    paint.set_anti_alias(true).set_style(PaintStyle::Stroke).set_stroke_width(width).set_color(color);
    paint
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pins_with_commas_in_the_label() {
        let pins = parse("0.42,0.615,Stables, guards;0.84,0.63,Offices").unwrap();
        assert_eq!(pins, vec![(0.42, 0.615, "Stables, guards".to_owned()), (0.84, 0.63, "Offices".to_owned())]);
    }

    #[test]
    fn rejects_malformed_pins() {
        assert!(parse("0.4").is_err());
        assert!(parse("a,b,c").is_err());
    }
}
