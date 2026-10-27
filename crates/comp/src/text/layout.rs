//! Text layout: shaping and line breaking by Skia's paragraph engine, then per-character positions so every
//! character, word and line can be moved on its own.

use std::{cell::RefCell, collections::{BTreeMap, HashMap}, path::{Path, PathBuf}, rc::Rc};

use anyhow::{anyhow, Context, Result};
use skia_safe::{
    font_style::{Slant, Weight, Width},
    textlayout::{FontCollection, ParagraphBuilder, ParagraphStyle, RectHeightStyle, RectWidthStyle, TextAlign, TextStyle, TypefaceFontProvider},
    Font, FontMgr, FontStyle, Typeface,
};
use turbomotion_core::typeface;

use crate::{plan::Plan, schema::Align};

/// What the text engine needs to find a font: the file's font roles and where relative font paths start.
/// A composition's [`Plan`] is one; [`Fonts`] is the same thing without a plan (build-time measuring).
pub trait FontBook {
    /// The font a role name (`"display"`) stands for, if the file declares it.
    fn role(&self, name: &str) -> Option<&str>;
    /// Every declared role name, for error messages.
    fn roles(&self) -> Vec<&str>;
    /// A font path as written, made absolute against the film root.
    fn resolve(&self, rel: &str) -> PathBuf;
}

impl FontBook for Plan {
    fn role(&self, name: &str) -> Option<&str> {
        self.fonts.get(name).map(String::as_str)
    }
    fn roles(&self) -> Vec<&str> {
        self.fonts.keys().map(String::as_str).collect()
    }
    fn resolve(&self, rel: &str) -> PathBuf {
        Plan::resolve(self, rel)
    }
}

/// Font roles and a root folder, for measuring text before any composition exists.
pub struct Fonts {
    pub roles: BTreeMap<String, String>,
    pub root: PathBuf,
}

impl FontBook for Fonts {
    fn role(&self, name: &str) -> Option<&str> {
        self.roles.get(name).map(String::as_str)
    }
    fn roles(&self) -> Vec<&str> {
        self.roles.keys().map(String::as_str).collect()
    }
    fn resolve(&self, rel: &str) -> PathBuf {
        let p = Path::new(rel);
        if p.is_absolute() { p.to_path_buf() } else { self.root.join(p) }
    }
}

/// Families tried after the designed face, so a line can mix Latin and emoji without swapping the whole font.
const FALLBACK_FAMILIES: &[&str] = &["Apple Color Emoji", "Segoe UI Emoji", "Noto Color Emoji"];

/// One drawable character at rest, in layout space (origin: top-left of the text block).
#[derive(Debug, Clone)]
pub struct Glyph {
    pub ch: char,
    pub x: f32,
    /// The baseline.
    pub y: f32,
    pub w: f32,
    /// What `w` gains when no character follows this one on its line: the pair kerning with the next character is gone,
    /// so the box is the character's own advance (and its tracking). Zero for the last character of a line, and for a
    /// character in a font that does not kern it, or that ends a ligature (`fi`).
    pub trail: f32,
    pub top: f32,
    pub bottom: f32,
    pub line: usize,
    pub word: usize,
    /// Index of this character in the text counting every character, spaces included (a typewriter that counts spaces uses it).
    pub slot: usize,
    /// The face that shaped this character (the designed font, or a fallback for glyphs it does not have).
    pub font: Font,
}

/// A run of glyphs (a word or a line) and its box `[left, top, right, bottom]`.
#[derive(Debug, Clone)]
pub struct Span {
    pub glyphs: std::ops::Range<usize>,
    pub rect: [f32; 4],
}

pub struct Layout {
    pub glyphs: Vec<Glyph>,
    pub words: Vec<Span>,
    pub lines: Vec<Span>,
    pub width: f32,
    pub height: f32,
    /// The point the layer's position refers to: the alignment edge, vertically the middle.
    pub anchor: [f32; 2],
    pub font: Font,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Spec {
    pub text: String,
    pub font: Option<String>,
    pub weight: u16,
    pub italic: bool,
    /// Size in 1/100 px.
    pub size: i64,
    /// Tracking in 1/100 px.
    pub tracking: i64,
    /// Line height multiple in 1/100.
    pub leading: Option<i64>,
    pub align: u8,
    pub width: Option<i64>,
    /// Restyled stretches, as character ranges of `text`.
    pub runs: Vec<RunSpec>,
}

/// A stretch of characters (`start..end`) drawn in another face than the rest of the line.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunSpec {
    pub start: usize,
    pub end: usize,
    pub font: Option<String>,
    pub weight: Option<u16>,
    pub italic: Option<bool>,
}

