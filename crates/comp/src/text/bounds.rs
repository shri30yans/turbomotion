//! Where the text is: the box of the characters that are showing, measured from the shaped layout and what each
//! glyph is doing at one instant (After Effects' `sourceRectAtTime`). It is the one measurement every consumer reads:
//! the text `box`, `comp inspect`, the SDK's `measure()` (all characters at rest) and, later, expressions.
//!
//! A glyph counts when it is drawn: it is not transparent, it is in front of the viewer, and what is left of its
//! transformed box after the animator's clip is not empty. Its box is the line's (ascent to descent), moved by
//! whatever the animators did to it, tracking included, so the bounds follow a typed reveal, a tracking-in or a masked
//! rise exactly as the eye does. Whitespace has no glyph, so it never counts: the bounds hug the first and last
//! visible character of a line. A character with nothing showing after it on its line is as wide as it would be typed
//! last (its own advance, without the kerning toward a character that is not there), so a typed bar is the width of
//! the text typed so far.

use std::collections::BTreeMap;

use serde::Serialize;

use super::{
    animate::GlyphState,
    layout::Layout,
    place::{corners, MIN_STROKE_WIDTH, MIN_VISIBLE_OPACITY},
};

/// `[left, top, right, bottom]` in layout space (pixels from the top-left of the text block).
pub type Edges = [f64; 4];

/// A rectangle as After Effects names one: its top-left corner and its size.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SourceRect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

impl SourceRect {
    pub fn from_edges([l, t, r, b]: Edges) -> SourceRect {
        SourceRect { left: l, top: t, width: r - l, height: b - t }
    }

    pub fn edges(&self) -> Edges {
        [self.left, self.top, self.left + self.width, self.top + self.height]
    }

    fn grown(&self, by: f64) -> SourceRect {
        SourceRect { left: self.left - by, top: self.top - by, width: self.width + 2.0 * by, height: self.height + 2.0 * by }
    }
}

/// The part of the bounds on one line of the text.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct LineRect {
    pub line: usize,
    pub rect: SourceRect,
}

/// The measured bounds of the visible characters.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextBounds {
    /// Every visible character.
    pub block: SourceRect,
    /// `block` grown on every side by half the widest stroke drawn: After Effects' `includeExtents`.
    pub extents: SourceRect,
    /// The visible characters of each line that has any, in line order.
    pub lines: Vec<LineRect>,
}

impl TextBounds {
    /// The bounds of `layout`'s glyphs as `glyphs` (their state at one instant) draw them, `stroke` px being the
    /// width of the line's stroke. `None` when no character is showing.
    pub fn of(layout: &Layout, glyphs: &[GlyphState], stroke: f64) -> Option<TextBounds> {
        let mut showing = vec![false; layout.glyphs.len()];
        for g in glyphs {
            showing[g.src] = g.opacity > MIN_VISIBLE_OPACITY;
        }
        let shown: Vec<(usize, Edges, f64)> = glyphs
            .iter()
            .filter(|g| showing[g.src])
            .filter_map(|g| {
                let drawn = hull(&corners(g, rest_box(layout, g.src, &showing))?);
                let visible = g.clip.map_or(Some(drawn), |clip| intersection(drawn, clip.map(f64::from)))?;
                Some((layout.glyphs[g.src].line, visible, stroke + g.stroke_w))
            })
            .collect();
        let reach = shown.iter().map(|s| s.2).filter(|w| *w > MIN_STROKE_WIDTH).fold(0.0, f64::max) / 2.0;
        TextBounds::from_lines(by_line(shown.iter().map(|&(line, e, _)| (line, e))), reach)
    }

    /// The bounds of every character of `layout` at rest, none of them transformed.
    pub fn of_layout(layout: &Layout) -> Option<TextBounds> {
        let showing = vec![true; layout.glyphs.len()];
        let boxes = (0..layout.glyphs.len()).map(|i| (layout.glyphs[i].line, rest_box(layout, i, &showing).map(f64::from)));
        TextBounds::from_lines(by_line(boxes), 0.0)
    }

    /// `sourceRectAtTime`'s rectangle: the characters' box, or with `include_extents` grown by the stroke.
    pub fn source_rect(&self, include_extents: bool) -> SourceRect {
        if include_extents { self.extents } else { self.block }
    }

