//! The rectangles of a text `box`: the measured bounds of the visible characters, padded, and widened to a minimum.
//! Pure geometry in layout space, so every number is asserted without a renderer.

use super::bounds::{Edges, TextBounds};
use crate::schema::Align;

/// A box's evaluated numbers at one instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxFit {
    /// Pixels beyond the characters: `[sides, top and bottom]`.
    pub padding: [f64; 2],
    /// The least width of a box, padding included.
    pub min_width: f64,
    /// The side of a box wider than its text that keeps its place.
    pub align: Align,
    /// A box per visible line, instead of one round the block.
    pub per_line: bool,
}

impl BoxFit {
    /// The boxes round `bounds`, `[left, top, right, bottom]` each: one per visible line, or one for the block.
    pub fn rects(&self, bounds: &TextBounds) -> Vec<Edges> {
        if self.per_line {
            bounds.lines.iter().map(|l| self.fit(l.rect.edges())).collect()
        } else {
            vec![self.fit(bounds.block.edges())]
        }
    }

    fn fit(&self, [left, top, right, bottom]: Edges) -> Edges {
        let [x, y] = self.padding;
        let (mut left, mut right) = (left - x, right + x);
        let short = (self.min_width - (right - left)).max(0.0);
        match self.align {
            Align::Left => right += short,
            Align::Right => left -= short,
            Align::Center => {
                left -= short / 2.0;
                right += short / 2.0;
            }
        }
        [left, top - y, right, bottom + y]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::bounds::{LineRect, SourceRect};

    fn bounds() -> TextBounds {
        let (a, b) = ([10.0, 0.0, 110.0, 40.0], [30.0, 40.0, 70.0, 80.0]);
        let rect = |e| SourceRect::from_edges(e);
        let block = rect([10.0, 0.0, 110.0, 80.0]);
        TextBounds { block, extents: block, lines: vec![LineRect { line: 0, rect: rect(a) }, LineRect { line: 1, rect: rect(b) }] }
    }

    fn fit(align: Align, min_width: f64, per_line: bool) -> BoxFit {
        BoxFit { padding: [6.0, 4.0], min_width, align, per_line }
    }

    #[test]
    fn a_box_is_the_block_with_the_padding_on_every_side() {
        assert_eq!(fit(Align::Left, 0.0, false).rects(&bounds()), [[4.0, -4.0, 116.0, 84.0]]);
    }

    #[test]
    fn per_line_gives_one_box_per_line_each_as_wide_as_its_own() {
        let r = fit(Align::Left, 0.0, true).rects(&bounds());
        assert_eq!(r, [[4.0, -4.0, 116.0, 44.0], [24.0, 36.0, 76.0, 84.0]]);
    }

    #[test]
    fn a_minimum_width_widens_a_narrow_box_toward_the_side_align_names() {
        let widen = |align| fit(align, 200.0, false).rects(&bounds())[0];
        assert_eq!(widen(Align::Left), [4.0, -4.0, 204.0, 84.0], "the left edge stays");
        assert_eq!(widen(Align::Right), [-84.0, -4.0, 116.0, 84.0], "the right edge stays");
        assert_eq!(widen(Align::Center), [-40.0, -4.0, 160.0, 84.0], "both edges move, the middle stays");
        assert_eq!(fit(Align::Left, 100.0, false).rects(&bounds())[0], [4.0, -4.0, 116.0, 84.0], "a box already wider is not touched");
    }
}
