//! The text `box` through the SDK: the option is written as the engine reads it, a wrong one is refused with the engine's
//! message, and `measure()` returns the bounds the box is fitted to.
use std::{fs, path::PathBuf};
use turbomotion_script::{build, check::auto, Options};

mod common;

fn script(name: &str, source: &str) -> PathBuf {
    let path = common::scratch("text-box", name).join("film.ts");
    fs::write(&path, source).unwrap();
    path
}

fn film(box_option: &str) -> String {
    format!("import {{comp, text}} from 'turbomotion';\nconst f = comp({{width: 400, height: 200, fps: 30, dur: 1}});\nf.add(text('Hello', {{id: 'bar', size: 60, pos: [40, 100], box: {box_option}}}));\nexport default f;")
}

#[test]
fn a_box_option_reaches_the_film_as_written() {
    let path = script("emits", &film("{fit: 'text', padding: [24, 12], radius: 16, fill: '#e63946', min_width: 200, align: 'center', per_line: true}"));
    let value = build(&path, &Options::default()).unwrap();
    let b = &value["layers"][0]["box"];
    assert_eq!((b["fit"].as_str(), b["padding"][0].as_f64(), b["radius"].as_f64(), b["min_width"].as_f64(), b["per_line"].as_bool()), (Some("text"), Some(24.0), Some(16.0), Some(200.0), Some(true)));
}

#[test]
fn the_engine_checks_a_box_written_in_a_script_and_names_the_line() {
    let good = script("good", &film("{fit: 'text', padding: [{from: 4, to: 24, dur: 0.5}, 12], fill: '#e63946'}"));
    let report = auto(&good, None, None, true).unwrap();
    assert!(report.success, "{}", report.diagnostics);
    let bad = script("bad", &film("{fit: 'text', padding: [-4, 12], fill: '#e63946'}"));
    let report = auto(&bad, None, None, true).unwrap();
    assert!(!report.success && report.diagnostics.contains("film.ts:3:") && report.diagnostics.contains("padding x is 0 or more pixels, not -4"), "{}", report.diagnostics);
}

#[test]
fn measure_returns_the_bounds_a_box_is_fitted_to() {
    let path = script("measure", "import {comp, measure} from 'turbomotion'; comp({width: 100, height: 100, fps: 30}); const m = measure('Hi there\\nyo', {size: 80}); export default {m, none: measure('', {size: 80}).bounds};");
    let value = build(&path, &Options::default()).unwrap();
    let (m, lines) = (&value["m"], &value["m"]["bounds"]["lines"]);
    assert_eq!(lines.as_array().unwrap().len(), 2, "one extent per line");
    let (first, last) = (&m["chars"][0], &m["chars"][7]);
    let width = last["x"].as_f64().unwrap() + last["w"].as_f64().unwrap() - first["x"].as_f64().unwrap();
    assert!((lines[0]["rect"]["width"].as_f64().unwrap() - width).abs() < 0.01, "extent {} vs characters {width}: {:?}", lines[0]["rect"]["width"], (first, last));
    assert_eq!(m["bounds"]["block"]["width"], lines[0]["rect"]["width"], "the block is as wide as its widest line");
    assert!(value["none"].is_null(), "no characters, no bounds");
}
