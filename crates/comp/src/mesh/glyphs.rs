//! Glyph outlines as flat polygons. The text layer's own layout supplies the shaping (positions, fallback faces), so a
//! `text3d` line breaks and kerns exactly like a `text` line; we only ask each glyph's face for its outline.

use skia_safe::{path::Verb, Path, Point};

use crate::text::layout::Layout;

/// A closed polygon in layout space (x right, y down), without the repeated closing point.
pub type Contour = Vec<[f32; 2]>;

/// Segments per curve. Ten keep a quarter circle of radius 100 px within 0.3 px of the true arc.
const STEPS: usize = 10;

/// The outline of every glyph in `layout`, one `Vec<Contour>` per glyph (spaces and missing glyphs have none).
pub fn outlines(layout: &Layout) -> Vec<Vec<Contour>> {
    layout
        .glyphs
        .iter()
        .filter_map(|g| {
            let id = g.font.unichar_to_glyph(g.ch as i32);
            let path = g.font.get_path(id).filter(|_| id != 0)?;
            let contours = flatten(&path, g.x, g.y);
            (!contours.is_empty()).then_some(contours)
        })
        .collect()
}

/// Turn a Skia path into polygons at offset `(dx, dy)`; quads, conics and cubics are sampled at `STEPS` points.
pub fn flatten(path: &Path, dx: f32, dy: f32) -> Vec<Contour> {
    let at = |p: Point| [p.x + dx, p.y + dy];
    let mut out: Vec<Contour> = vec![];
    let mut cur: Contour = vec![];
    let mut it = skia_safe::path::Iter::new(path, false);
    while let Some((verb, pts)) = it.next() {
        match verb {
            Verb::Move => {
                finish(&mut out, std::mem::take(&mut cur));
                cur.push(at(pts[0]));
            }
            Verb::Line => cur.push(at(pts[1])),
            Verb::Quad => sample(&mut cur, |t| quad(&pts, t, 1.0), at),
            Verb::Conic => {
                let w = it.conic_weight().unwrap_or(1.0);
                sample(&mut cur, |t| quad(&pts, t, w), at);
            }
            Verb::Cubic => sample(&mut cur, |t| cubic(&pts, t), at),
            Verb::Close | Verb::Done => finish(&mut out, std::mem::take(&mut cur)),
        }
    }
    finish(&mut out, cur);
    out
}

fn sample(cur: &mut Contour, f: impl Fn(f32) -> Point, at: impl Fn(Point) -> [f32; 2]) {
    for i in 1..=STEPS {
        cur.push(at(f(i as f32 / STEPS as f32)));
    }
}

/// Rational quadratic Bezier (a conic when `w != 1`).
fn quad(p: &[Point], t: f32, w: f32) -> Point {
    let (a, b, c) = ((1.0 - t) * (1.0 - t), 2.0 * w * t * (1.0 - t), t * t);
    let d = a + b + c;
    Point::new((p[0].x * a + p[1].x * b * 1.0 + p[2].x * c) / d, (p[0].y * a + p[1].y * b + p[2].y * c) / d)
}

fn cubic(p: &[Point], t: f32) -> Point {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    Point::new(p[0].x * a + p[1].x * b + p[2].x * c + p[3].x * d, p[0].y * a + p[1].y * b + p[2].y * c + p[3].y * d)
}

/// Keep a contour once it has an area: drop repeated points and the closing duplicate.
fn finish(out: &mut Vec<Contour>, mut c: Contour) {
    c.dedup_by(|a, b| (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3);
    while c.len() > 1 && (c[0][0] - c[c.len() - 1][0]).abs() < 1e-3 && (c[0][1] - c[c.len() - 1][1]).abs() < 1e-3 {
        c.pop();
    }
    if c.len() >= 3 {
        out.push(c);
    }
}
