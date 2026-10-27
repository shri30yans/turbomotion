//! The palette cycle and the stroke taper through the SDK: written as the engine reads them, and a wrong one is refused
//! with the engine's message and the line of the script.
use std::{fs, path::PathBuf};
use turbomotion_script::{build, check::auto, Options};

mod common;

fn script(name: &str, body: &str) -> PathBuf {
    let path = common::scratch("palette-taper", name).join("film.ts");
    fs::write(&path, format!("import {{comp, shape, solid}} from 'turbomotion';\nconst f = comp({{width: 400, height: 200, fps: 30, dur: 1}});\n{body}\nexport default f;")).unwrap();
    path
}

#[test]
fn a_repeat_with_a_palette_and_a_taper_reach_the_film_as_written() {
    let path = script(
        "emits",
        "f.add(solid('#fff', {id: 'bar', size: [40, 40], pos: [30, 30], repeat: {count: 3, offset: [50, 0], palette: ['#e63946', '#2a9d8f'], cycle: {mode: 'gradient', offset: 0.5, reverse: true}, paint: 'fill'}}));\n\
         f.add(shape([{d: 'M 40 100 L 360 100', stroke: {color: '#fff', width: 20}, taper: {start: {length: 80, width: 0}, end: {length: {fraction: 0.2}, width: {fraction: 0.25}}, ease: 'power2.out'}, fill: '#fff', repeat: {copies: 3, offset: [0, 20], palette: ['#f00', '#00f'], paint: 'stroke'}}]));",
    );
    let value = build(&path, &Options::default()).unwrap();
    let layers = value["layers"].as_array().unwrap();
    let bar = layers.iter().find(|l| l["id"] == "bar").unwrap();
    assert_eq!((bar["repeat"]["cycle"]["mode"].as_str(), bar["repeat"]["cycle"]["reverse"].as_bool(), bar["repeat"]["palette"][1].as_str()), (Some("gradient"), Some(true), Some("#2a9d8f")));
    let item = &layers.last().unwrap()["items"][0];
    assert_eq!((item["taper"]["start"]["length"].as_f64(), item["taper"]["end"]["width"]["fraction"].as_f64(), item["taper"]["ease"].as_str()), (Some(80.0), Some(0.25), Some("power2.out")));
    assert_eq!((item["repeat"]["palette"][0].as_str(), item["repeat"]["paint"].as_str()), (Some("#f00"), Some("stroke")));
}

#[test]
fn the_engine_checks_a_palette_and_a_taper_written_in_a_script_and_names_the_line() {
    let good = script("good", "f.add(shape([{d: 'M 40 100 L 360 100', stroke: {color: '#fff', width: 20}, taper: {end: {length: 90}}}]));");
    let report = auto(&good, None, None, true).unwrap();
    assert!(report.success, "{}", report.diagnostics);
    let empty = script("empty", "f.add(solid('#fff', {id: 'bar', repeat: {count: 3, palette: []}}));");
    let report = auto(&empty, None, None, true).unwrap();
    assert!(!report.success && report.diagnostics.contains("palette has no colours"), "{}", report.diagnostics);
    let fill_only = script("fill-only", "f.add(shape([{rect: [0, 0, 50, 50], fill: '#fff', taper: {end: {length: 10}}}]));");
    let report = auto(&fill_only, None, None, true).unwrap();
    assert!(!report.success && report.diagnostics.contains("film.ts:3:") && report.diagnostics.contains("taper thins a stroke"), "{}", report.diagnostics);
}
