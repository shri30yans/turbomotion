//! Turning `PathSpec`s into Skia paths. Done once at plan time; a path that cannot be built is a plan error.

use anyhow::{anyhow, bail, Context, Result};
use skia_safe::{Path, PathDirection, Point, RRect, Rect};

use crate::schema::PathSpec;

pub struct PathGeo {
    pub path: Path,
    /// The path to morph toward (same structure), when the spec asks for it.
    pub morph_to: Option<Path>,
}

impl PathGeo {
    pub fn build(spec: &PathSpec) -> Result<PathGeo> {
        let path = if let Some(d) = &spec.d {
            Path::from_svg(d).ok_or_else(|| anyhow!("could not read the SVG path {d:?}"))?
        } else if let Some(points) = &spec.points {
            if points.len() < 2 {
                bail!("a path of points needs at least two points");
            }
            if spec.smooth { smooth(points, spec.closed) } else { Path::polygon(&points.iter().map(|p| Point::new(p[0] as f32, p[1] as f32)).collect::<Vec<_>>(), spec.closed, None, None) }
        } else if let Some([x, y, w, h]) = spec.rect {
            let r = Rect::from_xywh(x as f32, y as f32, w as f32, h as f32);
            if spec.radius > 0.0 { Path::rrect(RRect::new_rect_xy(r, spec.radius as f32, spec.radius as f32), PathDirection::CW) } else { Path::rect(r, None) }
        } else if let Some([cx, cy, rx, ry]) = spec.ellipse {
            Path::oval(Rect::from_ltrb((cx - rx) as f32, (cy - ry) as f32, (cx + rx) as f32, (cy + ry) as f32), None)
        } else if let Some([cx, cy, r, n]) = spec.polygon {
            let n = n.round().max(3.0) as usize;
            let pts: Vec<Point> = (0..n).map(|i| polar(cx, cy, r, spec.rotate - 90.0 + 360.0 * i as f64 / n as f64)).collect();
            Path::polygon(&pts, true, None, None)
        } else if let Some([cx, cy, outer, inner, n]) = spec.star {
            let n = n.round().max(3.0) as usize;
            let pts: Vec<Point> = (0..n * 2).map(|i| polar(cx, cy, if i % 2 == 0 { outer } else { inner }, spec.rotate - 90.0 + 180.0 * i as f64 / n as f64)).collect();
            Path::polygon(&pts, true, None, None)
        } else {
            bail!("no geometry: give one of d, points, rect, ellipse, polygon, star");
        };
        let morph_to = spec.morph_to.as_ref().map(|d| Path::from_svg(d).ok_or_else(|| anyhow!("could not read the morph_to path {d:?}"))).transpose()?;
        if let Some(to) = &morph_to {
            path.interpolate(to, 0.5).context("morph_to must have the same structure as the path (same commands in the same order) to morph")?;
        }
        Ok(PathGeo { path, morph_to })
    }

    /// The path morphed by `amount` (0 = this path, 1 = `morph_to`).
    pub fn at(&self, amount: f64) -> Path {
        match (&self.morph_to, amount) {
            (Some(to), a) if a.abs() > 1e-9 => self.path.interpolate(to, 1.0 - a.clamp(0.0, 1.0) as f32).unwrap_or_else(|| self.path.clone()),
            _ => self.path.clone(),
        }
    }
}

fn polar(cx: f64, cy: f64, r: f64, deg: f64) -> Point {
    let a = deg.to_radians();
    Point::new((cx + r * a.cos()) as f32, (cy + r * a.sin()) as f32)
}

/// A Catmull-Rom spline through `points` as cubic Béziers.
fn smooth(points: &[[f64; 2]], closed: bool) -> Path {
    let n = points.len();
    let p = |i: isize| -> [f64; 2] {
        if closed { points[i.rem_euclid(n as isize) as usize] } else { points[i.clamp(0, n as isize - 1) as usize] }
    };
    let mut path = Path::new();
    path.move_to((points[0][0] as f32, points[0][1] as f32));
    let segs = if closed { n } else { n - 1 };
    for i in 0..segs as isize {
        let (p0, p1, p2, p3) = (p(i - 1), p(i), p(i + 1), p(i + 2));
        let c1 = (p1[0] + (p2[0] - p0[0]) / 6.0, p1[1] + (p2[1] - p0[1]) / 6.0);
        let c2 = (p2[0] - (p3[0] - p1[0]) / 6.0, p2[1] - (p3[1] - p1[1]) / 6.0);
        path.cubic_to((c1.0 as f32, c1.1 as f32), (c2.0 as f32, c2.1 as f32), (p2[0] as f32, p2[1] as f32));
    }
    if closed {
        path.close();
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(json: &str) -> PathSpec {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn every_geometry_builds_and_has_bounds() {
        for (json, w) in [
            (r#"{"rect": [10, 20, 100, 50], "radius": 8}"#, 100.0),
            (r#"{"ellipse": [0, 0, 40, 20]}"#, 80.0),
            (r#"{"polygon": [0, 0, 50, 6]}"#, 86.6),
            (r#"{"star": [0, 0, 60, 25, 5]}"#, 114.0),
            (r#"{"d": "M 0 0 L 100 0 L 100 100 Z"}"#, 100.0),
            (r#"{"points": [[0, 0], [50, 80], [100, 0]], "smooth": true}"#, 100.0),
        ] {
            let g = PathGeo::build(&spec(json)).unwrap_or_else(|e| panic!("{json}: {e}"));
            assert!((g.path.bounds().width() - w as f32).abs() < 8.0, "{json}: {}", g.path.bounds().width());
        }
    }

    #[test]
    fn morph_needs_matching_structure() {
        let ok = PathGeo::build(&spec(r#"{"d": "M 0 0 L 10 0 L 10 10 Z", "morph_to": "M 0 0 L 50 0 L 50 50 Z"}"#)).unwrap();
        assert!((ok.at(1.0).bounds().width() - 50.0).abs() < 1e-3);
        assert!(PathGeo::build(&spec(r#"{"d": "M 0 0 L 10 0 Z", "morph_to": "M 0 0 C 1 1 2 2 3 3 Z"}"#)).is_err());
    }
}
