//! Every runnable example the engine prints (`comp catalogue`) or the reference pages show (`docs/reference/*.md`) must
//! pass `comp check`: no errors and no warnings. An example that stops working fails the build, and so does a JSON block
//! in the docs the harness cannot place (it must be classified, never skipped).

mod examples;

use examples::{collect_catalogue, collect_docs, Declared, Example};
use serde_json::Value;
use turbomotion_comp::{api, check::Severity};

fn problems(examples: &[Example]) -> Vec<String> {
    let dir = examples::fixtures::folder();
    let mut out = Vec::new();
    for e in examples {
        match api::validate(e.film.clone(), &dir) {
            Err(err) => out.push(format!("{}: {err:#}", e.name)),
            Ok(findings) => {
                for f in findings {
                    if !e.allowed.contains(&f.rule) {
                        let kind = if f.severity == Severity::Error { "error" } else { "warning" };
                        out.push(format!("{}: {kind} {f}", e.name));
                    }
                }
            }
        }
    }
    out
}

#[test]
fn every_catalogue_example_passes_check() {
    let all = turbomotion_comp::catalogue::all();
    let examples = collect_catalogue(&all);
    assert!(examples.len() > 150, "the walker found only {} examples", examples.len());
    let bad = problems(&examples);
    assert!(bad.is_empty(), "{} of {} catalogue examples fail:\n{}", bad.len(), examples.len(), bad.join("\n"));
}

#[test]
fn every_json_block_in_the_reference_pages_passes_check() {
    let examples = collect_docs();
    assert!(examples.len() > 100, "the docs walker found only {} blocks", examples.len());
    let bad = problems(&examples);
    assert!(bad.is_empty(), "{} of {} doc examples fail:\n{}", bad.len(), examples.len(), bad.join("\n"));
}

#[test]
fn every_plugin_example_in_the_shared_effects_folder_passes_check() {
    let dir = examples::plugins::shared_folder();
    if !dir.is_dir() {
        // The shared effects folder belongs to the project that owns the
        // plugin library. A checkout of the engine alone has no such folder,
        // so there is nothing to check there; a checkout that has one still
        // fails on a broken plugin.
        eprintln!(
            "skipping: the shared effects folder {} is not in this checkout (set TURBOMOTION_EXAMPLE_FX_DIR to point at one)",
            dir.display()
        );
        return;
    }
    let examples = examples::plugins::collect(&dir);
    assert!(examples.len() >= 30, "the plugin walker found only {} plugins in {}", examples.len(), dir.display());
    let bad = problems(&examples);
    assert!(bad.is_empty(), "{} of {} plugin examples fail:\n{}", bad.len(), examples.len(), bad.join("\n"));
}

#[test]
fn a_plugin_whose_example_is_wrong_fails_and_one_without_a_header_example_is_tested_on_its_defaults() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("plugin-examples-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let header = |name: &str, extra: &str| format!("// @effect {name}\n// @summary a test plugin\n// @param amount float 0.5 0 1 -- how much\n{extra}half4 main(float2 xy) {{ return image.eval(xy) * amount; }}\n");
    std::fs::write(dir.join("plain.sksl"), header("plain", "")).unwrap();
    std::fs::write(dir.join("wrong.sksl"), header("wrong", "// @example {\"effect\": \"wrong\", \"amunt\": 0.2}\n")).unwrap();
    let examples = examples::plugins::collect(&dir);
    let names: Vec<_> = examples.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["plugin plain", "plugin wrong"]);
    assert_eq!(examples[0].film["layers"][0]["fx"][0], serde_json::json!({"effect": "plain", "amount": 0.5}), "the default use");
    let bad = problems(&examples);
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(bad[0].starts_with("plugin wrong:") && bad[0].contains("amunt"), "{bad:?}");
}

#[test]
fn a_bare_object_the_harness_cannot_place_is_an_error_not_a_skip() {
    let declared = Declared::from(&turbomotion_comp::catalogue::all());
    let err = declared.classify("x", &serde_json::json!({"nonsense": 1})).unwrap_err();
    assert!(err.contains("cannot tell what kind of example"), "{err}");
    assert!(declared.classify("x", &Value::Null).is_err());
}

#[test]
fn an_effect_example_that_forgets_its_declared_layer_input_fails_instead_of_being_helped() {
    let declared = Declared::from(&turbomotion_comp::catalogue::all());
    let film = declared.classify("x", &serde_json::json!({"effect": "turbulent_displace", "amount": 40})).unwrap();
    // `{err:#}` is the whole cause chain, which is what the command line prints; `to_string` is only its outermost context
    let err = api::validate(film, &examples::fixtures::folder()).map(|_| ()).unwrap_err();
    let chain = format!("{err:#}");
    assert!(chain.contains("noise"), "the error should name the missing input: {chain}");
    // and a named input gets a source, and only that one
    let film = declared.classify("x", &serde_json::json!({"effect": "blur", "radius": 4, "mode": "screen"})).unwrap();
    assert_eq!(film["layers"].as_array().unwrap().len(), 1, "no layer is invented for a value that is not a declared input");
}

#[test]
fn a_fragment_is_placed_beside_what_the_catalogue_says_it_needs_and_nothing_else() {
    let declared = Declared::from(&turbomotion_comp::catalogue::all());
    let types = |film: &Value| film["layers"].as_array().unwrap().iter().map(|l| l["type"].as_str().unwrap().to_string()).collect::<Vec<_>>();
    // a 3D layer gets the catalogue's camera
    let film = declared.classify("x", &serde_json::json!({"type": "text", "text": "x", "three_d": true, "pos": [960, 540]})).unwrap();
    assert_eq!(types(&film), ["text", "camera"]);
    // a light gets a 3D layer to shade, and that layer its camera
    let film = declared.classify("x", &serde_json::json!({"type": "light", "kind": "point"})).unwrap();
    assert_eq!(types(&film), ["mesh", "light", "camera"]);
    // a flat layer gets nothing
    let film = declared.classify("x", &serde_json::json!({"type": "solid", "color": "#fff"})).unwrap();
    assert_eq!(types(&film), ["solid"]);
    // a composition whose `type` is its text scale is a composition, not a layer
    let film = declared.classify("x", &serde_json::json!({"type": {"title": 48}, "layers": []})).unwrap();
    assert!(film["type"].is_object());
}
