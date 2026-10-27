//! `visible-effect-input`: a layer an effect reads as a map or mask is drawn on the composite as well unless it is hidden, so
//! the map shows on screen.

use super::{shows, Finding, Severity};
use crate::{eval::Evaluator, plan::Plan};

/// Moments of a composition at which the layers an effect reads are looked at.
const EFFECT_INPUT_SAMPLES: usize = 24;

/// A layer an effect reads as a map or mask is drawn on the composite as well unless it is hidden, so the map shows on
/// screen. That is rarely meant: say `"hidden": true` on it.
pub(super) fn run(plan: &Plan, ev: &Evaluator) -> anyhow::Result<Vec<Finding>> {
    let mut out = vec![];
    for (name, comp) in &plan.comps {
        // (layer read, who reads it, through which effect)
        let readers = comp.layers.iter().flat_map(|l| l.fx.iter().map(move |f| (l.label.as_str(), f))).chain(comp.look.iter().map(|f| ("look", f)));
        let mut reads: Vec<(usize, &str, &str)> = vec![];
        for (reader, f) in readers {
            reads.extend(f.layers_read().into_iter().map(|read| (read, reader, f.name())));
        }
        for (read, reader, effect) in reads {
            let seen = (0..EFFECT_INPUT_SAMPLES).map(|k| comp.dur * (k as f64 + 0.5) / EFFECT_INPUT_SAMPLES as f64).find_map(|t| match ev.frame(name, t) {
                Ok(f) => shows(&f.layers[read]).then_some(Ok(t)),
                Err(e) => Some(Err(e)),
            });
            if let Some(t) = seen.transpose()? {
                let label = &comp.layers[read].label;
                let place = if name.is_empty() { String::new() } else { format!(" in comp {name:?}") };
                out.push(Finding {
                    severity: Severity::Warn,
                    rule: "visible-effect-input",
                    layer: Some(format!("{label}{place}")),
                    t: Some(t),
                    message: format!("is read by effect {effect:?} on {reader} but is also drawn on screen, so the map itself shows"),
                    fix: "set \"hidden\": true on it; an effect still reads a hidden layer".to_string(),
                });
            }
        }
    }
    Ok(out)
}
