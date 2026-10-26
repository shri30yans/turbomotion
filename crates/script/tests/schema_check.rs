//! `sdk check` with no TypeScript compiler: validate what the film emitted and point at the script line.
mod common;
use std::fs;
use turbomotion_script::check::auto;

fn film(name: &str, source: &str) -> std::path::PathBuf {
    let dir = common::scratch("schema-check", name);
    fs::write(dir.join("film.ts"), source).unwrap();
    dir.join("film.ts")
}

#[test]
fn a_good_film_passes_and_a_bad_layer_is_mapped_to_the_line_that_made_it() {
    let good = film("good", "import {comp,text} from 'turbomotion';\nconst f = comp({width:200,height:100,fps:30,dur:1});\nf.add(text('hi',{id:'hi',pos:[100,50]}));\nexport default f;");
    let report = auto(&good, None, None, true).unwrap();
    assert!(report.success, "{}", report.diagnostics);
    assert!(report.mode.starts_with("schema"));
    let bad = film("bad", "import {comp,shape} from 'turbomotion';\nconst f = comp({width:200,height:100,fps:30,dur:1});\n\nf.add(shape([{rect:[0,0,5,5],fill:'#fff',wobble:1}],{id:'broken'}));\nexport default f;");
    let report = auto(&bad, None, None, true).unwrap();
    assert!(!report.success);
    assert!(report.diagnostics.contains("film.ts:4:"), "{}", report.diagnostics);
    assert!(report.diagnostics.contains("(shape \"broken\")"), "{}", report.diagnostics);
}

#[test]
fn a_script_error_is_reported_and_an_explicit_missing_compiler_is_an_error() {
    let throws = film("throws", "import {comp} from 'turbomotion';\nthrow new Error('nope');\nexport default comp({width:1,height:1,fps:1});");
    let report = auto(&throws, None, None, true).unwrap();
    assert!(!report.success && report.diagnostics.contains("nope"), "{}", report.diagnostics);
    assert!(auto(&throws, None, Some(std::path::Path::new("/definitely/not/tsc")), false).is_err());
}

#[test]
fn every_text_effect_helper_emits_layers_the_engine_schema_accepts() {
    let source = "import {comp,text,textOnPath,orbitText,wrapText,marquee,charFlip,decode,outlineText} from 'turbomotion';
const f = comp({width:640,height:360,fps:30,dur:3});
f.add(textOnPath('ring', {ellipse:[0,0,100,100], first_margin: 4, travel: 0.25}, {id:'a', pos:[320,180]}));
f.add(orbitText('SPIN ', 100, 6, {id:'b', pos:[320,180]}));
f.add(wrapText('globe', 'sphere', {id:'c', pos:[320,180], radius: 120, turn: 'time * 30', perspective: 'orthographic'}));
f.add(marquee('NEWS', {speed:-80, gap:40, repeat: 4}, {id:'d', pos:[0,60]}));
f.add(text('flip', {id:'e', pos:[320,100], animators:[charFlip({axis:'x', from:-80, z:-200})]}));
f.add(text('DECODE', {id:'g', pos:[320,260], animators:[decode({charset:'matrix', seed:5})]}));
f.add(outlineText('draw', {id:'h', pos:[320,300]}));
export default f;";
    let good = film("text-fx", source);
    let report = auto(&good, None, None, true).unwrap();
    assert!(report.success, "{}", report.diagnostics);
    let bad = film("text-fx-bad", &source.replace("repeat: 4", "repeat: 0"));
    let report = auto(&bad, None, None, true).unwrap();
    assert!(!report.success && report.diagnostics.contains("nonzero"), "{}", report.diagnostics);
}
