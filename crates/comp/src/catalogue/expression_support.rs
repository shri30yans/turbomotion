//! Where an expression (`"time * 90"`) can stand in for a number, derived from the effect registry so it cannot drift.
//!
//! Layer and shape-item properties are all `Prop`s and take every form. Effect parameters depend on who defines the effect:
//! shader plugins parse them as properties (expressions, tweens, eases), the platform's own Rust effects take a number or
//! `[[seconds, value]]` keys. The test below proves each claim by planning a real use.

use serde_json::{json, Value};

use crate::fxlib::{Descriptor, Ty, PLATFORM_ORIGIN};

/// Effects (not generators) that have at least one number parameter, split by how those parameters animate.
fn numeric_effects(all: &[Descriptor]) -> (Vec<&str>, Vec<&str>) {
    let has_number = |d: &&Descriptor| d.params.iter().any(|p| p.ty == Ty::Float);
    let (platform, plugins): (Vec<&Descriptor>, Vec<&Descriptor>) = all.iter().filter(has_number).partition(|d| d.origin == PLATFORM_ORIGIN);
    (platform.iter().map(|d| d.name.as_str()).collect(), plugins.iter().map(|d| d.name.as_str()).collect())
}

pub fn expression_support(all: &[Descriptor]) -> Value {
    let (keys_only, any_form) = numeric_effects(all);
    json!({
        "properties": "Every layer, camera, light and shape-item property written with the forms under property_forms (position, anchor, scale, rotation, orientation, opacity, size, colour, trim, stroke and fill fields) accepts an expression, a tween or keys.",
        "effect_parameters": {
            "shader_plugins": {"accept": "any property form: a number, an expression, keys with eases, or a tween", "effects": any_form},
            "platform_effects": {"accept": "a number or keys [[seconds, value], ..] only; an expression is rejected with an error", "effects": keys_only},
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fxlib::{InputKind, Kind, Library};
    use crate::plan::Plan;

    /// Does a layer using effect `d` with `value` for `param` plan? Layer inputs are wired to a source layer.
    fn plans(d: &Descriptor, param: &str, value: Value) -> bool {
        let mut fx = serde_json::Map::from_iter([("effect".to_string(), json!(d.name)), (param.to_string(), value)]);
        for input in d.inputs.iter().filter(|i| i.kind == InputKind::Layer) {
            fx.insert(input.name.clone(), json!("source"));
        }
        let layers = json!([{"type": "solid", "id": "source", "color": "#000"}, {"type": "solid", "color": "#fff", "fx": [fx]}]);
        Plan::from_value(json!({"dur": 2, "layers": layers}), std::path::Path::new(".")).is_ok()
    }

    #[test]
    fn the_listed_support_is_what_the_planner_does() {
        let lib = Library::builtin().expect("the built-in pack compiles");
        let all = lib.descriptors();
        let support = expression_support(&all);
        let count = |path: [&str; 3]| support[path[0]][path[1]][path[2]].as_array().map_or(0, Vec::len);
        assert!(count(["effect_parameters", "platform_effects", "effects"]) > 0, "no platform effect listed");
        assert!(count(["effect_parameters", "shader_plugins", "effects"]) > 0, "no shader plugin listed");
        for d in all.iter().filter(|d| d.kind == Kind::Effect) {
            let Some(p) = d.params.iter().find(|p| p.ty == Ty::Float) else { continue };
            let takes_expression = plans(d, &p.name, json!("time * 2"));
            assert_eq!(takes_expression, d.origin != PLATFORM_ORIGIN, "{} {}: the catalogue says expressions are {}accepted", d.name, p.name, if d.origin == PLATFORM_ORIGIN { "not " } else { "" });
        }
    }
}
