//! Painting shape items: fills (flat or gradient), strokes with dashes, trim, morph and repeaters.

use anyhow::Result;
use skia_safe::{
    gradient_shader::GradientShaderColors, Canvas, Color4f, ContourMeasureIter, Matrix, Paint, PaintCap, PaintJoin, PaintStyle, Path, PathEffect, PathFillType, Point, TileMode,
};
use turbomotion_core::color::Ink;

use super::color4f;
use crate::{
    eval::{shape::{item_matrix, FillState, GradientState, StrokeState}, ShapeState},
    plan::geometry::PathGeo,
};

pub fn paint(ink: &Ink, c: &Canvas, items: &[ShapeState], geos: &[PathGeo]) -> Result<()> {
    for (s, geo) in items.iter().zip(geos) {
        let mut path = geo.at(s.morph);
        if let Some([start, end, offset]) = s.trim {
            path = trimmed(&path, start, end, offset);
        }
        let copies = s.repeat.as_ref().map_or(1, |r| r.copies);
        let mut m = item_matrix(s);
        let mut opacity = s.opacity;
        for _ in 0..copies {
            c.save();
            c.concat(&m);
            if let Some(f) = &s.fill {
                fill(ink, c, &path, f, opacity, geo.morph_to.is_none() && s.trim.is_none())?;
            }
            if let Some(st) = &s.stroke {
                stroke(ink, c, &path, st, opacity)?;
            }
            c.restore();
            if let Some(r) = &s.repeat {
                let mut step = Matrix::translate((r.offset[0] as f32, r.offset[1] as f32));
                step.pre_rotate(r.rotation as f32, None);
                step.pre_scale((r.scale as f32, r.scale as f32), None);
                m = Matrix::concat(&m, &step);
                opacity *= r.opacity;
            }
        }
    }
    Ok(())
}

pub(super) fn gradient(g: &GradientState, opacity: f64) -> Option<skia_safe::Shader> {
    let colors: Vec<Color4f> = g.stops.iter().map(|(_, col)| color4f([col[0], col[1], col[2], col[3] * opacity as f32])).collect();
    let pos: Vec<f32> = g.stops.iter().map(|(p, _)| *p as f32).collect();
    if g.radial {
        skia_safe::gradient_shader::radial(Point::new(g.p0[0] as f32, g.p0[1] as f32), g.radius as f32, GradientShaderColors::ColorsInSpace(&colors, None), &pos[..], TileMode::Clamp, None, None)
    } else {
        skia_safe::gradient_shader::linear((Point::new(g.p0[0] as f32, g.p0[1] as f32), Point::new(g.p1[0] as f32, g.p1[1] as f32)), GradientShaderColors::ColorsInSpace(&colors, None), &pos[..], TileMode::Clamp, None, None)
    }
}

fn fill(ink: &Ink, c: &Canvas, path: &Path, f: &FillState, opacity: f64, _plain: bool) -> Result<()> {
    let mut p = Paint::default();
    p.set_anti_alias(true).set_style(PaintStyle::Fill);
    let a = (f.opacity * opacity) as f32;
    if let Some(g) = &f.gradient {
        p.set_shader(gradient(g, f.opacity * opacity));
    } else if let Some(col) = f.color {
        ink.apply(&mut p, c, color4f([col[0], col[1], col[2], col[3] * a]))?;
    } else {
        return Ok(());
    }
    let mut path = path.clone();
    path.set_fill_type(if f.even_odd { PathFillType::EvenOdd } else { PathFillType::Winding });
    c.draw_path(&path, &p);
    Ok(())
}

fn stroke(ink: &Ink, c: &Canvas, path: &Path, s: &StrokeState, opacity: f64) -> Result<()> {
    if s.width <= 0.0 {
        return Ok(());
    }
    let mut p = Paint::default();
    p.set_anti_alias(true).set_style(PaintStyle::Stroke).set_stroke_width(s.width as f32);
    ink.apply(&mut p, c, color4f([s.color[0], s.color[1], s.color[2], s.color[3] * (s.opacity * opacity) as f32]))?;
    p.set_stroke_cap(match s.cap.as_str() {
        "round" => PaintCap::Round,
        "square" => PaintCap::Square,
        _ => PaintCap::Butt,
    });
    p.set_stroke_join(match s.join.as_str() {
        "round" => PaintJoin::Round,
        "bevel" => PaintJoin::Bevel,
        _ => PaintJoin::Miter,
    });
    if let Some(d) = &s.dash {
        let d: Vec<f32> = d.iter().map(|v| *v as f32).collect();
        if d.len() >= 2 && d.len() % 2 == 0 {
            p.set_path_effect(PathEffect::dash(&d, s.dash_offset as f32));
        }
    }
    c.draw_path(path, &p);
    Ok(())
}

/// The part of every contour between fractions `start` and `end` of the whole outline, shifted by `offset` (wrapping).
pub fn trimmed(path: &Path, start: f64, end: f64, offset: f64) -> Path {
    let contours: Vec<_> = ContourMeasureIter::new(path, false, None).collect();
    let total: f32 = contours.iter().map(|c| c.length()).sum();
    if total <= 0.0 {
        return Path::new();
    }
    let (mut a, mut b) = (start + offset, end + offset);
    if (b - a) >= 1.0 {
        return path.clone();
    }
    a = a.rem_euclid(1.0);
    b = a + (end - start);
    let mut out = Path::new();
    let mut segment = |from: f64, to: f64| {
        let (from, to) = ((from as f32) * total, (to as f32) * total);
        let mut base = 0.0;
        for c in &contours {
            let len = c.length();
            let (lo, hi) = ((from - base).max(0.0), (to - base).min(len));
            if hi > lo {
                if let Some(seg) = c.segment(lo, hi, true) {
                    out.add_path(&seg, (0.0, 0.0), None);
                }
            }
            base += len;
        }
    };
    if b <= 1.0 {
        segment(a, b);
    } else {
        segment(a, 1.0);
        segment(0.0, b - 1.0);
    }
    out
}
