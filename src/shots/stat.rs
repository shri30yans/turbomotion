//! A big number that counts up on its cue, over a dimmed photograph.

use anyhow::{Context, Result};
use skia_safe::{Canvas, Paint, Point, Rect};

use super::{required, Loader, Moment, Scene, Stage};
use crate::{
    ease::Ease,
    film::Shot,
    layer::with_opacity,
    motion,
    palette::{OCHRE, WHITE},
    photo::Photo,
    text::{Family, Style, TextBlock, UNBOUNDED},
};

pub const PARAMS: &[&str] = &["from", "to", "value", "prefix", "unit", "label"];

const LEFT: f32 = 150.0;
const GAP: f32 = 28.0;
/// The count starts just after the cut.
const COUNT_FROM: f32 = 0.12;
const RULE: (f32, f32) = (900.0, 14.0);
const RULE_MARGIN: f32 = 26.0;
const LABEL_MARGIN: f32 = 22.0;

struct Stat {
    photo: Photo,
    number_style: Style,
    value: f64,
    decimals: usize,
    prefix: Option<TextBlock>,
    unit: Option<TextBlock>,
    label: TextBlock,
    /// Seconds into the shot at which the count lands on its cue word.
    land: f32,
}

pub fn prepare(loader: &mut Loader, shot: &Shot) -> Result<Box<dyn Scene>> {
    let text = required(shot, "value")?;
    let value = text.parse().with_context(|| format!("{}: value {text:?} is not a number", shot.id))?;
    let decimals = text.split_once('.').map_or(0, |(_, fraction)| fraction.len());
    let fonts = loader.fonts;
    let prefix = shot
        .param("prefix")
        .map(|p| fonts.typeset(p, &Style::new(Family::Fraunces, 150.0, 300.0, OCHRE), UNBOUNDED, false))
        .transpose()?;
    let unit = shot
        .param("unit")
        .map(|u| fonts.typeset(u, &Style { letter_spacing: -2.0, ..Style::new(Family::Manrope, 84.0, 600.0, OCHRE) }, UNBOUNDED, false))
        .transpose()?;
    let label_style = Style::mono(28.0, 3.0, crate::palette::white(217));
    let label = fonts.typeset(&shot.param("label").unwrap_or_default().to_uppercase(), &label_style, UNBOUNDED, false)?;
    Ok(Box::new(Stat {
        photo: Photo::dimmed(loader.context, loader.film, loader.public, shot)?,
        number_style: Style {
            letter_spacing: -10.0,
            line_height: Some(0.9),
            tabular: true,
            opsz: Some(144.0),
            soft: Some(30.0),
            ..Style::new(Family::Fraunces, 300.0, 600.0, WHITE)
        },
        value,
        decimals,
        prefix,
        unit,
        label,
        land: (shot.cue_at(0, Some(0.9)) + 0.1).min(shot.duration - 0.35).max(0.7),
    }))
}

impl Stat {
    /// The figure as `toLocaleString('en-US')` would print it part-way through the count.
    fn shown(&self, seconds: f32) -> String {
        let k = ((seconds - COUNT_FROM) / (self.land - COUNT_FROM)).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - f64::from(k)).powi(3);
        group_thousands(&format!("{:.*}", self.decimals, self.value * eased))
    }
}

/// Insert thousands separators into a decimal string.
fn group_thousands(number: &str) -> String {
    let (whole, fraction) = number.split_once('.').map_or((number, None), |(w, f)| (w, Some(f)));
    let (sign, digits) = whole.strip_prefix('-').map_or(("", whole), |d| ("-", d));
    let mut out = String::from(sign);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if let Some(fraction) = fraction {
        out.push('.');
        out.push_str(fraction);
    }
    out
}

impl Scene for Stat {
    fn draw(&self, canvas: &Canvas, stage: &Stage, at: Moment) -> Result<()> {
        self.photo.draw(canvas, stage.photo, stage.film, at.seconds)?;
        let fonts = stage.fonts;
        let number = fonts.typeset(&self.shown(at.seconds), &self.number_style, UNBOUNDED, false)?;

        // The figure is an inline flex row aligned on the baseline.
        let parts: Vec<&TextBlock> = self.prefix.iter().chain([&number]).chain(&self.unit).collect();
        let above = parts.iter().map(|p| p.baseline()).fold(0.0, f32::max);
        let below = parts.iter().map(|p| p.height - p.baseline()).fold(0.0, f32::max);
        let row_h = above + below;

        let total = row_h + RULE_MARGIN + RULE.1 + LABEL_MARGIN + self.label.height;
        let top = (stage.film.height as f32 - total) / 2.0;

        let figure = motion::fade_up(at.seconds, COUNT_FROM, 30.0, 0.6);
        with_opacity(canvas, figure.opacity, || {
            let mut x = LEFT;
            for part in &parts {
                part.draw_on_baseline(canvas, x, top + figure.dy + above);
                x += part.content_width() + GAP;
            }
        });

        let rule_top = top + row_h + RULE_MARGIN;
        let drawn = motion::draw(at.seconds, self.land - 0.3, 0.7, Ease::PowerInOut(2));
        let mut ink = Paint::default();
        ink.set_color(OCHRE);
        canvas.draw_rect(Rect::from_xywh(LEFT + 4.0, rule_top + 5.0, 892.0 * drawn, 4.0), &ink);

        let label = motion::fade_up(at.seconds, self.land - 0.1, 16.0, 0.6);
        with_opacity(canvas, label.opacity, || {
            self.label.draw(canvas, Point::new(LEFT, rule_top + RULE.1 + LABEL_MARGIN + label.dy));
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::group_thousands;

    #[test]
    fn groups_whole_digits_in_threes() {
        assert_eq!(group_thousands("1234567.50"), "1,234,567.50");
        assert_eq!(group_thousands("999"), "999");
        assert_eq!(group_thousands("1000"), "1,000");
    }
}
