//! Wrong guesses at the builders are refused where they are written, with the form that was meant.
mod common;
use std::fs;
use turbomotion_script::{build, check::auto, Options};

fn film(name: &str, body: &str) -> std::path::PathBuf {
    let dir = common::scratch("builder-errors", name);
    let path = dir.join("film.ts");
    fs::write(&path, format!("import {{ comp, shot, solid, text, rect, group, raw, shape }} from 'turbomotion';\nconst film = comp({{ width: 320, height: 180, fps: 30, dur: 1 }});\n{body}\nexport default film;")).unwrap();
    path
}

fn error_of(name: &str, body: &str) -> String {
    format!("{:#}", build(&film(name, body), &Options::default()).unwrap_err())
}

#[test]
fn a_plain_layer_object_in_a_shot_says_to_use_raw_and_raw_works() {
    let e = error_of("plain-object", "film.timeline({ shots: [shot({ id: 's', dur: 1, layers: [solid('#000'), { type: 'adjustment', fx: [] }] })] });");
    assert!(e.contains("needs raw()") && e.contains("adjustment"), "{e}");
    let ok = build(&film("raw-adjustment", "film.timeline({ shots: [shot({ id: 's', dur: 1, layers: [solid('#000'), raw({ type: 'adjustment', fx: [{ effect: 'vignette' }] })] })] });"), &Options::default()).unwrap();
    assert_eq!(ok["timeline"]["shots"][0]["layers"][1]["type"], "adjustment");
}

#[test]
fn solid_wants_the_colour_first() {
    let e = error_of("solid-object", "film.add(solid({ color: '#f00' } as any));");
    assert!(e.contains("solid(color, opts)") && e.contains("first argument, not an option"), "{e}");
    assert!(error_of("solid-number", "film.add(solid(5 as any));").contains("got number 5"));
}

#[test]
fn text_wants_the_words_first_and_a_counter_or_swap_is_still_fine() {
    let e = error_of("text-object", "film.add(text({ text: 'hi' } as any));");
    assert!(e.contains("text(content, opts)") && e.contains("first argument, not an option"), "{e}");
    assert!(error_of("text-number", "film.add(text(42 as any));").contains("String(n)"));
    let ok = build(&film("text-counter", "film.add(text({ from: 0, to: 9 } as any), text({ from: 'A', to: 'B' } as any));"), &Options::default()).unwrap();
    assert_eq!(ok["layers"].as_array().unwrap().len(), 2);
}

#[test]
fn the_other_builders_name_their_argument_forms() {
    assert!(error_of("rect", "film.add(rect({ w: 3 } as any, 4));").contains("rect(w, h, opts)"));
    assert!(error_of("group", "film.add(group([solid('#000')] as any));").contains("group(opts, children)"));
}

#[test]
fn an_unknown_text_option_names_the_option_the_nearest_valid_one_and_the_line() {
    let path = film("text-option", "film.add(text('hi', { id: 'hi', colour: '#fff' } as any));");
    let report = auto(&path, None, None, true).unwrap();
    assert!(!report.success);
    assert!(report.diagnostics.contains("film.ts:3:") && report.diagnostics.contains("unknown key \"colour\". Did you mean \"color\"?") && report.diagnostics.contains("A text layer takes text, font"), "{}", report.diagnostics);
}

#[test]
fn every_bad_layer_of_a_film_is_reported_at_its_own_script_line() {
    let path = film("many", "film.add(text('a', { id: 'a', colour: '#fff' } as any));\nfilm.add(solid('#000', { id: 'b', dure: 2 } as any));\nfilm.add(shape([{ ellipse: [0, 0, 5, 5], wobble: 1 } as any], { id: 'c' }));");
    let report = auto(&path, None, None, true).unwrap();
    assert!(!report.success);
    let lines: Vec<&str> = report.diagnostics.lines().collect();
    for (id, line) in [("a", "film.ts:3:"), ("b", "film.ts:4:"), ("c", "film.ts:5:")] {
        assert!(lines.iter().any(|l| l.contains(line) && l.contains(&format!("(")) && l.contains(&format!("\"{id}\")"))), "{id} missing in\n{}", report.diagnostics);
    }
}

#[test]
fn film_options_cannot_be_assigned_after_comp() {
    let e = error_of("late-fx-path", "(film as any).fx_path = ['../shared'];");
    assert!(e.contains("film.fx_path = ... has no effect") && e.contains("comp({ width, height, fps, fx_path"), "{e}");
    let ok = build(&film("early-fx-path", "film.add(solid('#000'));"), &Options::default()).unwrap();
    assert_eq!(ok["width"], 320);
}
