//! A shot hands over to the next one through its `exit`. Anything in the shot that only starts after that exit has
//! begun is seen while the shot is already leaving, so its entrance is wasted; this rule names those layers.

use super::{Finding, Severity};
use crate::{
    plan::{LayerPlan, Plan},
    schema::Kind,
};

/// Seconds a layer may start inside the exit before it is worth a warning (a frame or two is unseen anyway).
const SLACK: f64 = 0.05;

pub fn run(plan: &Plan) -> Vec<Finding> {
    let mut out = vec![];
    for outer in plan.comps.values() {
        for shot in &outer.layers {
            let Kind::Precomp(p) = &shot.def.kind else { continue };
            let Some(exit) = shot.def.common.exit.as_ref().filter(|e| e.dur > 0.0) else { continue };
            let Some(inner) = plan.comps.get(&p.comp) else { continue };
            if p.remap.is_some() || p.looping {
                continue;
            }
            // from here the next shot is on top, in the nested composition's own time
            let covered_from = ((shot.end - exit.dur - shot.start).max(0.0)) * p.speed + p.offset;
            let covered_from = covered_from.min(inner.dur);
            out.extend(late_layers(inner.layers.iter(), covered_from).map(|l| Finding {
                severity: Severity::Warn,
                rule: "transition-covers-last-beat",
                layer: Some(format!("{}/{}", shot.label, l.label)),
                t: Some(shot.start + (l.start - p.offset) / p.speed),
                message: format!(
                    "{} starts at {:.2}s in {:?}, but the {:?} exit of {:?} begins at {covered_from:.2}s, so it appears while the next shot is already covering this one",
                    l.label, l.start, p.comp, exit.preset, shot.label
                ),
                fix: "start it before the exit, shorten the next transition (its `in` dur), or lengthen the shot so its last beat lands in the clear".into(),
            }));
        }
    }
    out
}

fn late_layers<'a>(layers: impl Iterator<Item = &'a LayerPlan>, from: f64) -> impl Iterator<Item = &'a LayerPlan> {
    layers.filter(move |l| {
        let draws = !matches!(l.def.kind, Kind::Audio(_) | Kind::Camera(_) | Kind::Light(_) | Kind::Null | Kind::Adjustment) && !l.def.common.hidden;
        draws && l.start > from + SLACK && l.start < l.end
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn findings(late: f64) -> Vec<Finding> {
        let film = json!({"width": 640, "height": 360, "timeline": {"transition": {"type": "push_left", "dur": 1.0}, "shots": [
            {"id": "a", "dur": 4, "layers": [{"type": "text", "text": "early", "start": 0.5}, {"type": "text", "text": "last beat", "id": "beat", "start": late}]},
            {"id": "b", "dur": 3, "layers": []}
        ]}});
        run(&Plan::from_value(film, std::path::Path::new(".")).unwrap())
    }

    #[test]
    fn a_layer_that_starts_inside_the_next_transition_is_named() {
        let got = findings(3.5);
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0].rule, "transition-covers-last-beat");
        assert!(got[0].layer.as_deref().unwrap().contains("beat"), "{:?}", got[0].layer);
    }

    #[test]
    fn a_layer_that_lands_before_the_exit_is_quiet() {
        assert!(findings(2.5).is_empty());
    }
}
