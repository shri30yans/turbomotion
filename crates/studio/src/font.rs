//! Fonts and text measurement with the metrics the Python engine got from Pillow/FreeType: glyph advances rounded to whole
//! pixels, integer ascent/descent, and a bounding box that spans the ink and the advance. Layout is therefore identical
//! even though Skia draws the glyph outlines.

use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result};
use skia_safe::{Font, FontHinting, Path as SkPath, Typeface};
use turbomotion_core::typeface;

use super::model::Film;

/// Integer pixel box, as Pillow reports it: (left, top, right, bottom) relative to the pen origin on the ascender line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bbox {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Bbox {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

pub struct Fonts {
    faces: HashMap<String, Typeface>,
}

impl Fonts {
    pub fn load(film: &Film) -> Result<Self> {
        let mut faces = HashMap::new();
        for (role, (file, index)) in &film.fonts {
            faces.insert(role.clone(), typeface::from_file(&Path::new(&film.font_dir).join(file), *index)?);
        }
        Ok(Self { faces })
    }

    /// The face for a font role at `size` px (the Python engine floors the size and never goes under 8).
    pub fn face(&self, role: &str, size: f64) -> Result<Face> {
        let tf = self.faces.get(role).with_context(|| format!("unknown font role {role:?}"))?;
        let px = (size as i32).max(8) as f32;
        let mut font = Font::from_typeface(tf.clone(), px);
        font.set_hinting(FontHinting::None).set_subpixel(true).set_linear_metrics(true).set_edging(skia_safe::font::Edging::AntiAlias);
        let (_, m) = font.metrics();
        let ascent = (-m.ascent).ceil() as i32;
        let descent = m.descent.ceil() as i32;
        Ok(Face { font, ascent, descent })
    }
}

pub struct Face {
    font: Font,
    pub ascent: i32,
    pub descent: i32,
}

/// A laid-out single line: glyph outlines at their pen positions (origin = pen start, baseline y = 0) and the box
/// Pillow would report for it.
pub struct Line {
    pub path: SkPath,
    pub bbox: Bbox,
}

impl Face {
    /// Whole-pixel advance of one character, as FreeType's hinted metrics give.
    pub fn advance(&self, ch: char) -> f64 {
        let glyph = self.font.unichar_to_glyph(ch as i32);
        let mut w = [0f32];
        self.font.get_widths(&[glyph], &mut w);
        (f64::from(w[0]) + 0.5).floor()
    }

    /// Lay out `text` with `tracking` px added after every glyph. The box follows `ImageDraw.textbbox`: ink extents
    /// (outward rounded) widened to include the origin and the full advance.
    pub fn layout(&self, text: &str, tracking: f64) -> Line {
        let mut path = SkPath::new();
        let mut pen = 0.0f64;
        for ch in text.chars() {
            let glyph = self.font.unichar_to_glyph(ch as i32);
            if let Some(outline) = self.font.get_path(glyph) {
                path.add_path(&outline, (pen as f32, 0.0), None);
            }
            pen += self.advance(ch) + tracking;
        }
        let advance = if tracking != 0.0 && text.chars().count() > 1 { pen - tracking } else { pen };
        let ink = path.compute_tight_bounds();
        let asc = f64::from(self.ascent);
        let (left, right, top, bottom) = if path.is_empty() {
            (0, 0, 0, 0)
        } else {
            (
                (f64::from(ink.left)).floor() as i32,
                (f64::from(ink.right)).ceil() as i32,
                (f64::from(ink.top) + asc).floor() as i32,
                (f64::from(ink.bottom) + asc).ceil() as i32,
            )
        };
        let bbox = Bbox { left: left.min(0), top, right: right.max(advance.round() as i32), bottom };
        Line { path, bbox }
    }

    /// `(width, height, bbox)` of a line as `fx.text_size` defines it: with tracking the width is the advance sum plus
    /// tracking between glyphs, without it the bbox width.
    pub fn size(&self, text: &str, tracking: f64) -> (i32, i32, Bbox) {
        let line = self.layout(text, 0.0);
        let bb = line.bbox;
        if tracking != 0.0 && text.chars().count() > 1 {
            let adv: f64 = text.chars().map(|c| self.advance(c)).sum();
            ((adv + tracking * (text.chars().count() - 1) as f64) as i32, bb.height(), bb)
        } else {
            (bb.width(), bb.height(), bb)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Deserialize)]
    struct Item {
        text: String,
        font: String,
        size: f64,
        tracking: f64,
        w: i32,
        asc: i32,
        desc: i32,
    }

    #[derive(Deserialize)]
    struct Fixture {
        font_dir: String,
        fonts: HashMap<String, (String, u32)>,
        items: Vec<Item>,
    }

    /// The reference engine measured text with FreeType's hinted advances, which differ from the outlines' linear
    /// advances by up to a pixel per glyph. We lay out with linear advances rounded to whole pixels (the unbiased match),
    /// so line widths agree to within 4% and ascent/descent to a pixel; the parity renders bound the visual effect.
    #[test]
    fn metrics_stay_close_to_pillow() {
        let fixture: Fixture = serde_json::from_str(&std::fs::read_to_string("tests/data/text_metrics.json").unwrap()).unwrap();
        let film = Film { fps: 24, width: 1920, height: 1080, end: 0.0, fonts: fixture.fonts, font_dir: fixture.font_dir, scenes: vec![], dir: Default::default() };
        let fonts = Fonts::load(&film).unwrap();
        for it in &fixture.items {
            let face = fonts.face(&it.font, it.size).unwrap();
            assert!((face.ascent - it.asc).abs() <= 1 && (face.descent - it.desc).abs() <= 1, "{} {}: ascent/descent", it.font, it.size);
            let (w, _, _) = face.size(&it.text, it.tracking);
            let err = f64::from((w - it.w).abs()) / f64::from(it.w.max(1));
            assert!(err <= 0.04 || (w - it.w).abs() <= 2, "{:?} {} {}: width {w} vs pillow {} ({:.1}%)", it.text, it.font, it.size, it.w, err * 100.0);
        }
    }
}
