//! Painting text: each character at its rest position through its own transform, opacity, blur and colour.

use anyhow::Result;
use skia_safe::{image_filters, Canvas, Matrix, Paint, PaintJoin, PaintStyle, Rect, TileMode};

use super::color4f;
use crate::eval::text::TextState;

pub fn paint(c: &Canvas, t: &TextState) -> Result<()> {
    let Some(layout) = &t.layout else { return Ok(()) };
    let font = &layout.font;
    for g in &t.glyphs {
        if g.opacity <= 0.002 || g.m.0.iter().any(|v| !v.is_finite()) {
            continue;
        }
        c.save();
        if let Some([l, tp, r, b]) = g.clip {
            c.clip_rect(Rect::new(l, tp, r, b), None, Some(true));
        }
        let [a, b, cc, d, e, f] = g.m.0;
        c.concat(&Matrix::new_all(a, cc, e, b, d, f, 0.0, 0.0, 1.0));
        let col = match g.color {
            Some(rgb) => [rgb[0] as f32, rgb[1] as f32, rgb[2] as f32, t.color[3]],
            None => t.color,
        };
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        if g.blur > 0.3 {
            paint.set_image_filter(image_filters::blur((g.blur as f32, g.blur as f32), TileMode::Decal, None, None));
        }
        let s = g.ch.to_string();
        if let Some((sc, w)) = &t.stroke {
            let mut sp = paint.clone();
            sp.set_style(PaintStyle::Stroke).set_stroke_width(*w as f32).set_stroke_join(PaintJoin::Round).set_color4f(color4f([sc[0], sc[1], sc[2], sc[3] * g.opacity as f32]), None);
            c.draw_str(&s, (g.x, g.y), font, &sp);
        }
        paint.set_color4f(color4f([col[0], col[1], col[2], col[3] * g.opacity as f32]), None);
        c.draw_str(&s, (g.x, g.y), font, &paint);
        c.restore();
    }
    Ok(())
}
