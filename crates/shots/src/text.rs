//! Text: the film's three bundled fonts, Skia paragraph layout, and the mask reveals
//! (`lines` / `chars`) that GSAP SplitText performs in the browser.

use std::{fs, path::Path};

use anyhow::{bail, Context, Result};
use skia_safe::{
    font_arguments::{variation_position::Coordinate, VariationPosition},
    textlayout::{
        FontCollection, Paragraph, ParagraphBuilder, ParagraphStyle, RectHeightStyle, RectWidthStyle, TextAlign, TextShadow,
        TextStyle, TypefaceFontProvider,
    },
    Canvas, Color, FontArguments, FontMgr, FourByteTag, Paint, PaintJoin, PaintStyle, Point, Rect,
};

use crate::probe;

/// Effectively unlimited line width.
pub const UNBOUNDED: f32 = 1.0e6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Fraunces,
    Manrope,
    Plex,
}

impl Family {
    const ALL: [Family; 3] = [Family::Fraunces, Family::Manrope, Family::Plex];

    /// CSS family name registered for the face.
    fn name(self) -> &'static str {
        match self {
            Self::Fraunces => "FrauncesFilm",
            Self::Manrope => "ManropeFilm",
            Self::Plex => "PlexFilm",
        }
    }

    fn file(self) -> &'static str {
        match self {
            Self::Fraunces => "Fraunces.ttf",
            Self::Manrope => "manrope.ttf",
            Self::Plex => "plexmono.ttf",
        }
    }
}

/// A CSS `text-shadow`; `blur` is the CSS blur radius (Skia wants its sigma, half of it).
#[derive(Debug, Clone, Copy)]
pub struct Shadow {
    pub color: Color,
    pub dx: f32,
    pub dy: f32,
    pub blur: f32,
}

/// An SVG `paint-order: stroke` outline drawn under the fill.
#[derive(Debug, Clone, Copy)]
pub struct Halo {
    pub color: Color,
    pub width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub family: Family,
    pub size: f32,
    pub weight: f32,
    pub color: Color,
    pub letter_spacing: f32,
    /// CSS `line-height` as a multiple of the font size; `None` is `normal`.
    pub line_height: Option<f32>,
    pub shadow: Option<Shadow>,
    pub halo: Option<Halo>,
    pub opsz: Option<f32>,
    pub soft: Option<f32>,
    /// CSS `font-variant-numeric: tabular-nums`.
    pub tabular: bool,
    /// Synthetic oblique: the bundled fonts ship no italic faces, so browsers slant the upright.
    pub italic: bool,
    pub align: Align,
}

impl Style {
    pub fn new(family: Family, size: f32, weight: f32, color: Color) -> Self {
        Self {
            family,
            size,
            weight,
            color,
            letter_spacing: 0.0,
            line_height: None,
            shadow: None,
            halo: None,
            opsz: None,
            soft: None,
            tabular: false,
            italic: false,
            align: Align::Left,
        }
    }

    /// The film's label face: Plex Mono 500 with tracking.
    pub fn mono(size: f32, letter_spacing: f32, color: Color) -> Self {
        Self { letter_spacing, ..Self::new(Family::Plex, size, 500.0, color) }
    }
}

/// Skew of a browser's synthetic oblique (`SkFont::setSkewX(-1/4)`).
const OBLIQUE_SKEW: f32 = -0.25;

pub struct Fonts {
    collection: FontCollection,
}

impl Fonts {
    pub fn load(dir: &Path) -> Result<Self> {
        let manager = FontMgr::new();
        let mut provider = TypefaceFontProvider::new();
        for family in Family::ALL {
            let path = dir.join(family.file());
            let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
            let face = manager.new_from_data(&bytes, None).with_context(|| format!("decoding {}", path.display()))?;
            provider.register_typeface(face, family.name());
        }
        let mut collection = FontCollection::new();
        collection.set_asset_font_manager(Some(provider.into()));
        Ok(Self { collection })
    }

