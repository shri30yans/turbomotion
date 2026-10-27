//! Measuring text before anything is drawn: the same shaping the renderer uses, answered as numbers so a script can
//! lay words out, right-align a caret or place a label without guessing widths.

use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

use super::{
    bounds::TextBounds,
    layout::{Engine, FontBook, Spec},
};
use crate::schema::Align;

/// What to measure. Every field but `text` has the default a text layer has.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Request {
    pub text: String,
    /// A font role, a file path (relative to the film's folder) or an installed family. Default: the engine's default face.
    pub font: Option<String>,
    pub size: f64,
    pub weight: u16,
    pub italic: bool,
    /// Extra space after every character, px.
    pub tracking: f64,
    /// Line height multiple.
    pub leading: Option<f64>,
    /// Wrap at this width, px.
    pub width: Option<f64>,
    pub align: AlignName,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AlignName {
    #[default]
    Left,
    Center,
    Right,
}

impl Default for Request {
    fn default() -> Self {
        Request { text: String::new(), font: None, size: 64.0, weight: 400, italic: false, tracking: 0.0, leading: None, width: None, align: AlignName::Left }
    }
}

/// One character's box on its line (`x` from the block's left edge, px).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CharBox {
    pub ch: char,
    pub x: f32,
    pub w: f32,
    pub line: usize,
}

/// One word's box.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WordBox {
    pub text: String,
    pub x: f32,
    pub w: f32,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Measured {
    /// Width of the widest line (or the wrap width), px.
    pub width: f32,
    pub height: f32,
    /// Distance from the top of the block to the first line's baseline.
    pub baseline: f32,
    pub lines: usize,
    /// Every character of the text, spaces included, in order.
    pub chars: Vec<CharBox>,
    pub words: Vec<WordBox>,
    /// Where the characters are (`null` for a text with none): the same measurement a text layer's `box` and
    /// `comp inspect` read, with every character showing. `lines` has the extent of each line.
    pub bounds: Option<TextBounds>,
}

/// Shape `req` with `engine`, finding fonts through `book`.
pub fn measure(engine: &Engine, book: &dyn FontBook, req: &Request) -> Result<Measured> {
    ensure!(req.size > 0.0 && req.size.is_finite(), "measure: \"size\" must be a number of pixels above 0, got {}", req.size);
    ensure!((1..=1000).contains(&req.weight), "measure: \"weight\" is 100..900 (400 regular, 700 bold), got {}", req.weight);
    let align = match req.align {
        AlignName::Left => Align::Left,
        AlignName::Center => Align::Center,
        AlignName::Right => Align::Right,
    };
    let spec = Spec::new(&req.text, req.font.as_deref(), Some(req.weight), req.italic, req.size, req.tracking, req.leading, align, req.width);
    let layout = engine.layout(book, &spec)?;
    let mut chars: Vec<CharBox> = Vec::with_capacity(req.text.chars().count());
    let mut glyphs = layout.glyphs.iter().peekable();
    let mut prev_end = 0.0f32;
    let mut prev_line = 0usize;
    for (slot, ch) in req.text.chars().enumerate() {
        match glyphs.peek() {
            Some(g) if g.slot == slot => {
                chars.push(CharBox { ch, x: g.x, w: g.w, line: g.line });
                prev_end = g.x + g.w;
                prev_line = g.line;
                glyphs.next();
            }
            _ => {
                // Whitespace has no glyph: it spans from the end of the previous character to the start of the next one on the same line.
                let next = glyphs.peek().filter(|g| g.line == prev_line).map_or(prev_end, |g| g.x);
                chars.push(CharBox { ch, x: prev_end, w: (next - prev_end).max(0.0), line: prev_line });
            }
        }
    }
    let words = layout
        .words
        .iter()
        .map(|w| {
            let g = &layout.glyphs[w.glyphs.clone()];
            WordBox { text: g.iter().map(|g| g.ch).collect(), x: w.rect[0], w: w.rect[2] - w.rect[0], line: g[0].line }
        })
        .collect();
    let baseline = layout.glyphs.first().map_or(req.size as f32, |g| g.y);
    Ok(Measured { width: layout.width, height: layout.height, baseline, lines: layout.lines.len().max(1), chars, words, bounds: TextBounds::of_layout(&layout) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::layout::Fonts;
    use std::collections::BTreeMap;

    fn book() -> Fonts {
        Fonts { roles: BTreeMap::new(), root: std::path::PathBuf::from(".") }
    }

    fn req(text: &str) -> Request {
        Request { text: text.into(), size: 80.0, ..Request::default() }
    }

    #[test]
    fn a_longer_string_is_wider_and_every_character_has_a_box() {
        let e = Engine::new();
        let (a, b) = (measure(&e, &book(), &req("Hi")).unwrap(), measure(&e, &book(), &req("Hi there")).unwrap());
        assert!(b.width > a.width * 2.0, "{} vs {}", a.width, b.width);
        assert_eq!(b.chars.len(), 8);
        assert_eq!(b.words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(), ["Hi", "there"]);
        let space = &b.chars[2];
        assert!(space.ch == ' ' && space.w > 5.0, "{space:?}");
        assert!((space.x - (b.chars[1].x + b.chars[1].w)).abs() < 0.01);
        assert!((b.chars[3].x - (space.x + space.w)).abs() < 0.01, "the next word starts where the space ends");
    }

    #[test]
    fn tracking_widens_and_wrapping_adds_lines() {
        let e = Engine::new();
        let plain = measure(&e, &book(), &req("abcd")).unwrap();
        let wide = measure(&e, &book(), &Request { tracking: 10.0, ..req("abcd") }).unwrap();
        assert!(wide.width > plain.width + 25.0, "{} vs {}", plain.width, wide.width);
        let wrapped = measure(&e, &book(), &Request { width: Some(f64::from(plain.width) * 1.5), ..req("abcd abcd abcd") }).unwrap();
        assert!(wrapped.lines >= 2, "{}", wrapped.lines);
    }

    #[test]
    fn an_unknown_font_is_named_with_the_declared_roles() {
        let e = Engine::new();
        let mut b = book();
        b.roles.insert("display".into(), "fonts/none.ttf".into());
        let err = format!("{:#}", measure(&e, &b, &Request { font: Some("Nope Sans 9".into()), ..req("x") }).unwrap_err());
        assert!(err.contains("Nope Sans 9") && err.contains("display"), "{err}");
        let err = format!("{:#}", measure(&e, &b, &Request { font: Some("display".into()), ..req("x") }).unwrap_err());
        assert!(err.contains("fonts/none.ttf"), "{err}");
    }
}
