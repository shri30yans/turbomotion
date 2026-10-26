//! `check`: is this composition sane, and does it do what its author meant? Two parts, both on evaluated frames, so no
//! pixel is drawn: lint rules that need no knowledge of intent (off-screen text, upscaled photos), and assertions the
//! author writes down (`comp.assert.json`) so intent becomes a test.

pub mod assertions;
mod behind_camera;
pub mod cost;
pub mod cover;
pub mod layout;
pub mod lint;
mod text_animators;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The composition will not do what it says.
    Error,
    /// Probably a mistake or a quality problem.
    Warn,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub severity: Severity,
    /// A stable name for the rule, e.g. `outside-safe-area`.
    pub rule: &'static str,
    pub layer: Option<String>,
    /// Seconds into the composition where it was seen.
    pub t: Option<f64>,
    pub message: String,
    pub fix: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sev = if self.severity == Severity::Error { "error" } else { "warn " };
        write!(f, "{sev} [{}]", self.rule)?;
        if let Some(l) = &self.layer {
            write!(f, " {l}")?;
        }
        if let Some(t) = self.t {
            write!(f, " at {t:.2}s")?;
        }
        write!(f, ": {}", self.message)?;
        if !self.fix.is_empty() {
            write!(f, "\n        fix: {}", self.fix)?;
        }
        Ok(())
    }
}

/// Every frame time of the root composition, capped so a long film stays quick to check.
pub fn frame_times(plan: &crate::plan::Plan, cap: usize) -> Vec<f64> {
    let n = plan.frames() as usize;
    let step = n.div_ceil(cap).max(1);
    (0..n).step_by(step).map(|f| f as f64 / f64::from(plan.fps)).collect()
}

/// A layer shows when it is drawn and not (nearly) transparent.
pub fn shows(l: &crate::eval::LayerState) -> bool {
    if !(l.visible && l.opacity > 0.05) {
        return false;
    }
    if l.clip.is_some_and(|c| !c.reveals()) {
        return false;
    }
    // a text layer whose characters are all still hidden by an animator is not showing yet
    match &l.content {
        crate::eval::Content::Text(t) if !t.glyphs.is_empty() => t.glyphs.iter().any(|g| g.opacity * l.opacity > 0.05),
        _ => true,
    }
}
