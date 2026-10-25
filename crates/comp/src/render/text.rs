//! Painting text: each character at its rest position through its own transform, opacity, blur and colour.

use anyhow::Result;
use skia_safe::{color_filters, image_filters, BlendMode, Canvas, Color4f, ColorMatrix, Matrix, Paint, PaintJoin, PaintStyle, Path, RRect, Rect, TileMode};

use super::color4f;
use crate::{eval::text::TextState, text::layout::is_color_glyph};

pub fn paint(c: &Canvas, t: &TextState) -> Result<()> {
    let Some(layout) = &t.layout else { return Ok(()) };
    if let Some(m) = &t.mark {
        if m.progress > 0.002 {
            let w = ((m.rect[2] - m.rect[0]) as f64 * m.progress).max(1.0) as f32;
            let h = (m.rect[3] - m.rect[1]).max(1.0);
            let mut p = Paint::default();
            p.set_anti_alias(true).set_color4f(color4f([m.color[0], m.color[1], m.color[2], m.color[3]]), None);
            if m.rough {
                c.draw_path(&highlighter_path(m.rect, w, h), &p);
            } else {
                let r = (h * 0.18).min(8.0);
                c.draw_rrect(RRect::new_rect_xy(Rect::from_xywh(m.rect[0], m.rect[1], w, h), r, r), &p);
            }
        }
    }
    let fill_shader = t.fill.as_ref().and_then(|f| f.gradient.as_ref()).and_then(|g| super::shape::gradient(g, t.fill.as_ref().map_or(1.0, |f| f.opacity)));
    for (g, laid) in t.glyphs.iter().zip(&layout.glyphs) {
        if g.opacity <= 0.002 || g.m.0.iter().any(|v| !v.is_finite()) {
            continue;
        }
        c.save();
        if let Some([l, tp, r, b]) = g.clip {
            c.clip_rect(Rect::new(l, tp, r, b), None, Some(true));
        }
        let [a, b, cc, d, e, f] = g.m.0;
        c.concat(&Matrix::new_all(a, cc, e, b, d, f, 0.0, 0.0, 1.0));
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        if g.blur > 0.3 {
            paint.set_image_filter(image_filters::blur((g.blur as f32, g.blur as f32), TileMode::Decal, None, None));
        }
        let s = g.ch.to_string();
        let font = &laid.font;
        let col = match g.color {
            Some(rgb) => [rgb[0] as f32, rgb[1] as f32, rgb[2] as f32, t.color[3]],
            None => t.fill.as_ref().and_then(|f| f.color).unwrap_or(t.color),
        };
        let fill = color4f([col[0], col[1], col[2], col[3] * g.opacity as f32]);
        if is_color_glyph(font, g.ch) {
            // Bitmap glyphs have no outline. Draw them as ink in the same fill as the rest of the line.
            paint.set_color_filter(ink_in_fill(fill));
            if let Some(sh) = &fill_shader {
                paint.set_shader(sh.clone()).set_alpha_f(g.opacity as f32);
            } else {
                paint.set_color4f(fill, None);
            }
            c.draw_str(&s, (g.x, g.y), font, &paint);
            c.restore();
            continue;
        }
        if let Some((sc, w)) = &t.stroke {
            let mut sp = paint.clone();
            sp.set_style(PaintStyle::Stroke).set_stroke_width(*w as f32).set_stroke_join(PaintJoin::Round).set_color4f(color4f([sc[0], sc[1], sc[2], sc[3] * g.opacity as f32]), None);
            c.draw_str(&s, (g.x, g.y), font, &sp);
        }
        if let Some(sh) = &fill_shader {
            paint.set_shader(sh.clone()).set_alpha_f(g.opacity as f32);
        } else {
            paint.set_color4f(fill, None);
        }
        c.draw_str(&s, (g.x, g.y), font, &paint);
        c.restore();
    }
    Ok(())
}

/// Turn a colour bitmap glyph into coverage and recolour it with `fill`, so it matches the line.
fn ink_in_fill(fill: Color4f) -> skia_safe::ColorFilter {
    let ink = color_filters::matrix(
        &ColorMatrix::new(0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, -0.2126, -0.7152, -0.0722, 0.0, 1.0),
        None,
    );
    color_filters::compose(color_filters::blend_with_color_space(fill, None, BlendMode::SrcIn).expect("src-in"), ink).expect("compose")
}

/// A highlighter stroke with a ragged top and bottom, like ink on paper.
fn highlighter_path(rect: [f32; 4], w: f32, h: f32) -> Path {
    let n = 14;
    let jig = h * 0.22;
    let hash = |i: i32| {
        let x = i.wrapping_mul(1103515245).wrapping_add(12345) >> 16;
        ((x & 32767) as f32 / 32767.0 - 0.5) * jig
    };
    let mut path = Path::new();
    path.move_to((rect[0], rect[1] + hash(1)));
    for i in 1..=n {
        let t = i as f32 / n as f32;
        path.line_to((rect[0] + w * t, rect[1] + hash(i)));
    }
    for i in (0..=n).rev() {
        let t = i as f32 / n as f32;
        path.line_to((rect[0] + w * t, rect[3] + hash(100 + i)));
    }
    path.close();
    path
}
