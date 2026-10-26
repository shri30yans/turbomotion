//! Painting particle sprites: one soft-edged sprite for glows, vector shapes for the rest.

use anyhow::Result;
use skia_safe::{color_filters, gradient_shader, BlendMode, Canvas, Color, Color4f, Image, Matrix, Paint, PaintCap, PaintStyle, Path, Point, Rect, TileMode};

use crate::{
    eval::particles::{Particle, ParticleState},
    plan::geometry::PathGeo,
    schema::ParticleShape,
};

/// A white sprite with a smooth radial falloff, tinted per particle.
pub fn soft_sprite() -> Result<Image> {
    let n = 128;
    let mut s = skia_safe::surfaces::raster_n32_premul((n, n)).ok_or_else(|| anyhow::anyhow!("no raster surface for the particle sprite"))?;
    let mut p = Paint::default();
    let c = f32::from(n as i16) / 2.0;
    let colors = [Color4f::new(1.0, 1.0, 1.0, 1.0), Color4f::new(1.0, 1.0, 1.0, 0.55), Color4f::new(1.0, 1.0, 1.0, 0.0)];
    p.set_shader(gradient_shader::radial(Point::new(c, c), c, &colors[..], Some(&[0.0f32, 0.45, 1.0][..]), TileMode::Clamp, None, None));
    s.canvas().draw_rect(Rect::from_wh(n as f32, n as f32), &p);
    Ok(s.image_snapshot())
}

fn tint(p: &Particle) -> (Color, f32) {
    (Color::from_rgb(p.color[0].clamp(0.0, 255.0) as u8, p.color[1].clamp(0.0, 255.0) as u8, p.color[2].clamp(0.0, 255.0) as u8), p.color[3].clamp(0.0, 1.0))
}

fn star(r: f32) -> Path {
    let mut path = Path::new();
    for i in 0..10 {
        let a = std::f32::consts::PI * i as f32 / 5.0 - std::f32::consts::FRAC_PI_2;
        let rr = if i % 2 == 0 { r } else { r * 0.45 };
        let pt = (a.cos() * rr, a.sin() * rr);
        if i == 0 { path.move_to(pt); } else { path.line_to(pt); }
    }
    path.close();
    path
}

/// The sprite outline fitted to a unit diameter and centred, so a particle's size is its width.
fn unit_outline(path: &Path) -> Path {
    let b = path.bounds();
    let span = b.width().max(b.height()).max(1e-6);
    let mut to_unit = Matrix::scale((1.0 / span, 1.0 / span));
    to_unit.pre_translate((-b.center_x(), -b.center_y()));
    path.with_transform(&to_unit)
}

/// `sprite` is the layer's own outline when its shape is a `path`.
pub fn paint(c: &Canvas, sprite: &Image, outline: Option<&PathGeo>, st: &ParticleState) -> Result<()> {
    let outline = outline.map(|g| unit_outline(&g.path));
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    for p in &st.items {
        let r = (p.size / 2.0) as f32;
        let (color, alpha) = tint(p);
        if r <= 0.05 || alpha <= 0.002 {
            continue;
        }
        paint.set_style(PaintStyle::Fill).set_color(color).set_alpha_f(alpha).set_color_filter(None);
        let at = (p.x as f32, p.y as f32);
        c.save();
        c.translate(at);
        match &st.shape {
            ParticleShape::Soft => {
                paint.set_color_filter(color_filters::blend(color, BlendMode::SrcIn));
                paint.set_color(Color::WHITE).set_alpha_f(alpha);
                c.draw_image_rect(sprite, None, Rect::from_xywh(-r * 2.0, -r * 2.0, r * 4.0, r * 4.0), &paint);
            }
            ParticleShape::Circle => {
                c.draw_circle((0.0, 0.0), r, &paint);
            }
            ParticleShape::Square => {
                c.rotate(p.rotation as f32, None);
                c.draw_rect(Rect::from_xywh(-r, -r, r * 2.0, r * 2.0), &paint);
            }
            ParticleShape::Star => {
                c.rotate(p.rotation as f32, None);
                c.draw_path(&star(r), &paint);
            }
            ParticleShape::Ring => {
                paint.set_style(PaintStyle::Stroke).set_stroke_width((r * 0.25).max(0.75));
                c.draw_circle((0.0, 0.0), r, &paint);
            }
            ParticleShape::Spark => {
                // a streak along the direction of travel, longer when faster
                let v = (p.vx * p.vx + p.vy * p.vy).sqrt().max(1e-6);
                let len = (r * 2.0 + (v as f32) * 0.04).max(r * 2.0);
                paint.set_style(PaintStyle::Stroke).set_stroke_width((r * 0.7).max(0.75)).set_stroke_cap(PaintCap::Round);
                c.draw_line((0.0, 0.0), (-(p.vx / v) as f32 * len, -(p.vy / v) as f32 * len), &paint);
            }
            ParticleShape::Path(_) => {
                c.rotate(p.rotation as f32, None);
                c.scale((r * 2.0, r * 2.0));
                if let Some(path) = &outline {
                    c.draw_path(path, &paint);
                }
            }
        }
        c.restore();
    }
    Ok(())
}
