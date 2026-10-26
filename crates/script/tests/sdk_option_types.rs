//! The SDK's option types name every field the engine accepts. The engine's own lists are read from its source, so a
//! field added there without an SDK type fails here instead of being discovered by trial.
use std::{fs, path::Path};

fn read(relative: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}

/// The quoted names of `const NAME: &[&str] = &[ ... ];`.
fn const_list(source: &str, name: &str) -> Vec<String> {
    let start = source.find(&format!("const {name}")).unwrap_or_else(|| panic!("no const {name}"));
    let body = &source[start..start + source[start..].find("];").unwrap()];
    body.split('"').skip(1).step_by(2).map(str::to_string).collect()
}

/// The field names of `pub struct NAME { ... }` (the first one in `source`).
fn struct_fields(source: &str, name: &str) -> Vec<String> {
    let start = source.find(&format!("pub struct {name} {{")).unwrap_or_else(|| panic!("no struct {name}"));
    let body = &source[start..start + source[start..].find("\n}").unwrap()];
    body.lines().skip(1).filter_map(|l| l.trim().strip_prefix("pub ")).filter_map(|l| l.split(':').next()).map(str::to_string).collect()
}

/// The names declared by `interface NAME ... { ... }` in an SDK type file.
fn interface_keys(source: &str, name: &str) -> Vec<String> {
    let start = source.find(&format!("export interface {name}")).unwrap_or_else(|| panic!("no interface {name}"));
    let body = &source[start..start + source[start..].find("\n}").unwrap()];
    let is_key = |l: &&str| l.starts_with("  ") && !l.starts_with("   ") && !l.trim_start().starts_with("/**");
    body.lines().filter(is_key).filter_map(|l| l.trim().split(['?', ':']).next()).map(str::to_string).collect()
}

fn missing(engine: &[String], sdk: &[String], exempt: &[&str]) -> Vec<String> {
    engine.iter().filter(|k| !sdk.contains(k) && !exempt.contains(&k.as_str())).cloned().collect()
}

#[test]
fn every_common_layer_field_has_an_sdk_option() {
    let engine = const_list(&read("../comp/src/schema/mod.rs"), "COMMON_KEYS");
    let sdk = interface_keys(&read("sdk/types.ts"), "LayerOpts");
    assert!(missing(&engine, &sdk, &[]).is_empty(), "LayerOpts lacks {:?}", missing(&engine, &sdk, &[]));
}

#[test]
fn every_text_field_has_an_sdk_option() {
    let engine = struct_fields(&read("../comp/src/schema/kinds.rs"), "Text");
    let sdk = interface_keys(&read("sdk/types.ts"), "TextOpts");
    // `text` is the first argument of `text()`, not an option
    assert!(missing(&engine, &sdk, &["text"]).is_empty(), "TextOpts lacks {:?}", missing(&engine, &sdk, &["text"]));
}

#[test]
fn every_shape_item_key_has_an_sdk_type() {
    let schema = read("../comp/src/schema/shape.rs");
    let engine: Vec<String> = [const_list(&schema, "PATH_KEYS"), const_list(&schema, "ITEM_KEYS")].concat();
    let types = read("sdk/shape-types.ts");
    let sdk: Vec<String> = [interface_keys(&types, "PathGeometry"), interface_keys(&types, "ItemPaint")].concat();
    assert!(missing(&engine, &sdk, &[]).is_empty(), "shape types lack {:?}", missing(&engine, &sdk, &[]));
}

#[test]
fn the_effect_space_option_is_typed_with_exactly_the_engines_names() {
    let engine = const_list(&read("../comp/src/schema/effect_space.rs"), "NAMES");
    assert_eq!(engine, ["screen", "layer"]);
    let source = read("sdk/film-types.ts");
    assert!(interface_keys(&source, "CompOpts").contains(&"effect_space".to_string()), "CompOpts lacks effect_space");
    let line = source.lines().find(|l| l.trim_start().starts_with("effect_space?:")).expect("the effect_space option");
    let typed: Vec<String> = line.split('"').skip(1).step_by(2).map(str::to_string).collect();
    assert_eq!(typed, engine, "the SDK type is `\"screen\" | \"layer\"`, in the engine's order");
}

#[test]
fn the_color_option_is_typed_with_exactly_the_engines_keys_and_names() {
    let engine_source = read("../comp/src/schema/color.rs");
    let source = read("sdk/film-types.ts");
    assert!(interface_keys(&source, "CompOpts").contains(&"color".to_string()), "CompOpts lacks color");
    let engine = struct_fields(&engine_source, "ColorBlock");
    let typed = interface_keys(&source, "ColorOpts");
    assert!(missing(&engine, &typed, &[]).is_empty() && missing(&typed, &engine, &[]).is_empty(), "engine keys {engine:?} against SDK keys {typed:?}");
    let gamuts = const_list(&engine_source[engine_source.find("impl Gamut").unwrap()..], "NAMES");
    assert_eq!(gamuts, ["srgb", "display_p3"]);
    for key in ["space", "working_profile"] {
        let line = source.lines().find(|l| l.trim_start().starts_with(&format!("{key}?:"))).unwrap_or_else(|| panic!("the {key} option"));
        let typed: Vec<String> = line.split('"').skip(1).step_by(2).map(str::to_string).collect();
        assert_eq!(typed, gamuts, "{key} is typed with the engine's gamut names, in its order");
    }
    assert_eq!(const_list(&engine_source, "NAMES"), ["half"], "depth has the one value the SDK types");
}