impl Spec {
    // one argument per field of the spec, in the order callers read them
    #[allow(clippy::too_many_arguments)]
    pub fn new(text: &str, font: Option<&str>, weight: Option<u16>, italic: bool, size: f64, tracking: f64, leading: Option<f64>, align: Align, width: Option<f64>) -> Spec {
        Spec {
            text: text.to_string(),
            font: font.map(str::to_string),
            weight: weight.unwrap_or(400),
            italic,
            size: (size * 100.0).round() as i64,
            tracking: (tracking * 100.0).round() as i64,
            leading: leading.map(|l| (l * 100.0).round() as i64),
            align: match align {
                Align::Left => 0,
                Align::Center => 1,
                Align::Right => 2,
            },
            width: width.map(|w| (w * 100.0).round() as i64),
            runs: Vec::new(),
        }
    }

    pub fn with_runs(mut self, runs: Vec<RunSpec>) -> Spec {
        self.runs = runs;
        self
    }
}

/// Find the typeface a font spec names: a role from `fonts`, a file path (`fonts/Anton.ttf`, `x.ttc#2`) or an
/// installed family. The error names the spec as written.
pub fn resolve_typeface(book: &dyn FontBook, font: Option<&str>, weight: u16, italic: bool) -> Result<Typeface> {
    {
        let spec = font.map(|f| book.role(f).unwrap_or(f));
        match spec {
            None => typeface::from_family("Helvetica Neue", weight, italic).or_else(|_| typeface::from_family("Helvetica", weight, italic)).or_else(|_| {
                FontMgr::new().legacy_make_typeface(None, FontStyle::new(Weight::from(i32::from(weight)), Width::NORMAL, if italic { Slant::Italic } else { Slant::Upright })).ok_or_else(|| anyhow!("no default font"))
            }),
            Some(s) if s.contains('/') || [".ttf", ".otf", ".ttc", ".otc", ".woff"].iter().any(|e| s.to_ascii_lowercase().contains(e)) => {
                let (file, index) = s.split_once('#').map_or((s, 0), |(f, i)| (f, i.parse().unwrap_or(0)));
                typeface::from_file(&book.resolve(file), index)
            }
            Some(family) => typeface::from_family(family, weight, italic).map_err(|_| anyhow!("{}", super::missing_font::explain(book, family, &typeface::families()))),
        }
    }
    .with_context(|| format!("font {:?}", font.unwrap_or("(default)")))
}

/// Typefaces and finished layouts, shared by every text layer of a renderer.
pub struct Engine {
    faces: RefCell<HashMap<String, (Typeface, String)>>,
    layouts: RefCell<HashMap<Spec, Rc<Layout>>>,
    provider: RefCell<TypefaceFontProvider>,
    collection: FontCollection,
}

impl Engine {
    pub fn new() -> Engine {
        let provider = TypefaceFontProvider::new();
        let mut collection = FontCollection::new();
        collection.set_asset_font_manager(Some((*provider).clone()));
        collection.set_default_font_manager(FontMgr::new(), None);
        collection.enable_font_fallback();
        Engine { faces: RefCell::new(HashMap::new()), layouts: RefCell::new(HashMap::new()), provider: RefCell::new(provider), collection }
    }

    /// Resolve a font spec: a role from `fonts`, a file path (`fonts/Anton.ttf`, `x.ttc#2`) or an installed family.
    fn face(&self, book: &dyn FontBook, font: Option<&str>, weight: u16, italic: bool) -> Result<(Typeface, String)> {
        let key = format!("{}|{weight}|{italic}", font.unwrap_or(""));
        if let Some(f) = self.faces.borrow().get(&key) {
            return Ok(f.clone());
        }
        let tf = resolve_typeface(book, font, weight, italic)?;
        let alias = format!("tm-face-{}", self.faces.borrow().len());
        let family = tf.family_name();
        {
            let mut provider = self.provider.borrow_mut();
            provider.register_typeface(tf.clone(), alias.as_str());
            if !family.is_empty() && family != alias {
                provider.register_typeface(tf.clone(), family.as_str());
            }
        }
        self.faces.borrow_mut().insert(key, (tf.clone(), alias.clone()));
        Ok((tf, alias))
    }

