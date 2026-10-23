//! Drawing vocabulary shared by the diagrams (`diagrams/kit.tsx` and the `Person` of `Explainers.tsx`).

use anyhow::Result;
use skia_safe::Color;

use super::node::{Anchor, Anim, Node};
use crate::{
    palette::{PAPER, TERRACOTTA},
    text::{Family, Fonts, Halo, Style},
};

/// `label()`: the sans face used for annotations.
pub fn label(size: f32, color: Color, weight: f32) -> Style {
    Style { letter_spacing: -0.4, ..Style::new(Family::Manrope, size, weight, color) }
}

/// `mono()`: tracked caps for dimensions and captions.
pub fn mono(size: f32, color: Color) -> Style {
    Style::mono(size, 2.0, color)
}

/// `halo(c)`: a paper-coloured outline painted under the text.
pub fn halo(style: Style, color: Color) -> Style {
    Style { halo: Some(Halo { color, width: 8.0 }), ..style }
}

/// Builds text nodes with the film's fonts.
#[derive(Clone, Copy)]
pub struct Kit<'a> {
    pub fonts: &'a Fonts,
}

impl Kit<'_> {
    /// An SVG `<text>` whose baseline starts at `(x, y)`.
    pub fn text(&self, text: &str, x: f32, y: f32, anchor: Anchor, style: Style) -> Result<Node> {
        Ok(Node::text(self.fonts.typeset(text, &style, crate::text::UNBOUNDED, false)?, x, y, anchor))
    }

    /// A tracked-caps label centred on `x`.
    pub fn centred(&self, text: &str, x: f32, y: f32, style: Style) -> Result<Node> {
        self.text(text, x, y, Anchor::Middle, style)
    }

    /// A dimension line with end ticks and a centred label, optionally revealed (`Dim`).
    pub fn dim(&self, from: (f32, f32), to: (f32, f32), text: &str, reveal: Option<&str>) -> Result<Node> {
        let (x1, y1) = from;
        let (x2, y2) = to;
        let angle = (y2 - y1).atan2(x2 - x1);
        let (nx, ny) = (-angle.sin() * 14.0, angle.cos() * 14.0);
        let line = Node::path(&format!(
            "M{x1} {y1} L{x2} {y2} M{} {} L{} {} M{} {} L{} {}",
            x1 + nx, y1 + ny, x1 - nx, y1 - ny, x2 + nx, y2 + ny, x2 - nx, y2 - ny
        ))?
        .no_fill()
        .stroke(TERRACOTTA, 3.0);
        let caption = self.centred(text, (x1 + x2) / 2.0, (y1 + y2) / 2.0 - 16.0, halo(label(30.0, TERRACOTTA, 700.0), PAPER))?;
        let group = Node::group([line, caption]);
        Ok(match reveal {
            Some(spec) => group.reveal(spec, Anim::fade()),
            None => group,
        })
    }
}

/// A standing figure `h` tall with its feet at `(x, ground)`.
pub fn person(x: f32, ground: f32, h: f32, color: Color) -> Result<Node> {
    let head = h * 0.13;
    let head_circle = Node::circle(x, ground - h + head, head).fill(color);
    let body = Node::path(&format!(
        "M{} {} Q{} {} {} {} L{} {} L{} {} L{} {} L{} {} L{} {} L{} {} L{} {} Z",
        x - h * 0.13, ground - h + head * 2.3, x, ground - h + head * 1.9, x + h * 0.13, ground - h + head * 2.3,
        x + h * 0.11, ground - h * 0.48, x + h * 0.08, ground, x + h * 0.02, ground, x, ground - h * 0.42,
        x - h * 0.02, ground, x - h * 0.08, ground, x - h * 0.11, ground - h * 0.48
    ))?
    .fill(color);
    Ok(Node::group([head_circle, body]))
}
