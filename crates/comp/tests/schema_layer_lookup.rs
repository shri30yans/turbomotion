//! A layer is found by its bare id or by its path through precompositions, and never ambiguously.

#[allow(dead_code)] // each test crate uses a different part of the helpers
mod schema_common;

use schema_common::plan_of;
use serde_json::{json, Value};
use turbomotion_comp::lookup::{paths, resolve};

/// A film as the SDK builds it: a root layer, and two shots (precomps named by the shot id), one holding a nested card.
fn film() -> Value {
    json!({"dur": 6,
        "comps": {
            "intro": {"dur": 3, "layers": [
                {"id": "title", "type": "solid", "color": "#fff"},
                {"id": "glow", "type": "solid", "color": "#fff"},
                {"id": "card", "type": "precomp", "comp": "card"}]},
            "outro": {"dur": 3, "layers": [{"id": "glow", "type": "solid", "color": "#fff"}]},
            "card": {"dur": 3, "layers": [{"id": "word", "type": "solid", "color": "#fff"}]}},
        "layers": [
            {"id": "bug", "type": "solid", "color": "#fff"},
            {"id": "intro", "type": "precomp", "comp": "intro"},
            {"id": "outro", "type": "precomp", "comp": "outro", "start": 3}]})
}

#[test]
fn every_layer_has_a_path_through_its_precompositions() {
    let known = paths(&plan_of(film()));
    for p in ["bug", "intro", "intro/title", "intro/card", "intro/card/word", "outro/glow"] {
        assert!(known.iter().any(|k| k == p), "{p} missing from {known:?}");
    }
}

#[test]
fn a_bare_id_resolves_to_its_path_and_a_full_path_is_kept() {
    let plan = plan_of(film());
    assert_eq!(resolve(&plan, "title").unwrap(), "intro/title");
    assert_eq!(resolve(&plan, "word").unwrap(), "intro/card/word");
    assert_eq!(resolve(&plan, "outro/glow").unwrap(), "outro/glow");
    assert_eq!(resolve(&plan, "bug").unwrap(), "bug");
}

#[test]
fn an_id_held_by_two_shots_is_an_error_listing_both() {
    let message = resolve(&plan_of(film()), "glow").unwrap_err().to_string();
    assert!(message.contains("intro/glow") && message.contains("outro/glow"), "{message}");
}

#[test]
fn an_unknown_id_lists_what_exists() {
    let message = resolve(&plan_of(film()), "nope").unwrap_err().to_string();
    assert!(message.contains("intro/title") && message.contains("bug"), "{message}");
}

#[test]
fn a_path_the_film_does_not_list_passes_through_for_the_selector_to_judge() {
    assert_eq!(resolve(&plan_of(film()), "intro/missing").unwrap(), "intro/missing");
}

#[test]
fn a_long_listing_is_cut_with_a_count() {
    let layers: Vec<Value> = (0..30).map(|i| json!({"id": format!("l{i}"), "type": "solid", "color": "#fff"})).collect();
    let message = resolve(&plan_of(json!({"dur": 2, "layers": layers})), "nope").unwrap_err().to_string();
    assert!(message.contains("(30 in all)"), "{message}");
}
