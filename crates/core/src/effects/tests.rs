//! The registry: every declared effect builds, parameters are checked, names are unique.

use super::*;

fn params_of(v: &Value) -> Map<String, Value> {
    let mut o = v.as_object().expect("an example is an object").clone();
    o.remove("effect");
    o
}

#[test]
fn every_catalogue_example_builds_as_its_own_effect_with_declared_parameters_only() {
    assert!(CATALOGUE.len() >= 25);
    for entry in CATALOGUE {
        let example = (entry.example)();
        assert_eq!(example["effect"], entry.effect, "{}", entry.effect);
        let effect = Effect::build(entry.effect, &params_of(&example)).unwrap_or_else(|e| panic!("{}: {e:#}", entry.effect));
        assert_eq!(effect.name(), entry.effect);
    }
}

#[test]
fn leaving_every_parameter_out_builds_every_effect_from_its_defaults() {
    for entry in CATALOGUE {
        Effect::build(entry.effect, &Map::new()).unwrap_or_else(|e| panic!("{}: {e:#}", entry.effect));
    }
}

#[test]
fn names_are_unique_and_parameter_names_are_unique_within_an_effect() {
    let mut names: Vec<_> = CATALOGUE.iter().map(|e| e.effect).collect();
    names.sort_unstable();
    let before = names.len();
    names.dedup();
    assert_eq!(before, names.len());
    for entry in CATALOGUE {
        let mut params: Vec<_> = entry.params.iter().map(|p| p.name).collect();
        params.sort_unstable();
        let before = params.len();
        params.dedup();
        assert_eq!(before, params.len(), "{}", entry.effect);
    }
}

#[test]
fn every_parameter_is_documented_and_has_a_sane_range() {
    for entry in CATALOGUE {
        assert!(!entry.summary.is_empty(), "{}", entry.effect);
        for p in entry.params {
            assert!(!p.doc.is_empty(), "{} {}", entry.effect, p.name);
            if let (Some(lo), Some(hi)) = (p.min, p.max) {
                assert!(lo < hi, "{} {}", entry.effect, p.name);
            }
        }
    }
}

#[test]
fn unknown_effects_and_parameters_are_errors() {
    let none = Map::new();
    assert!(Effect::build("sparkle", &none).unwrap_err().to_string().contains("blur"));
    let e = Effect::build("blur", &params_of(&serde_json::json!({"radis": 3}))).unwrap_err().to_string();
    assert!(e.contains("\"radis\"") && e.contains("radius"), "{e}");
    let keys_out_of_order = params_of(&serde_json::json!({"radius": [[1, 2], [0, 3]]}));
    assert!(Effect::build("blur", &keys_out_of_order).unwrap_err().to_string().contains("parameter \"radius\""));
}

#[test]
fn a_value_outside_the_declared_range_is_held_at_its_edge() {
    let Effect::Pixelate(p) = Effect::build("pixelate", &params_of(&serde_json::json!({"size": 0.2}))).unwrap() else { panic!("a pixelate") };
    assert_eq!(p.size.at(0.0), 1.0);
}
