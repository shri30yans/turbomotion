//! `sdk watch` re-renders when an imported file changes, and `sdk inspect` reaches inside a shot.
mod common;
use std::{fs, time::Duration};
use turbomotion_script::{cli::Script, watch::{run, Watch}};

#[test]
fn an_edit_to_an_imported_file_renders_the_sheet_again() {
    let dir = common::scratch("watch", "edit");
    fs::write(dir.join("look.ts"), "export const color = '#ff0000';").unwrap();
    fs::write(dir.join("film.ts"), "import {comp,solid} from 'turbomotion';\nimport {color} from './look';\nconst f = comp({width:96,height:54,fps:10,dur:1});\nf.add(solid(color));\nexport default f;").unwrap();
    let script = Script { file: dir.join("film.ts"), shot: None, sdk_dir: None };
    let sheet = dir.join("sheet.png");
    let (look, out) = (dir.join("look.ts"), sheet.clone());
    let editor = std::thread::spawn(move || {
        while !out.exists() { std::thread::sleep(Duration::from_millis(20)); }
        let mut first = fs::read(&out).unwrap();
        loop { std::thread::sleep(Duration::from_millis(150)); let again = fs::read(&out).unwrap(); if again == first { break; } first = again; }
        std::thread::sleep(Duration::from_millis(1100)); // beyond coarse filesystem timestamps
        fs::write(&look, "export const color = '#0000ff';").unwrap();
        first
    });
    run(&Watch { script: &script, sheet: &sheet, times: &[0.2], interval: Duration::from_millis(50), max_renders: Some(2) }).unwrap();
    let first = editor.join().unwrap();
    assert_ne!(first, fs::read(&sheet).unwrap(), "the second render shows the edit");
}

#[test]
fn inspect_reaches_a_layer_inside_a_shot() {
    let dir = common::scratch("watch", "inspect");
    fs::write(dir.join("film.ts"), "import {comp,shot,text} from 'turbomotion';\nconst f = comp({width:200,height:100,fps:30});\nf.timeline({shots:[shot({id:'s4',dur:2,layers:[text('w',{id:'word',pos:[50,50]})]})]});\nexport default f;").unwrap();
    let film = turbomotion_script::build(&dir.join("film.ts"), &Default::default()).unwrap();
    let frame = turbomotion_comp::api::inspect(film, &dir, 0.5, Some("s4/word")).unwrap();
    assert_eq!(frame["layers"][0]["id"], "word");
}