    pub fn layout(&self, book: &dyn FontBook, spec: &Spec) -> Result<Rc<Layout>> {
        if let Some(l) = self.layouts.borrow().get(spec) {
            return Ok(l.clone());
        }
        let layout = Rc::new(self.build(book, spec)?);
        let mut cache = self.layouts.borrow_mut();
        if cache.len() > 512 {
            cache.clear();
        }
        cache.insert(spec.clone(), layout.clone());
        Ok(layout)
    }

    fn build(&self, book: &dyn FontBook, spec: &Spec) -> Result<Layout> {
        let (tf, alias) = self.face(book, spec.font.as_deref(), spec.weight, spec.italic)?;
        let size = spec.size as f32 / 100.0;
        let mut families = vec![alias.as_str()];
        families.extend(FALLBACK_FAMILIES.iter().copied());
        let mut ts = TextStyle::new();
        ts.set_font_size(size)
            .set_font_families(&families)
            .set_letter_spacing(spec.tracking as f32 / 100.0)
            .set_color(skia_safe::Color::WHITE);
        if let Some(l) = spec.leading {
            ts.set_height(l as f32 / 100.0).set_height_override(true);
        }
        let align = match spec.align {
            0 => TextAlign::Left,
            1 => TextAlign::Center,
            _ => TextAlign::Right,
        };
        let mut ps = ParagraphStyle::new();
        ps.set_text_align(align).set_apply_rounding_hack(false);
        // One style per stretch: the line's own, and each run's override of face, weight or slant.
        let mut stretches: Vec<(String, TextStyle)> = Vec::new();
        let chars: Vec<char> = spec.text.chars().collect();
        let mut cuts: Vec<usize> = vec![0, chars.len()];
        for r in &spec.runs {
            cuts.extend([r.start.min(chars.len()), r.end.min(chars.len())]);
        }
        cuts.sort_unstable();
        cuts.dedup();
        for w in cuts.windows(2) {
            let (a, b) = (w[0], w[1]);
            let run = spec.runs.iter().rev().find(|r| r.start <= a && b <= r.end);
            let mut style = ts.clone();
            if let Some(r) = run {
                let (_, run_alias) = self.face(book, r.font.as_deref().or(spec.font.as_deref()), r.weight.unwrap_or(spec.weight), r.italic.unwrap_or(spec.italic))?;
                let mut fams = vec![run_alias.as_str()];
                fams.extend(FALLBACK_FAMILIES.iter().copied());
                style.set_font_families(&fams);
            }
            stretches.push((chars[a..b].iter().collect(), style));
        }
        let make = |width: f32| {
            let mut pb = ParagraphBuilder::new(&ps, &self.collection);
            for (s, style) in &stretches {
                pb.push_style(style);
                pb.add_text(s);
                pb.pop();
            }
            let mut p = pb.build();
            p.layout(width);
            p
        };
        let natural = make(1.0e6);
        let width = spec.width.map_or_else(|| natural.longest_line().ceil().max(1.0), |w| w as f32 / 100.0);
        let mut p = if spec.width.is_some() { make(width) } else { make(width + 0.5) };
        let metrics: Vec<(f64, f64, f64)> = p.get_line_metrics().iter().map(|m| (m.baseline, m.ascent, m.descent)).collect();
        let mut primary = Font::from_typeface(tf, size);
        prepare_font(&mut primary);

        let mut glyphs: Vec<Glyph> = Vec::new();
        let (mut utf16, mut word, mut slot) = (0usize, 0usize, 0usize);
        let mut word_open = false;
        for ch in spec.text.chars() {
            let len = ch.len_utf16();
            if ch.is_whitespace() {
                if word_open {
                    word += 1;
                    word_open = false;
                }
            } else {
                let rects = p.get_rects_for_range(utf16..utf16 + len, RectHeightStyle::Max, RectWidthStyle::Tight);
                if let (Some(b), Some(line)) = (rects.first(), p.get_line_number_at_utf16_offset(utf16)) {
                    let (baseline, ascent, descent) = metrics[line.min(metrics.len() - 1)];
                    let r = b.rect;
                    let mut face = p.get_font_at_utf16_offset(utf16);
                    prepare_font(&mut face);
                    let font = if face.unichar_to_glyph(ch as i32) == 0 { primary.clone() } else { face };
                    // the last character of a ligature (`fi`) ends the cluster: nothing was pulled from it
                    let ends_ligature = p.get_glyph_cluster_at(utf16).is_some_and(|c| c.text_range.len() > len && c.text_range.end == utf16 + len);
                    let trail = if ends_ligature { 0.0 } else { font.measure_str(ch.to_string(), None).0 + spec.tracking as f32 / 100.0 - r.width() };
                    glyphs.push(Glyph { ch, x: r.left, y: baseline as f32, w: r.width(), trail, top: (baseline - ascent) as f32, bottom: (baseline + descent) as f32, line, word, slot, font });
                    word_open = true;
                }
            }
            utf16 += len;
            slot += 1;
        }
        let span = |ids: std::ops::Range<usize>, glyphs: &[Glyph]| -> Span {
            let g = &glyphs[ids.clone()];
            let l = g.iter().map(|g| g.x).fold(f32::INFINITY, f32::min);
            let r = g.iter().map(|g| g.x + g.w).fold(f32::NEG_INFINITY, f32::max);
            let t = g.iter().map(|g| g.top).fold(f32::INFINITY, f32::min);
            let b = g.iter().map(|g| g.bottom).fold(f32::NEG_INFINITY, f32::max);
            Span { glyphs: ids, rect: [l, t, r, b] }
        };
        let group = |key: &dyn Fn(&Glyph) -> usize| -> Vec<Span> {
            let mut out = vec![];
            let mut start = 0;
            for i in 1..=glyphs.len() {
                if i == glyphs.len() || key(&glyphs[i]) != key(&glyphs[start]) {
                    if i > start {
                        out.push(span(start..i, &glyphs));
                    }
                    start = i;
                }
            }
            out
        };
        let words = group(&|g: &Glyph| g.word * 10_000 + g.line);
        let lines = group(&|g: &Glyph| g.line);
        let height = p.height();
        let ax = match spec.align {
            0 => 0.0,
            1 => width / 2.0,
            _ => width,
        };
        Ok(Layout { glyphs, words, lines, width, height, anchor: [ax, height / 2.0], font: primary })
    }
}