    fn text_style(&self, s: &Style, kerning: bool, outline: bool) -> TextStyle {
        let axis = |tag: &[u8; 4], value: f32| Coordinate {
            axis: FourByteTag::from_chars(tag[0] as char, tag[1] as char, tag[2] as char, tag[3] as char),
            value,
        };
        let mut axes = Vec::new();
        if s.family != Family::Plex {
            axes.push(axis(b"wght", s.weight));
        }
        if s.family == Family::Fraunces {
            // CSS `font-optical-sizing: auto` sets opsz to the font size.
            axes.push(axis(b"opsz", s.opsz.unwrap_or(s.size.clamp(9.0, 144.0))));
            axes.push(axis(b"SOFT", s.soft.unwrap_or(0.0)));
        }
        let arguments = FontArguments::new().set_variation_design_position(VariationPosition { coordinates: &axes });
        let mut t = TextStyle::new();
        t.set_font_families(&[s.family.name()]).set_font_size(s.size).set_color(s.color).set_letter_spacing(s.letter_spacing);
        t.set_font_arguments(&arguments);
        if s.tabular {
            t.add_font_feature("tnum", 1);
        }
        if !kerning {
            t.add_font_feature("kern", 0);
        }
        if let Some(h) = s.line_height {
            t.set_height(h).set_height_override(true).set_half_leading(true);
        }
        match (outline, s.halo, s.shadow) {
            (true, Some(halo), _) => {
                let mut paint = Paint::default();
                paint.set_style(PaintStyle::Stroke).set_stroke_width(halo.width).set_stroke_join(PaintJoin::Round).set_anti_alias(true).set_color(halo.color);
                t.set_foreground_paint(&paint);
            }
            (false, _, Some(sh)) => {
                t.add_shadow(TextShadow::new(sh.color, Point::new(sh.dx, sh.dy), f64::from(sh.blur) / 2.0));
            }
            _ => {}
        }
        t
    }

    fn build(&self, runs: &[(&str, &Style)], width: f32, kerning: bool, outline: bool) -> Paragraph {
        let mut paragraph_style = ParagraphStyle::new();
        if runs[0].1.align == Align::Right {
            paragraph_style.set_text_align(TextAlign::Right);
        }
        let mut builder = ParagraphBuilder::new(&paragraph_style, &self.collection);
        for (text, style) in runs {
            builder.push_style(&self.text_style(style, kerning, outline)).add_text(*text);
        }
        let mut paragraph = builder.build();
        paragraph.layout(width);
        paragraph
    }

    fn paragraph(&self, text: &str, style: &Style, width: f32, kerning: bool) -> Paragraph {
        self.build(&[(text, style)], width, kerning, false)
    }

    /// Width of `text` on a single line, as a CSS `max-content` box.
    pub fn measure(&self, text: &str, style: &Style) -> f32 {
        self.paragraph(text, style, UNBOUNDED, true).longest_line()
    }

    /// Width of a CSS shrink-to-fit box around `text`: its single-line width, up to `available`.
    /// Rounded up so the box never wraps the line it was measured from.
    pub fn shrink_to_fit(&self, text: &str, style: &Style, available: f32) -> f32 {
        self.measure(text, style).ceil().min(available)
    }

    /// Several styled runs on one line (an SVG `<text>` with a `<tspan>`).
    pub fn typeset_inline(&self, runs: &[(&str, Style)]) -> Result<TextBlock> {
        let refs: Vec<(&str, &Style)> = runs.iter().map(|(t, s)| (*t, s)).collect();
        let first = &runs.first().context("empty text")?.1;
        let whole = self.build(&refs, UNBOUNDED, true, false);
        let (top, line_height, ascent) = {
            let metrics = whole.get_line_metrics();
            let m = metrics.first().context("empty text block")?;
            ((m.baseline - m.ascent) as f32, m.height as f32, m.ascent as f32)
        };
        let (height, longest) = (whole.height(), whole.longest_line());
        let line = Line {
            under: first.halo.map(|_| self.build(&refs, UNBOUNDED, true, true)),
            paragraph: whole,
            x: 0.0,
            top,
            height: line_height,
            baseline: ascent,
            chars: Vec::new(),
        };
        Ok(TextBlock { text: runs.iter().map(|(t, _)| *t).collect(), lines: vec![line], width: longest, height, size: first.size, longest, italic: first.italic })
    }

