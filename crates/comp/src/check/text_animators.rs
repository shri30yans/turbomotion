//! Text animators that cannot do what they say: a positive opacity delta (the rest opacity is already full) and
//! animators that change no character at any moment the layer is on, which is how a range selector with the wrong
//! `start`, `end` or `offset` looks: no error, no sweep.

use super::{Finding, Severity};
use crate::{
    eval::{Content, Evaluator},
    plan::Plan,
    schema::Kind,
};

/// Sampled moments per layer.
const SAMPLES: usize = 48;

fn warn(rule: &'static str, layer: String, t: Option<f64>, message: String, fix: &str) -> Finding {
    Finding { severity: Severity::Warn, rule, layer: Some(layer), t, message, fix: fix.to_string() }
}

pub(super) fn run(plan: &Plan, ev: &Evaluator, out: &mut Vec<Finding>) -> anyhow::Result<()> {
    for (name, comp) in &plan.comps {
        for l in &comp.layers {
            let Kind::Text(tx) = &l.def.kind else { continue };
            let label = if name.is_empty() { l.label.clone() } else { format!("{} in comp {name:?}", l.label) };
            for (k, a) in tx.animators.iter().enumerate() {
                if a.props.opacity.is_some_and(|o| o > 0.0) {
                    out.push(warn(
                        "animator-opacity-delta",
                        label.clone(),
                        None,
                        format!("animator {k} sets opacity {}: an animator's opacity is a DELTA added to each character's own opacity, which is already full, so a positive value changes nothing", a.props.opacity.unwrap_or_default()),
                        "use a negative delta: -1 hides the characters the animator reaches (direction \"in\" then reveals them as the weight falls to 0)",
                    ));
                }
            }
            if tx.animators.is_empty() || l.def.common.hidden {
                continue;
            }
            let mut on = false;
            let mut changed = false;
            for s in 0..SAMPLES {
                let t = l.start + (l.end.min(comp.dur) - l.start).max(0.0) * (s as f64 + 0.5) / SAMPLES as f64;
                let st = &ev.frame(name, t)?.layers[l.index];
                if let (true, Content::Text(text)) = (st.active, &st.content) {
                    on = true;
                    changed |= text.animated;
                }
            }
            if on && !changed {
                out.push(warn(
                    "animator-never-applies",
                    label,
                    None,
                    "no animator changes any character at any moment the layer is on, so the text just sits at rest".into(),
                    "a range selector affects the characters whose position (0..1 along the text) lies between `start` and `end`, both shifted by `offset`: animate `offset` (e.g. from -1 to 1) to sweep a window across; and values such as opacity are deltas (-1 hides); in stagger mode check `at` and `dur`",
                ));
            }
        }
    }
    Ok(())
}
