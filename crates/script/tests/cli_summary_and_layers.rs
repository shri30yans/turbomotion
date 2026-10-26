//! What `sdk build` reports about a film, and how a bare layer id is found inside a shot (the resolver lives in `comp`).
mod common;
use std::fs;
use turbomotion_script::{build, summary::summary};

const FILM: &str = "import {comp,shot,text} from 'turbomotion';
const f = comp({width:200,height:100,fps:30});
f.timeline({shots:[
  shot({id:'intro',dur:2,layers:[text('a',{id:'title',pos:[50,50]}),text('b',{id:'glow',pos:[60,50]})]}),
  shot({id:'outro',dur:2,layers:[text('c',{id:'glow',pos:[70,50]})]})]});
export default f;";

/// `test` names the scratch folder: tests of one process run in parallel and must not write the same `film.ts`.
fn film(test: &str) -> (serde_json::Value, std::path::PathBuf) {
    let dir = common::scratch("cli", &format!("layers-{test}"));
    fs::write(dir.join("film.ts"), FILM).unwrap();
    (build(&dir.join("film.ts"), &Default::default()).unwrap(), dir)
}

#[test]
fn the_summary_counts_the_layers_inside_shots() {
    let (film, _) = film("summary");
    assert!(summary(&film).ends_with("2 shots, 3 layers"), "{}", summary(&film));
}

#[test]
fn a_bare_id_inside_a_shot_is_inspected_and_an_ambiguous_one_is_refused() {
    let (film, dir) = film("bare-id");
    let frame = turbomotion_comp::api::inspect(film.clone(), &dir, 0.5, Some("title")).unwrap();
    assert_eq!(frame["layers"][0]["id"], "title");
    let error = turbomotion_comp::api::inspect(film, &dir, 0.5, Some("glow")).unwrap_err().to_string();
    assert!(error.contains("intro/glow") && error.contains("outro/glow"), "{error}");
}