    /// Lay `text` out in a box `width` wide, wrapping like CSS.
    pub fn typeset(&self, text: &str, style: &Style, width: f32, per_char: bool) -> Result<TextBlock> {
        // SplitText wraps every character in its own box, which cuts kerning between them.
        let kerning = !per_char;
        let whole = self.paragraph(text, style, width, kerning);
        // Each line is drawn on its own, placed by the alignment offset `whole` computed for it.
        let placed = Style { align: Align::Left, ..*style };
        let utf16: Vec<u16> = text.encode_utf16().collect();
        let mut lines = Vec::new();
        for m in whole.get_line_metrics() {
            let slice = String::from_utf16(&utf16[m.start_index..m.end_excluding_whitespaces]).context("line text")?;
            let top = (m.baseline - m.ascent) as f32;
            let chars = if per_char {
                let mut chars = Vec::new();
                for (i, ch) in slice.chars().enumerate().filter(|(_, c)| !c.is_whitespace()) {
                    let start = m.start_index + slice.chars().take(i).map(char::len_utf16).sum::<usize>();
                    let rect = whole
                        .get_rects_for_range(start..start + ch.len_utf16(), RectHeightStyle::Max, RectWidthStyle::Tight)
                        .first()
                        .context("glyph rect")?
                        .rect;
                    chars.push(Glyph { paragraph: self.paragraph(&ch.to_string(), &placed, UNBOUNDED, false), x: rect.left - style.letter_spacing });
                }
                chars
            } else {
                Vec::new()
            };
            lines.push(Line {
                paragraph: self.paragraph(&slice, &placed, UNBOUNDED, kerning),
                under: style.halo.map(|_| self.build(&[(slice.as_str(), &placed)], UNBOUNDED, kerning, true)),
                x: m.left as f32 - style.letter_spacing,
                top,
                height: m.height as f32,
                baseline: m.ascent as f32,
                chars,
            });
        }
        if lines.is_empty() {
            bail!("empty text block");
        }
        Ok(TextBlock { text: text.to_owned(), lines, width, height: whole.height(), size: style.size, longest: whole.longest_line(), italic: style.italic })
    }
}

struct Glyph {
    paragraph: Paragraph,
    x: f32,
}

struct Line {
    paragraph: Paragraph,
    /// The outline painted beneath the fill, when the style has a halo.
    under: Option<Paragraph>,
    x: f32,
    top: f32,
    height: f32,
    /// Distance from the paragraph's top to its baseline.
    baseline: f32,
    chars: Vec<Glyph>,
}

pub struct TextBlock {
    text: String,
    lines: Vec<Line>,
    width: f32,
    pub height: f32,
    size: f32,
    longest: f32,
    italic: bool,
}

impl TextBlock {
    /// The text as written.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Width of the widest line (what a shrink-to-fit CSS box would take).
    pub fn content_width(&self) -> f32 {
        self.longest
    }

    /// Distance from the block's top to the first line's baseline.
    pub fn baseline(&self) -> f32 {
        self.lines[0].top + self.lines[0].baseline
    }

    pub fn draw(&self, canvas: &Canvas, at: Point) {
        for line in &self.lines {
            self.report(canvas, line, Point::new(at.x + line.x, at.y + line.top));
            self.paint(canvas, line, line.under.iter().chain([&line.paragraph]), Point::new(at.x + line.x, at.y + line.top));
        }
    }

    /// Draw the first line with its baseline at `baseline_y`, starting at `x`.
    pub fn draw_on_baseline(&self, canvas: &Canvas, x: f32, baseline_y: f32) {
        self.draw(canvas, Point::new(x, baseline_y - self.baseline()));
    }

