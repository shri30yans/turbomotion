//! The colour grade: its SkSL (shared by every shader that grades) and the uniforms that parameterise it.

use serde::Deserialize;

use super::{color::Rgb, shader::Uniforms};

/// SkSL prelude declaring `g1`, `g2`, `g3` and `half3 grade(half3)`; prepend it to a program's own source.
pub const SKSL: &str = include_str!("shaders/grade.sksl");

/// Lift / gamma / gain / saturation / tint / contrast. An empty grade is skipped entirely.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Grade {
    pub lift: Option<f64>,
    pub gamma: Option<f64>,
    pub gain: Option<f64>,
    pub sat: Option<f64>,
    pub tint: Option<Rgb>,
    pub tint_k: Option<f64>,
    pub contrast: Option<f64>,
}

impl Grade {
    pub fn is_empty(&self) -> bool {
        self.lift.is_none() && self.gamma.is_none() && self.gain.is_none() && self.sat.is_none() && self.tint.is_none() && self.tint_k.is_none() && self.contrast.is_none()
    }
}

/// Uniforms for the prelude, in declaration order: (lift, gamma, gain, sat), (tint rgb, tint_k), (contrast, enabled).
/// An empty grade is switched off entirely, as the reference skips it.
pub fn uniforms(g: &Grade) -> Uniforms {
    let [r, gr, b] = g.tint.map(|t| t.map(|v| f64::from(v) / 255.0)).unwrap_or([0.0; 3]);
    Uniforms::new()
        .v4(g.lift.unwrap_or(0.0), g.gamma.unwrap_or(1.0), g.gain.unwrap_or(1.0), g.sat.unwrap_or(1.0))
        .v4(r, gr, b, g.tint.map_or(0.0, |_| g.tint_k.unwrap_or(0.0)))
        .v2(g.contrast.unwrap_or(1.0), if g.is_empty() { 0.0 } else { 1.0 })
}