    fn from_lines(lines: BTreeMap<usize, Edges>, reach: f64) -> Option<TextBounds> {
        let block = lines.values().copied().reduce(union)?;
        let block = SourceRect::from_edges(block);
        let lines = lines.into_iter().map(|(line, e)| LineRect { line, rect: SourceRect::from_edges(e) }).collect();
        Some(TextBounds { block, extents: block.grown(reach), lines })
    }
}

/// Glyph `i`'s box at rest: its line's height, its advance wide. When a later character on its line exists but is not
/// showing, the advance is the character's own (`Glyph::trail`), so a typed prefix is as wide as the same prefix typed
/// alone. The last character of a line already has no neighbour to kern with, so its width is used as measured.
fn rest_box(layout: &Layout, i: usize, showing: &[bool]) -> [f32; 4] {
    let g = &layout.glyphs[i];
    let hidden_next = layout.glyphs.get(i + 1).is_some_and(|next| next.line == g.line && !showing[i + 1]);
    [g.x, g.top, g.x + g.w + if hidden_next { g.trail } else { 0.0 }, g.bottom]
}

/// The boxes of each line: the union of the boxes given for it.
fn by_line(boxes: impl Iterator<Item = (usize, Edges)>) -> BTreeMap<usize, Edges> {
    let mut lines: BTreeMap<usize, Edges> = BTreeMap::new();
    for (line, e) in boxes {
        lines.entry(line).and_modify(|l| *l = union(*l, e)).or_insert(e);
    }
    lines
}

/// The smallest box holding both.
pub fn union(a: Edges, b: Edges) -> Edges {
    [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]
}

/// What the boxes have in common, or `None` when they do not overlap.
fn intersection(a: Edges, b: Edges) -> Option<Edges> {
    let e = [a[0].max(b[0]), a[1].max(b[1]), a[2].min(b[2]), a[3].min(b[3])];
    (e[2] > e[0] && e[3] > e[1]).then_some(e)
}

/// The box round four points.
fn hull(points: &[[f64; 2]; 4]) -> Edges {
    let extreme = |axis: usize, start: f64, pick: fn(f64, f64) -> f64| points.iter().map(|p| p[axis]).fold(start, pick);
    [extreme(0, f64::INFINITY, f64::min), extreme(1, f64::INFINITY, f64::min), extreme(0, f64::NEG_INFINITY, f64::max), extreme(1, f64::NEG_INFINITY, f64::max)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rectangle_is_its_corner_and_size_and_grows_on_every_side() {
        let r = SourceRect::from_edges([10.0, 20.0, 110.0, 70.0]);
        assert_eq!((r.left, r.top, r.width, r.height), (10.0, 20.0, 100.0, 50.0));
        assert_eq!(r.edges(), [10.0, 20.0, 110.0, 70.0]);
        assert_eq!(r.grown(3.0).edges(), [7.0, 17.0, 113.0, 73.0]);
    }

    #[test]
    fn boxes_are_joined_cut_and_wrapped_round_points() {
        assert_eq!(union([0.0, 0.0, 4.0, 4.0], [2.0, -1.0, 9.0, 3.0]), [0.0, -1.0, 9.0, 4.0]);
        assert_eq!(intersection([0.0, 0.0, 4.0, 4.0], [2.0, -1.0, 9.0, 3.0]), Some([2.0, 0.0, 4.0, 3.0]));
        assert_eq!(intersection([0.0, 0.0, 4.0, 4.0], [4.0, 0.0, 9.0, 4.0]), None, "touching is not overlapping");
        assert_eq!(hull(&[[1.0, 5.0], [4.0, 2.0], [7.0, 6.0], [3.0, 9.0]]), [1.0, 2.0, 7.0, 9.0]);
    }

    #[test]
    fn lines_are_joined_per_line_in_line_order() {
        let lines = by_line([(1, [0.0, 10.0, 5.0, 20.0]), (0, [3.0, 0.0, 8.0, 9.0]), (1, [6.0, 11.0, 9.0, 21.0])].into_iter());
        assert_eq!(lines.into_iter().collect::<Vec<_>>(), [(0, [3.0, 0.0, 8.0, 9.0]), (1, [0.0, 10.0, 9.0, 21.0])]);
    }
}