    /// Each line rises through its own clip box (SplitText `mask: 'lines'`); `slide(i)` is
    /// line `i`'s remaining offset in percent of its height.
    pub fn draw_lines(&self, canvas: &Canvas, at: Point, slide: impl Fn(usize) -> f32) {
        for (i, line) in self.lines.iter().enumerate() {
            let dy = slide(i) / 100.0 * line.height;
            self.report(canvas, line, Point::new(at.x + line.x, at.y + line.top));
            self.in_mask(canvas, at, line, |c| {
                self.paint(c, line, [&line.paragraph], Point::new(at.x + line.x, at.y + line.top + dy));
            });
        }
    }

    /// As `draw_lines`, but every non-space character slides on its own; `slide(n)` is for
    /// the `n`th character across the block.
    pub fn draw_chars(&self, canvas: &Canvas, at: Point, slide: impl Fn(usize) -> f32) {
        let mut n = 0;
        for line in &self.lines {
            self.report(canvas, line, Point::new(at.x + line.x, at.y + line.top));
            self.in_mask(canvas, at, line, |c| {
                for glyph in &line.chars {
                    let dy = slide(n) / 100.0 * line.height;
                    self.paint(c, line, [&glyph.paragraph], Point::new(at.x + glyph.x, at.y + line.top + dy));
                    n += 1;
                }
            });
        }
    }

    /// Tell the layout QA where `line` lands, its top-left at `origin` in the canvas's current space.
    fn report(&self, canvas: &Canvas, line: &Line, origin: Point) {
        let local = Rect::from_xywh(origin.x, origin.y, line.paragraph.longest_line(), line.height);
        probe::text(&self.text, canvas.local_to_device_as_3x3().map_rect(local).0);
    }

    /// Paint paragraphs with their top-left at `origin`, slanted about the baseline when italic.
    fn paint<'a>(&self, canvas: &Canvas, line: &Line, paragraphs: impl IntoIterator<Item = &'a Paragraph>, origin: Point) {
        canvas.save();
        if self.italic {
            canvas.translate((origin.x, origin.y + line.baseline));
            canvas.skew((OBLIQUE_SKEW, 0.0));
            canvas.translate((0.0, -line.baseline));
        } else {
            canvas.translate((origin.x, origin.y));
        }
        for paragraph in paragraphs {
            paragraph.paint(canvas, (0.0, 0.0));
        }
        canvas.restore();
    }

    /// Clip to the line box, padded so descenders and italic overhangs survive (`padMasks`).
    fn in_mask(&self, canvas: &Canvas, at: Point, line: &Line, draw: impl FnOnce(&Canvas)) {
        let em = self.size;
        canvas.save();
        canvas.clip_rect(
            Rect::new(at.x - 0.14 * em, at.y + line.top - 0.06 * em, at.x + self.width + 0.14 * em, at.y + line.top + line.height + 0.26 * em),
            None,
            false,
        );
        draw(canvas);
        canvas.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skia_safe::{surfaces, AlphaType, ColorType, ImageInfo};

    fn fonts() -> Fonts {
        Fonts::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/fonts")).expect("bundled fonts")
    }

    fn raster(draw: impl Fn(&Canvas)) -> Vec<u8> {
        let info = ImageInfo::new((900, 300), ColorType::RGBA8888, AlphaType::Premul, None);
        let mut surface = surfaces::raster(&info, None, None).expect("raster surface");
        surface.canvas().clear(Color::BLACK);
        draw(surface.canvas());
        let mut pixels = vec![0u8; info.compute_min_byte_size()];
        assert!(surface.read_pixels(&info, &mut pixels, info.min_row_bytes(), (0, 0)));
        pixels
    }

    #[test]
    fn per_char_layout_reproduces_the_whole_line() {
        let fonts = fonts();
        let style = Style { letter_spacing: -6.0, line_height: Some(0.95), soft: Some(50.0), ..Style::new(Family::Fraunces, 220.0, 600.0, Color::WHITE) };
        let block = fonts.typeset("Water", &style, 900.0, true).expect("typeset");
        let whole = raster(|c| block.draw(c, Point::new(20.0, 20.0)));
        let split = raster(|c| block.draw_chars(c, Point::new(20.0, 20.0), |_| 0.0));
        let off = whole.iter().zip(&split).filter(|(a, b)| a.abs_diff(**b) > 24).count();
        assert!(off < 400, "{off} pixels differ");
    }
}
