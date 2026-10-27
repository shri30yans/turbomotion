//! Motion blur that will not do what its author wrote: a shutter nothing uses, a shutter that is shut, switches that are
//! ignored, and a cost the older key lets run past what the new one allows.

use super::{Finding, Severity};
use crate::{
    plan::{Plan, ROOT},
    schema::{Blur, MAX_BLUR_SAMPLES},
};

fn warn(rule: &'static str, layer: Option<String>, message: String, fix: &str) -> Finding {
    Finding { severity: Severity::Warn, rule, layer, t: None, message, fix: fix.to_string() }
}

pub(super) fn run(plan: &Plan, out: &mut Vec<Finding>) {
    let root = plan.root();
    let switched = root.layers.iter().filter(|l| l.def.common.mblur).count();
    match root.blur {
        Some(_) if switched == 0 => out.push(warn(
            "motion-blur-unused",
            None,
            "the composition sets a motion blur shutter but no layer of it has \"mblur\": true, so nothing is blurred".into(),
            "set \"mblur\": true on the layers that should blur (a precomp layer blurs everything it shows), or drop the shutter",
        )),
        Some(blur) if switched > 0 && (blur.shutter().shutter_angle <= 0.0 || blur.shutter().samples < 2) => out.push(warn(
            "motion-blur-shut",
            None,
            format!("{switched} layer(s) have \"mblur\": true but the shutter averages nothing (angle {}, {} samples)", blur.shutter().shutter_angle, blur.shutter().samples),
            "give the shutter an angle above 0 and at least 2 samples, or switch the layers off",
        )),
        _ => {}
    }
    if let Some(Blur::Legacy(s)) = root.blur.filter(|_| switched > 0) {
        if s.samples > MAX_BLUR_SAMPLES {
            out.push(warn(
                "motion-blur-cost",
                None,
                format!("\"mblur\" averages {} samples, so every blurred layer is drawn {} times per frame; \"motion_blur\" allows at most {MAX_BLUR_SAMPLES} for that reason", s.samples, s.samples),
                "use \"motion_blur\": {\"samples\": 16} or fewer",
            ));
        }
    }
    for (name, comp) in plan.comps.iter().filter(|(n, _)| n.as_str() != ROOT) {
        for l in comp.layers.iter().filter(|l| l.def.common.mblur) {
            out.push(warn(
                "motion-blur-nested",
                Some(format!("{} in comp {name:?}", l.label)),
                "has \"mblur\": true but is inside a nested composition: only the layers of the root composition blur, so this switch does nothing".into(),
                "put \"mblur\": true on the precomp layer that shows this composition: everything it shows blurs",
            ));
        }
        if comp.blur.is_some() {
            out.push(warn(
                "motion-blur-nested",
                Some(format!("comp {name:?}")),
                "sets its own motion blur shutter, but the root composition's is the one used".into(),
                "set \"motion_blur\" on the root composition only",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::{json, Value};

    use crate::{check::lint, plan::Plan};

    fn rules(film: Value) -> Vec<&'static str> {
        lint::run(&Plan::from_value(film, Path::new(".")).unwrap()).unwrap().iter().map(|f| f.rule).filter(|r| r.starts_with("motion-blur")).collect()
    }

    fn film(block: Value, switch: bool) -> Value {
        let mut f = json!({"width": 320, "height": 180, "dur": 1, "comps": {"inner": {"layers": [{"type": "solid", "color": "#fff", "size": [40, 40]}]}},
            "layers": [{"type": "solid", "color": "#ffffff", "size": [40, 40], "pos": [160, 90], "mblur": switch}]});
        f.as_object_mut().unwrap().extend(block.as_object().unwrap().clone());
        f
    }

    #[test]
    fn a_shutter_nothing_uses_or_a_shut_one_is_reported() {
        assert!(rules(film(json!({"motion_blur": {}}), false)).contains(&"motion-blur-unused"));
        assert!(rules(film(json!({"mblur": {}}), false)).contains(&"motion-blur-unused"));
        assert!(rules(film(json!({"motion_blur": {"shutter_angle": 0}}), true)).contains(&"motion-blur-shut"));
        assert!(rules(film(json!({"motion_blur": {}}), true)).is_empty());
        assert!(rules(film(json!({}), true)).is_empty(), "a switch alone uses the older key's defaults");
    }

    #[test]
    fn the_older_key_above_the_cap_is_reported_with_the_cost() {
        let r = lint::run(&Plan::from_value(film(json!({"mblur": {"samples": 32}}), true), Path::new(".")).unwrap()).unwrap();
        let f = r.iter().find(|f| f.rule == "motion-blur-cost").expect("a cost warning");
        assert!(f.message.contains("32 times") && f.message.contains("at most 16"), "{}", f.message);
        assert!(rules(film(json!({"mblur": {"samples": 16}}), true)).iter().all(|r| *r != "motion-blur-cost"));
    }

    #[test]
    fn a_switch_inside_a_nested_composition_is_reported_as_ignored() {
        let mut nested = film(json!({}), false);
        nested["comps"]["inner"]["layers"][0]["mblur"] = json!(true);
        let r = lint::run(&Plan::from_value(nested, Path::new(".")).unwrap()).unwrap();
        let f = r.iter().find(|f| f.rule == "motion-blur-nested").expect("the inner switch is reported");
        assert!(f.message.contains("only the layers of the root") && f.layer.as_deref().is_some_and(|l| l.contains("inner")), "{f}");
    }
}
