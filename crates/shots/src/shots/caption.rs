//! The corner captions shared by map and diagram boards: a tag top-left, a legend bottom-right.

use skia_safe::{Canvas, Color, Paint, Point, Rect};

use crate::{palette::TERRACOTTA, text::TextBlock};

pub const MARGIN: f32 = 72.0;
/// The legend's colour on a light board.
pub const LEGEND_INK: Color = Color::from_rgb(0x8C, 0x7A, 0x60);

const TAG_TOP: f32 = 60.0;
const MARKER: f32 = 12.0;
const MARKER_GAP: f32 = 14.0;

/// A terracotta square and `tag`, centred on one row.
pub fn draw_tag(canvas: &Canvas, tag: &TextBlock) {
    let row = tag.height.max(MARKER);
    let mut square = Paint::default();
    square.set_color(TERRACOTTA);
    canvas.draw_rect(Rect::from_xywh(MARGIN, TAG_TOP + (row - MARKER) / 2.0, MARKER, MARKER), &square);
    tag.draw(canvas, Point::new(MARGIN + MARKER + MARKER_GAP, TAG_TOP + (row - tag.height) / 2.0));
}

/// `legend` right-aligned to the margin with its bottom `bottom` pixels above the frame's.
pub fn draw_legend(canvas: &Canvas, legend: &TextBlock, size: (f32, f32), bottom: f32) {
    legend.draw(canvas, Point::new(size.0 - MARGIN - legend.content_width(), size.1 - bottom - legend.height));
}
