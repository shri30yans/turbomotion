//! Which keys a layer of a given type accepts, asked without a whole layer: the message `comp check` gives for a key
//! that is not one of them, and a check of a list of keys for the SDK, which reports a misspelt option where it is written.

use turbomotion_core::suggest::hint;

use super::{fields::Fields, Kind, COMMON_KEYS};

/// Layer keys that a step of `Plan::from_value` reads and removes before a layer is parsed, so the layer's own key list
/// does not show them: `(layer type or "*" for every type, key)`. The test below plans a film that uses each one.
const EXPANDED: &[(&str, &str)] = &[("*", "repeat"), ("*", "extrude"), ("*", "on"), ("precomp", "block"), ("shape", "surface")];

/// The complaint about `key` on a layer labelled `label` of type `ty`: the nearest valid key and what the type takes.
pub fn unknown_key(label: &str, ty: &str, own: Fields, key: &str) -> String {
    let valid: Vec<&str> = COMMON_KEYS.iter().chain(own.keys).copied().collect();
    format!(
        "{label}: unknown key {key:?}.{}{} A {ty} layer takes {}, and every layer takes the common keys (`turbomotion comp catalogue layer {ty}` lists both with their forms)",
        hint(key, &valid),
        belongs_to(key),
        if own.keys.is_empty() { "no keys of its own".to_string() } else { own.keys.join(", ") }
    )
}

/// Where `key` is valid, when it is a key of other layer types: the usual mistake is a text key on a solid.
fn belongs_to(key: &str) -> String {
    let owners: Vec<&str> = Kind::NAMES.iter().copied().filter(|n| super::fields::of_kind(n).is_some_and(|f| f.keys.contains(&key))).collect();
    if owners.is_empty() { String::new() } else { format!(" `{key}` is a key of {} layers.", owners.join(", ")) }
}

/// Every complaint about `keys` (`type` is the kind itself and is not a key) on a layer of type `ty`; empty when they all belong.
pub fn problems(label: &str, ty: &str, keys: &[&str]) -> Vec<String> {
    let Some(own) = super::fields::of_kind(ty) else {
        return vec![format!("{label}: unknown layer type {ty:?}.{} The types are {}", hint(ty, Kind::NAMES), Kind::NAMES.join(", "))];
    };
    keys.iter().filter(|k| **k != "type" && !COMMON_KEYS.contains(k) && !own.keys.contains(k) && !EXPANDED.iter().any(|(t, x)| x == *k && (*t == "*" || *t == ty))).map(|k| unknown_key(label, ty, own, k)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_misspelt_key_names_the_nearest_valid_one_and_what_the_type_takes() {
        let p = problems("text layer \"hi\"", "text", &["text", "colour", "pos", "outline"]);
        assert_eq!(p.len(), 2, "{p:?}");
        assert!(p[0].contains("unknown key \"colour\". Did you mean \"color\"?") && p[0].contains("A text layer takes text, font"), "{}", p[0]);
        assert!(p[1].contains("\"outline\"") && p[1].contains("outline_only"), "{}", p[1]);
    }

    #[test]
    fn a_key_of_another_layer_type_says_which_layers_take_it() {
        let p = problems("solid \"s\"", "solid", &["box", "per_line"]);
        assert!(p[0].contains("unknown key \"box\".") && p[0].contains("`box` is a key of text layers."), "{}", p[0]);
        assert!(!p[1].contains("is a key of"), "`per_line` is not a layer key anywhere, only a key of the box: {}", p[1]);
    }

    #[test]
    fn the_keys_an_expansion_step_consumes_are_accepted_and_really_are_consumed() {
        let layers = serde_json::json!([
            {"type": "shape", "id": "plate", "items": [{"rect": [-50, -30, 100, 60]}], "surface": "card", "pos": [200, 200]},
            {"type": "text", "id": "tag", "text": "Hi", "on": "plate", "size": 40},
            {"type": "solid", "id": "slab", "color": "#8a5cff", "size": [40, 40], "repeat": {"count": 2, "offset": [10, 0]}},
            {"type": "text", "id": "deep", "text": "88", "size": 120, "three_d": true, "pos": [300, 200, 0], "extrude": {"depth": 20}},
            {"type": "camera", "id": "cam"},
            {"type": "precomp", "id": "lt", "block": "lower_third"}
        ]);
        let film = serde_json::json!({"width": 640, "height": 360, "dur": 1, "surface": {"card": {"fill": "#223"}}, "layers": layers});
        crate::plan::Plan::from_value(film, std::path::Path::new(".")).expect("the engine plans a film that uses every expanded key");
        for l in layers.as_array().unwrap() {
            let keys: Vec<&str> = l.as_object().unwrap().keys().map(String::as_str).collect();
            let ty = l["type"].as_str().unwrap();
            assert!(problems("l", ty, &keys).is_empty(), "{ty}: {:?}", problems("l", ty, &keys));
        }
        assert!(!problems("l", "solid", &["surface"]).is_empty(), "`surface` belongs to shapes only");
    }

    #[test]
    fn every_valid_key_passes_and_an_unknown_type_is_named_with_the_types() {
        assert!(problems("l", "solid", &["color", "size", "pos", "fx", "id"]).is_empty());
        assert!(problems("l", "null", &["pos", "scale"]).is_empty());
        let p = problems("l", "solidd", &["color"]);
        assert!(p[0].contains("unknown layer type \"solidd\"") && p[0].contains("Did you mean \"solid\"?"), "{p:?}");
    }
}
