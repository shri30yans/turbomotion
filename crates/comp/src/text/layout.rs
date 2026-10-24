//! Text layout: shaping and line breaking by Skia's paragraph engine, then per-character positions so every
//! character, word and line can be moved on its own.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use anyhow::{anyhow, Context, Result};
use skia_safe::{
    font_style::{Slant, Weight, Width},
    textlayout::{FontCollection, ParagraphBuilder, ParagraphStyle, RectHeightStyle, RectWidthStyle, TextAlign, TextStyle, TypefaceFontProvider},
    Font, FontMgr, FontStyle, Typeface,
};
use turbomotion_core::typeface;

use crate::{plan::Plan, schema::Align};

/// One drawable character at rest, in layout space (origin: top-left of the text block).
#[derive(Debug, Clone)]
pub struct Glyph {
    pub ch: char,
    pub x: f32,
    /// The baseline.
    pub y: f32,
    pub w: f32,
    pub top: f32,
    pub bottom: f32,
    pub line: usize,
    pub word: usize,
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
}

impl Spec {
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
        }
    }
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
        Engine { faces: RefCell::new(HashMap::new()), layouts: RefCell::new(HashMap::new()), provider: RefCell::new(provider), collection }
    }

    /// Resolve a font spec: a role from `fonts`, a file path (`fonts/Anton.ttf`, `x.ttc#2`) or an installed family.
    fn face(&self, plan: &Plan, font: Option<&str>, weight: u16, italic: bool) -> Result<(Typeface, String)> {
        let key = format!("{}|{weight}|{italic}", font.unwrap_or(""));
        if let Some(f) = self.faces.borrow().get(&key) {
            return Ok(f.clone());
        }
        let tf = (|| -> Result<Typeface> {
            let spec = font.map(|f| plan.fonts.get(f).map_or(f, String::as_str));
            match spec {
                None => typeface::from_family("Helvetica Neue", weight, italic).or_else(|_| typeface::from_family("Helvetica", weight, italic)).or_else(|_| {
                    FontMgr::new().legacy_make_typeface(None, FontStyle::new(Weight::from(i32::from(weight)), Width::NORMAL, if italic { Slant::Italic } else { Slant::Upright })).ok_or_else(|| anyhow!("no default font"))
                }),
                Some(s) if s.contains('/') || [".ttf", ".otf", ".ttc", ".otc"].iter().any(|e| s.to_ascii_lowercase().contains(e)) => {
                    let (file, index) = s.split_once('#').map_or((s, 0), |(f, i)| (f, i.parse().unwrap_or(0)));
                    typeface::from_file(&plan.resolve(file), index)
                }
                Some(family) => typeface::from_family(family, weight, italic).map_err(|_| {
                    let all = typeface::families();
                    let near: Vec<_> = all.iter().filter(|f| f.to_lowercase().contains(&family.to_lowercase().split_whitespace().next().unwrap_or("").to_string())).take(8).cloned().collect();
                    anyhow!("no installed font family {family:?}{}. Try `turbomotion comp fonts` to list them, or give a font file path.", if near.is_empty() { String::new() } else { format!(" (similar: {})", near.join(", ")) })
                }),
            }
        })()
        .with_context(|| format!("font {:?}", font.unwrap_or("(default)")))?;
        let alias = format!("tm-face-{}", self.faces.borrow().len());
        self.provider.borrow_mut().register_typeface(tf.clone(), alias.as_str());
        self.faces.borrow_mut().insert(key, (tf.clone(), alias.clone()));
        Ok((tf, alias))
    }

    pub fn layout(&self, plan: &Plan, spec: &Spec) -> Result<Rc<Layout>> {
        if let Some(l) = self.layouts.borrow().get(spec) {
            return Ok(l.clone());
        }
        let layout = Rc::new(self.build(plan, spec)?);
        let mut cache = self.layouts.borrow_mut();
        if cache.len() > 512 {
            cache.clear();
        }
        cache.insert(spec.clone(), layout.clone());
        Ok(layout)
    }

    fn build(&self, plan: &Plan, spec: &Spec) -> Result<Layout> {
        let (tf, alias) = self.face(plan, spec.font.as_deref(), spec.weight, spec.italic)?;
        let size = spec.size as f32 / 100.0;
        let mut ts = TextStyle::new();
        ts.set_font_size(size).set_font_families(&[alias.as_str()]).set_letter_spacing(spec.tracking as f32 / 100.0).set_color(skia_safe::Color::WHITE);
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
        let make = |width: f32| {
            let mut pb = ParagraphBuilder::new(&ps, &self.collection);
            pb.push_style(&ts);
            pb.add_text(&spec.text);
            let mut p = pb.build();
            p.layout(width);
            p
        };
        let natural = make(1.0e6);
        let width = spec.width.map_or_else(|| natural.longest_line().ceil().max(1.0), |w| w as f32 / 100.0);
        let p = if spec.width.is_some() { make(width) } else { make(width + 0.5) };
        let metrics = p.get_line_metrics();

        let mut glyphs: Vec<Glyph> = Vec::new();
        let (mut utf16, mut word, mut prev_space) = (0usize, 0usize, true);
        let mut word_open = false;
        for ch in spec.text.chars() {
            let len = ch.len_utf16();
            if ch.is_whitespace() {
                if word_open {
                    word += 1;
                    word_open = false;
                }
                prev_space = true;
            } else {
                let rects = p.get_rects_for_range(utf16..utf16 + len, RectHeightStyle::Max, RectWidthStyle::Tight);
                if let (Some(b), Some(line)) = (rects.first(), p.get_line_number_at_utf16_offset(utf16)) {
                    let m = &metrics[line.min(metrics.len() - 1)];
                    let r = b.rect;
                    glyphs.push(Glyph { ch, x: r.left, y: m.baseline as f32, w: r.width(), top: (m.baseline - m.ascent) as f32, bottom: (m.baseline + m.descent) as f32, line, word });
                    word_open = true;
                }
                prev_space = false;
            }
            utf16 += len;
        }
        let _ = prev_space;
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
        let mut font = Font::from_typeface(tf, size);
        font.set_subpixel(true).set_edging(skia_safe::font::Edging::AntiAlias).set_hinting(skia_safe::FontHinting::None);
        Ok(Layout { glyphs, words, lines, width, height, anchor: [ax, height / 2.0], font })
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
