//! Vector geometry as written in a composition: shapes, masks and motion paths all use `PathSpec`.

use serde::Deserialize;

use super::wrap::P;

/// Exactly one of the geometry fields is set.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default, deny_unknown_fields)]
pub struct PathSpec {
    /// An SVG path string: `"M 0 0 L 100 0 L 100 100 Z"`.
    pub d: Option<String>,
    /// Points joined by straight lines, or by a smooth curve with `smooth: true`.
    pub points: Option<Vec<[f64; 2]>>,
    pub closed: bool,
    pub smooth: bool,
    /// `[x, y, w, h]` with optional corner `radius`.
    pub rect: Option<[f64; 4]>,
    pub radius: f64,
    /// `[cx, cy, rx, ry]`.
    pub ellipse: Option<[f64; 4]>,
    /// Regular polygon: `[cx, cy, radius, sides]`.
    pub polygon: Option<[f64; 4]>,
    /// Star: `[cx, cy, outer, inner, points]`.
    pub star: Option<[f64; 5]>,
    /// Rotation in degrees for polygon and star.
    pub rotate: f64,
    /// Morph toward another SVG path (same structure) by `amount` 0..1, GSAP MorphSVG style.
    pub morph_to: Option<String>,
    pub morph: Option<P>,
}

impl PathSpec {
    pub fn geometry_count(&self) -> usize {
        [self.d.is_some(), self.points.is_some(), self.rect.is_some(), self.ellipse.is_some(), self.polygon.is_some(), self.star.is_some()].iter().filter(|b| **b).count()
    }
}