fn prepare_font(font: &mut Font) {
    font.set_subpixel(true).set_edging(skia_safe::font::Edging::AntiAlias).set_hinting(skia_safe::FontHinting::None);
}

/// Bitmap / COLR glyphs have an id but no outline, so paint uses them as ink in the layer colour.
pub fn is_color_glyph(font: &Font, ch: char) -> bool {
    let id = font.unichar_to_glyph(ch as i32);
    id != 0 && font.get_path(id).is_none()
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::Plan;
    use serde_json::json;

    #[test]
    fn a_missing_glyph_uses_a_fallback_face_and_keeps_the_designed_one_for_the_rest() {
        let plan = Plan::from_value(json!({"dur": 1, "layers": [{"type": "solid", "color": "#000"}]}), std::path::Path::new(".")).unwrap();
        let layout = Engine::new().layout(&plan, &Spec::new("LOSS 📉", None, Some(700), false, 72.0, 0.0, None, Align::Left, None)).unwrap();
        let latin = layout.glyphs.iter().find(|g| g.ch == 'L').expect("L");
        let emoji = layout.glyphs.iter().find(|g| g.ch == '📉').expect("chart");
        let latin_family = latin.font.typeface().family_name();
        let emoji_family = emoji.font.typeface().family_name();
        assert_ne!(latin_family, emoji_family, "the emoji still sits on {latin_family}");
        assert!(emoji.font.unichar_to_glyph(emoji.ch as i32) != 0, "no glyph for 📉 on {emoji_family}");
        assert!(emoji.w > 8.0, "emoji advance {}", emoji.w);
    }
}
