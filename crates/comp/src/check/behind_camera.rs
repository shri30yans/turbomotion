//! A 3D layer with one corner at or behind the camera lens cannot be projected, so the evaluator drops the whole layer
//! for that frame. That is how a very wide layer (a marquee row) parented to something that turns "vanishes" partway
//! through a move; say so, with the moment and the fix.

use super::{Finding, Severity};
use crate::{
    eval::Evaluator,
    plan::Plan,
    schema::{Cull, Kind},
};

/// Sampled moments per layer.
const SAMPLES: usize = 48;

pub(super) fn run(plan: &Plan, ev: &Evaluator, out: &mut Vec<Finding>) -> anyhow::Result<()> {
    for (name, comp) in &plan.comps {
        for l in &comp.layers {
            let draws = !matches!(l.def.kind, Kind::Camera(_) | Kind::Light(_) | Kind::Null | Kind::Audio(_) | Kind::Adjustment) && !l.is_matte_source && !l.def.common.hidden && l.in_3d;
            if !draws || l.def.common.cull.unwrap_or_default() != Cull::None {
                continue;
            }
            for k in 0..SAMPLES {
                let t = l.start + (l.end.min(comp.dur) - l.start).max(0.0) * (k as f64 + 0.5) / SAMPLES as f64;
                let f = ev.frame(name, t)?;
                let st = &f.layers[l.index];
                if st.active && !st.visible && st.bounds.is_none() && st.opacity > 0.05 && f.camera.is_some() {
                    let label = if name.is_empty() { l.label.clone() } else { format!("{} in comp {name:?}", l.label) };
                    out.push(Finding {
                        severity: Severity::Warn,
                        rule: "passes-behind-camera",
                        layer: Some(label),
                        t: Some(t),
                        message: "a corner of this 3D layer is at or behind the camera lens, and a layer with a corner behind the lens is not drawn at all (it is not clipped), so it disappears".into(),
                        fix: "make the layer narrower (a marquee: fewer `repeat` copies or a smaller `size`), turn its parent less, or move the camera back (larger `zoom`, more negative `pos` z)".into(),
                    });
                    break;
                }
            }
        }
    }
    Ok(())
}
