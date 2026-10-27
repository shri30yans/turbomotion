//! Painting text: each character at its rest position through its own transform, opacity, blur and colour.

use anyhow::Result;
use skia_safe::{color_filters, image_filters, BlendMode, Canvas, Color4f, ColorMatrix, ContourMeasureIter, Font, Matrix, Paint, PaintJoin, PaintStyle, Path, RRect, Rect, TileMode};
use turbomotion_core::color::Ink;

use super::color4f;
use crate::{
    eval::{mat4::Mat4, text::{bounds::BoxState, TextState}},
    text::{
        animate::shift_hsv,
        layout::is_color_glyph,
        place::{corners, MIN_STROKE_WIDTH, MIN_VISIBLE_OPACITY},
    },
};

pub fn paint(ink: &Ink, c: &Canvas, t: &TextState) -> Result<()> {
    let Some(layout) = &t.layout else { return Ok(()) };
    if let Some(b) = &t.text_box {
        paint_box(ink, c, b)?;
    }
    if let Some(m) = &t.mark {
        if m.progress > 0.002 {
            let w = ((m.rect[2] - m.rect[0]) as f64 * m.progress).max(1.0) as f32;
            let h = (m.rect[3] - m.rect[1]).max(1.0);
            let mut p = Paint::default();
            p.set_anti_alias(true);
            ink.apply(&mut p, c, color4f(m.color))?;
            if m.rough {
                c.draw_path(&highlighter_path(m.rect, w, h), &p);
            } else {
                let r = (h * 0.18).min(8.0);
                c.draw_rrect(RRect::new_rect_xy(Rect::from_xywh(m.rect[0], m.rect[1], w, h), r, r), &p);
            }
        }
    }
    let fill_shader = t.fill.as_ref().and_then(|f| f.gradient.as_ref()).and_then(|g| super::shape::gradient(g, t.fill.as_ref().map_or(1.0, |f| f.opacity)));
    // outline length of every glyph, for a trim that runs along the whole line
    let trim_lengths: Option<Vec<f32>> = t.trim.filter(|_| !t.trim_each).map(|_| layout.glyphs.iter().map(|lg| outline(&lg.font, lg.ch).map_or(0.0, |p| outline_length(&p))).collect());
    let trim_total: f32 = trim_lengths.as_ref().map_or(0.0, |v| v.iter().sum());
    for g in &t.glyphs {
        let laid = &layout.glyphs[g.src];
        if g.opacity <= MIN_VISIBLE_OPACITY || g.m.0.iter().any(|v| !v.is_finite()) || g.m4.is_some_and(|m| m.iter().any(|v| !v.is_finite())) {
            continue;
        }
        if g.m4.is_some() && corners(g, [laid.x, laid.top, laid.x + laid.w, laid.bottom]).is_none() {
            // a corner is behind the viewer: the glyph would smear across the frame
            continue;
        }
        c.save();
        if let Some([l, tp, r, b]) = g.clip {
            c.clip_rect(Rect::new(l, tp, r, b), None, Some(true));
        }
        match g.m4 {
            Some(m) => {
                c.concat_44(&Mat4(m).to_m44());
            }
            None => {
                let [a, b, cc, d, e, f] = g.m.0;
                c.concat(&Matrix::new_all(a, cc, e, b, d, f, 0.0, 0.0, 1.0));
            }
        }
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        if g.blur > 0.3 {
            paint.set_image_filter(image_filters::blur((g.blur as f32, g.blur as f32), TileMode::Decal, None, None));
        }
        let s = g.ch.to_string();
        let font = &laid.font;
        let mut col = match g.color {
            Some(rgb) => [rgb[0] as f32, rgb[1] as f32, rgb[2] as f32, t.color[3]],
            None => t.fill.as_ref().and_then(|f| f.color).unwrap_or(t.color),
        };
        if g.hue != 0.0 || g.sat != 0.0 || g.bri != 0.0 {
            let [r, gg, b] = shift_hsv([col[0], col[1], col[2]], g.hue, g.sat, g.bri);
            col = [r, gg, b, col[3]];
        }
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
        let stroke_w = t.stroke.as_ref().map_or(0.0, |(_, w)| *w) + g.stroke_w;
        if stroke_w > MIN_STROKE_WIDTH {
            let sc = t.stroke.as_ref().map_or(col, |(sc, _)| *sc);
            let mut sp = paint.clone();
            sp.set_style(PaintStyle::Stroke).set_stroke_width(stroke_w as f32).set_stroke_join(PaintJoin::Round);
            ink.apply(&mut sp, c, color4f([sc[0], sc[1], sc[2], sc[3] * g.opacity as f32]))?;
            if let Some(window) = t.trim {
                sp.set_stroke_cap(skia_safe::PaintCap::Round);
                if let Some(mut path) = outline(font, g.ch) {
                    path.offset((g.x, g.y));
                    let own = outline_length(&path);
                    let lengths = trim_lengths.as_ref().map(|lens| (lens[..g.src].iter().sum::<f32>(), trim_total));
                    if let Some((from, to)) = trim_span(window, own, lengths) {
                        c.draw_path(&super::shape::trimmed(&path, f64::from(from), f64::from(to), 0.0), &sp);
                    }
                }
            } else {
                c.draw_str(&s, (g.x, g.y), font, &sp);
            }
        }
        if !t.outline_only {
            if let Some(sh) = &fill_shader {
                paint.set_shader(sh.clone()).set_alpha_f(g.opacity as f32);
            } else {
                ink.apply(&mut paint, c, fill)?;
            }
            c.draw_str(&s, (g.x, g.y), font, &paint);
        }
        c.restore();
    }
    Ok(())
}

/// The `box`: a rounded rectangle for each of its rects, behind everything else the layer draws. The corner radius is
/// held to half the shorter side, so a large one makes a pill.
fn paint_box(ink: &Ink, c: &Canvas, b: &BoxState) -> Result<()> {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    ink.apply(&mut paint, c, color4f(b.fill))?;
    for &[left, top, right, bottom] in &b.rects {
        let rect = Rect::new(left as f32, top as f32, right as f32, bottom as f32);
        let radius = (b.radius as f32).min(rect.width() / 2.0).min(rect.height() / 2.0);
        c.draw_rrect(RRect::new_rect_xy(rect, radius, radius), &paint);
    }
    Ok(())
}

/// The fractions of one glyph's outline (`own` long) inside the trim `window`. `along` is the length of the outlines
/// before this glyph and of the whole line when the trim runs along the line, `None` when each glyph trims its own.
fn trim_span(window: [f64; 2], own: f32, along: Option<(f32, f32)>) -> Option<(f32, f32)> {
    if own <= 0.0 {
        return None;
    }
    let [a, b] = window.map(|f| f as f32);
    let (from, to) = match along {
        Some((before, total)) => ((a * total - before) / own, (b * total - before) / own),
        None => (a, b),
    };
    let (from, to) = (from.clamp(0.0, 1.0), to.clamp(0.0, 1.0));
    (to > from).then_some((from, to))
}

/// A glyph's outline with its baseline origin at (0, 0).
fn outline(font: &Font, ch: char) -> Option<Path> {
    let id = font.unichar_to_glyph(ch as i32);
    if id == 0 {
        return None;
    }
    font.get_path(id)
}

fn outline_length(path: &Path) -> f32 {
    ContourMeasureIter::new(path, false, None).map(|m| m.length()).sum()
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
