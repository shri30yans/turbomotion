//! Text layers: one or more lines of tracked, aligned, optionally wrapped text drawn into a transparent layer, laid out
//! the way `fx.text_layer` does (advance-based tracking, line gap, padding) so every caller keeps its coordinates.

use std::rc::Rc;

use anyhow::Result;
use skia_safe::{Canvas, Color, Paint};

use super::{
    cache::Cache,
    font::Fonts,
    layer::Layer,
    model::Rgb,
};

/// Everything that determines a text layer's pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct TextSpec {
    pub text: String,
    pub font: String,
    pub size: f64,
    pub fill: Rgb,
    pub tracking: f64,
    pub pad: i32,
    pub align: Align,
    pub line_gap: f64,
    pub maxw: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Align {
    Left,
    Center,
    Right,
}

impl Align {
    pub fn parse(name: &str) -> Self {
        match name {
            "center" => Self::Center,
            "right" => Self::Right,
            _ => Self::Left,
        }
    }
}

impl TextSpec {
    /// A spec with the engine's usual defaults (`T()` in the reference: 10 px padding, centred).
    pub fn new(text: impl Into<String>, font: &str, size: f64, fill: Rgb) -> Self {
        Self { text: text.into(), font: font.into(), size, fill, tracking: 0.0, pad: 10, align: Align::Center, line_gap: 1.12, maxw: None }
    }
    pub fn tracking(mut self, t: f64) -> Self {
        self.tracking = t;
        self
    }
    pub fn align(mut self, a: Align) -> Self {
        self.align = a;
        self
    }
    pub fn pad(mut self, p: i32) -> Self {
        self.pad = p;
        self
    }
    pub fn maxw(mut self, w: Option<f64>) -> Self {
        self.maxw = w;
        self
    }

    fn key(&self) -> (String, String, i32, Rgb, i64, i32, Align, Option<i64>) {
        (self.text.clone(), self.font.clone(), self.size as i32, self.fill, (self.tracking * 100.0) as i64, self.pad, self.align, self.maxw.map(|w| w as i64))
    }
}

pub struct Typesetter {
    fonts: Fonts,
    layers: Cache<(String, String, i32, Rgb, i64, i32, Align, Option<i64>), Layer>,
}

impl Typesetter {
    pub fn new(fonts: Fonts) -> Self {
        Self { fonts, layers: Cache::new(512) }
    }

    pub fn fonts(&self) -> &Fonts {
        &self.fonts
    }

    /// The text layer for `spec`, from the cache when it was drawn before.
    pub fn layer(&self, like: &Canvas, spec: &TextSpec) -> Result<Rc<Layer>> {
        self.layers.get_or_try_insert(spec.key(), || self.draw(like, spec))
    }

    fn lines(&self, spec: &TextSpec) -> Result<Vec<String>> {
        let face = self.fonts.face(&spec.font, spec.size)?;
        let Some(maxw) = spec.maxw else { return Ok(spec.text.split('\n').map(String::from).collect()) };
        let spaced = spec.text.replace('\n', " \n ");
        let (mut lines, mut cur) = (Vec::new(), String::new());
        for word in spaced.split(' ') {
            if word == "\n" {
                lines.push(std::mem::take(&mut cur));
                continue;
            }
            let joined = format!("{cur} {word}").trim().to_string();
            if f64::from(face.size(&joined, spec.tracking).0) <= maxw || cur.is_empty() {
                cur = joined;
            } else {
                lines.push(std::mem::replace(&mut cur, word.to_string()));
            }
        }
        lines.push(cur);
        Ok(lines)
    }

    fn draw(&self, like: &Canvas, spec: &TextSpec) -> Result<Layer> {
        let face = self.fonts.face(&spec.font, spec.size)?;
        let lines = self.lines(spec)?;
        let line_h = (f64::from(face.ascent + face.descent) * spec.line_gap) as i32;
        let widths: Vec<i32> = lines.iter().map(|l| if l.is_empty() { 0 } else { face.size(l, spec.tracking).0 }).collect();
        let w = widths.iter().copied().max().unwrap_or(1).max(1) + 2 * spec.pad + 4;
        let h = line_h * lines.len() as i32 + 2 * spec.pad;
        let mut paint = Paint::default();
        paint.set_anti_alias(true).set_color(Color::from_rgb(spec.fill[0], spec.fill[1], spec.fill[2]));
        Layer::render(like, w, h, |c| {
            for (i, line) in lines.iter().enumerate() {
                let x = match spec.align {
                    Align::Left => f64::from(spec.pad),
                    Align::Center => f64::from(w - widths[i]) / 2.0,
                    Align::Right => f64::from(w - spec.pad - widths[i]),
                };
                let baseline = spec.pad + i as i32 * line_h + face.ascent;
                let laid = face.layout(line, spec.tracking);
                c.save();
                c.translate((x as f32, baseline as f32));
                c.draw_path(&laid.path, &paint);
                c.restore();
            }
        })
    }
}
