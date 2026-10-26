//! Production authoring boundaries: editable SDK modules, film roots, measured text and shot isolation.
use std::{fs, path::{Path, PathBuf}};
use serde_json::json;
use turbomotion_script::{build, Options};

mod common;

fn folder(name: &str) -> PathBuf { common::scratch("authoring", name) }
fn script(name: &str, source: &str) -> PathBuf {
    let path = folder(name).join("film.ts");
    fs::write(&path, source).unwrap();
    path
}
#[test]
fn disk_sdk_loads_new_modules_without_an_embedded_registry_entry() {
    let dir = folder("disk-sdk");
    fs::create_dir_all(dir.join("components")).unwrap();
    fs::write(dir.join("index.ts"), "export { label } from './components/fresh';").unwrap();
    fs::write(dir.join("components/fresh.ts"), "export const label: string = 'first';").unwrap();
    let path = script("disk-film", "import {label} from 'turbomotion'; export default {label};");
    let options = Options { sdk_dir: Some(dir.clone()), ..Options::default() };
    assert_eq!(build(&path, &options).unwrap()["label"], "first");
    fs::write(dir.join("components/fresh.ts"), "export const label: string = 'edited';").unwrap();
    assert_eq!(build(&path, &options).unwrap()["label"], "edited");
    let missing = Options { sdk_dir: Some(dir.join("missing")), ..Options::default() };
    assert!(build(&path, &missing).unwrap_err().to_string().contains("needs index.ts"));
}
#[test]
fn declared_sdk_does_not_mix_in_embedded_modules() {
    let dir = folder("incomplete-sdk");
    fs::write(dir.join("index.ts"), "export {comp} from './comp';").unwrap();
    let path = script("missing-sdk-module", "import {comp} from 'turbomotion'; export default comp({});");
    let error = build(&path, &Options { sdk_dir: Some(dir), ..Options::default() }).unwrap_err();
    assert!(error.to_string().contains("check --sdk-dir"), "{error:#}");
}
#[test]
fn film_assets_resolve_from_script_even_when_json_moves() {
    let path = script("root-film", "import {comp,rect} from 'turbomotion'; const f=comp({width:100,height:100,fps:30,dur:1}); f.add(rect(10,10)); export default f;");
    let value = build(&path, &Options::default()).unwrap();
    let output = folder("moved-output");
    let plan = turbomotion_comp::plan::Plan::from_value(value, &output).unwrap();
    assert_eq!(plan.dir, fs::canonicalize(path.parent().unwrap()).unwrap());
    let bad = turbomotion_comp::plan::Plan::from_value(json!({"root":42}), &output);
    assert!(bad.err().unwrap().to_string().contains("root must be"));
    let relative = turbomotion_comp::plan::Plan::from_value(json!({"root":"assets","dur":1,"layers":[]}), &output).unwrap();
    assert_eq!(relative.dir, output.join("assets"));
}
#[test]
fn measure_uses_renderer_shaping_and_rejects_invalid_options() {
    let path = script("measure", "import {comp,measure} from 'turbomotion'; comp({width:100,height:100,fps:30}); export default {a:measure('Hi',{size:80}),b:measure('Hi there',{size:80}),tracked:measure('Hi',{size:80,tracking:10})};");
    let value = build(&path, &Options::default()).unwrap();
    assert!(value["b"]["width"].as_f64().unwrap() > value["a"]["width"].as_f64().unwrap()*2.0);
    assert!(value["tracked"]["width"].as_f64().unwrap() > value["a"]["width"].as_f64().unwrap());
    assert_eq!(value["b"]["chars"].as_array().unwrap().len(),8);
    let bad = script("bad-measure", "import {measure} from 'turbomotion'; export default measure('x',{size:0});");
    assert!(build(&bad,&Options::default()).unwrap_err().to_string().contains("size"));
}
#[test]
fn vector_and_colour_tweens_load_as_a_composition() {
    let path=script("vector", "import {comp,text,timeline} from 'turbomotion'; const f=comp({width:100,height:100,fps:30,dur:2}); const t=text('Hi',{scale:[1,2],color:'#000000'}); timeline().to(t,{scale:[2,3],color:'#ffffff',duration:1},0); f.add(t); export default f;");
    let value=build(&path,&Options::default()).unwrap();
    assert_eq!(value["layers"][0]["scale"]["from"],json!([1,2]));
    assert_eq!(value["layers"][0]["color"]["to"],"#ffffff");
    turbomotion_comp::plan::Plan::from_value(value,path.parent().unwrap()).unwrap();
}
#[test]
fn explicit_film_root_applies_before_block_and_shader_expansion() {
    let dir=folder("root-with-dependencies");
    let assets=dir.join("assets");
    fs::create_dir_all(assets.join("blocks")).unwrap();
    fs::create_dir_all(assets.join("shaders")).unwrap();
    fs::write(assets.join("blocks/local_card.json"), r##"{"width":100,"height":100,"dur":1,"layers":[{"type":"solid","color":"#fff"}]}"##).unwrap();
    fs::write(assets.join("shaders/local_noise.sksl"), "// @effect local_noise\n// @kind generator\n// @summary root path fixture\nhalf4 main(float2 xy) { return half4(1); }\n").unwrap();
    let path=script("root-with-dependencies", "import {comp,raw} from 'turbomotion'; const f=comp({width:100,height:100,fps:30,dur:1,root:'assets',fx_path:['shaders']}); f.add(raw({type:'precomp',block:'local_card'}),raw({type:'texture',shader:'local_noise'})); export default f;");
    let film=build(&path,&Options::default()).unwrap();
    let plan=turbomotion_comp::plan::Plan::from_value(film,&folder("other-output-directory")).unwrap();
    assert_eq!(plan.dir,fs::canonicalize(&assets).unwrap());
    assert_eq!(plan.root().layers.len(),2);
    assert!(plan.fx.get("local_noise").is_ok());
}
#[test]
fn selected_shot_plans_at_zero_with_matching_overlays() {
    let path=script("shot-plan", "import {comp,shot,text} from 'turbomotion'; const f=comp({width:320,height:240,fps:30}); f.timeline({sfx:false,transition:{type:'dissolve',dur:0.2},shots:[shot({id:'first',dur:2,layers:[text('first')]}),shot({id:'second',start:5,dur:3,layers:[text('second')]})],overlays:[{shot:'second',at:0.2,dur:1,layer:text('overlay')}]}); export default f;");
    let mut film=build(&path,&Options::default()).unwrap();
    turbomotion_script::shot::isolate(&mut film,"second").unwrap();
    let plan=turbomotion_comp::plan::Plan::from_value(film,path.parent().unwrap()).unwrap();
    assert_eq!(plan.root().dur,3.0);
    assert_eq!(plan.root().layers[0].start,0.0);
    assert_eq!(plan.root().layers.len(),2);
    assert_eq!(plan.root().layers[1].start,0.2);
    assert!(!plan.comps.contains_key("shot:first"));
}

#[test]
fn a_stage_glides_only_the_properties_that_differ_and_fades_a_whole_group() {
    let path = script("stage", r#"
import {comp, shape, group, rect, stage, timeline} from 'turbomotion';
const film = comp({width: 400, height: 300, fps: 30, dur: 3});
const a = rect(40, 40, {id: 'a'});
const kid = rect(10, 10, {id: 'kid'});
const g = group({id: 'g'}, [kid]);
const s = stage({a, g}, {
  rest: {a: {x: 100, y: 100}, g: {x: 200, y: 100, opacity: 0}},
  next: {a: {x: 300, y: 100}, g: {opacity: 1}},
}).start('rest');
const tl = timeline();
const end = s.go(tl, 'next', {at: 1, dur: 0.5, ease: {default: 'power2.out', opacity: 'none'}});
film.add(a, g);
export default film;
"#);
    let v = build(&path, &Options::default()).unwrap();
    let layers = v["layers"].as_array().unwrap();
    let by = |id: &str| layers.iter().find(|l| l["id"] == id).unwrap_or_else(|| panic!("{id} in {layers:?}"));
    // `a` moves in x only: y is untouched, so it stays a constant
    assert_eq!(by("a")["pos"][1], 100);
    assert_eq!(by("a")["pos"][0]["to"], 300);
    // a group's opacity does not reach its children in the engine, so the stage fades the child itself
    assert_eq!(by("kid")["opacity"]["to"], 1);
    assert_eq!(by("kid")["opacity"]["ease"], "none");
}

#[test]
fn a_text_stack_places_words_with_shaped_widths_and_glides_earlier_words_when_a_row_grows() {
    let path = script("stack", r#"
import {comp, textStack, timeline} from 'turbomotion';
const film = comp({width: 800, height: 400, fps: 30, dur: 3});
const tl = timeline();
const roles = {key: {size: 80, weight: 800, color: '#fff', upper: true}, tag: {size: 30, color: '#fff'}};
const stack = textStack({tl, roles, pos: [400, 100], width: 700, align: 'center', items: [
  {text: 'so at', at: 0.2, role: 'tag', row: 0},
  {text: 'two', at: 0.6, role: 'key', row: 0},
  {text: 'am', at: 1.0, role: 'key', row: 1},
]});
film.add(...stack.layers);
export default film;
"#);
    let v = build(&path, &Options::default()).unwrap();
    let layers = v["layers"].as_array().unwrap();
    assert_eq!(layers.len(), 3);
    // the first word was alone on its row, then re-centred when "two" joined it: it glides in x (an eased expression)
    let glides = |i: usize, axis: usize| layers[i]["pos"][axis].as_str().is_some_and(|e| e.contains("ease("));
    assert!(glides(0, 0), "{:?}", layers[0]["pos"]);
    // the last word started a new row and nothing arrives after it: it is placed once, below the first row's type
    assert!(!glides(2, 0) && !glides(2, 1));
    assert!(layers[2]["pos"][1].as_f64().unwrap() > 100.0 + 70.0, "{:?}", layers[2]["pos"]);
}

#[test]
fn a_written_composition_records_its_root_relative_to_itself() {
    let path = script("relative-root", "import {comp,rect} from 'turbomotion'; const f=comp({width:100,height:100,fps:30,dur:1,root:'assets'}); f.add(rect(10,10)); export default f;");
    let out = folder("relative-root-out").join("film.comp.json");
    let script = turbomotion_script::cli::Script { file: path.clone(), shot: None, sdk_dir: None };
    turbomotion_script::cli::build_to(&script, Some(out.clone())).unwrap();
    let v: serde_json::Value = serde_json::from_slice(&fs::read(out).unwrap()).unwrap();
    let root = v["root"].as_str().unwrap();
    assert!(!Path::new(root).is_absolute(), "{root}");
    assert!(root.ends_with("/assets") && root.contains("relative-root-"), "{root}");
}

#[test]
fn a_cutout_moves_its_body_by_expression_and_leaves_the_group_free() {
    let path = script("cutout", r#"
import {comp, cutout, shot} from 'turbomotion';
const film = comp({width: 400, height: 600, fps: 30, dur: 2});
const c = cutout({id: 'p', src: 'a.png', size: [200, 300], pivot: 'base', pos: [200, 300],
  idle: {float: {amp: 10, period: 2}, sway: {deg: 4, period: 3}}, shadow: {width: 180, y: 160}, ring: {rx: 120}});
film.add(c);
export default film;
"#);
    let v = build(&path, &Options::default()).unwrap();
    let layers = v["layers"].as_array().unwrap();
    let by = |id: &str| layers.iter().find(|l| l["id"] == id).unwrap_or_else(|| panic!("{id} in {layers:?}"));
    // the group is a plain handle: a stage or timeline can move it without meeting an expression
    assert_eq!(by("p")["type"], "null");
    assert_eq!(by("p")["pos"], json!([200, 300]));
    // the body bobs and rocks by itself; an image anchors from its top-left, so a base pivot is [w/2, h]
    let body = by("p-body");
    assert!(body["pos"][1].as_str().unwrap().contains("sin("), "{}", body["pos"]);
    assert!(body["rotation"].as_str().unwrap().contains("sin("), "{}", body["rotation"]);
    assert_eq!(body["anchor"], json!([100, 300]));
    // children follow the group, and the shadow breathes with the float
    assert_eq!(body["parent"], "p");
    assert!(by("p-shadow")["scale"].as_str().unwrap().contains("sin("));
    assert_eq!(by("p-ring")["parent"], "p");
}

#[test]
fn a_cutout_with_sway_and_tilt_keeps_both_in_one_rotation_triple() {
    let path = script("cutout-both", r#"
import {comp, cutout} from 'turbomotion';
const film = comp({width: 400, height: 600, fps: 30, dur: 2});
film.add(cutout({id: 'p', src: 'a.png', size: [200, 300],
  idle: {sway: {deg: 4, period: 3}, tilt: {x: 6, y: 8, period: 5}}}));
export default film;
"#);
    let v = build(&path, &Options::default()).unwrap();
    let body = v["layers"].as_array().unwrap().iter().find(|l| l["id"] == "p-body").unwrap().clone();
    let r = body["rotation"].as_array().unwrap_or_else(|| panic!("rotation is one triple: {}", body["rotation"]));
    assert_eq!(r.len(), 3);
    assert!(r[0].as_str().unwrap().starts_with("6*"), "X from tilt.x: {r:?}");
    assert!(r[1].as_str().unwrap().starts_with("8*"), "Y from tilt.y: {r:?}");
    assert!(r[2].as_str().unwrap().starts_with("4*"), "Z from sway.deg, not lost: {r:?}");
    assert_eq!(body["three_d"], true);
}

#[test]
fn merging_props_with_a_duplicate_key_throws() {
    let path = script("merge-props", r#"
import {mergeProps} from 'turbomotion';
const ok = mergeProps('ok', {a: 1}, {b: 2});
let msg = '';
try { mergeProps('clash', {rotation: 1}, {rotation: [0, 0, 1]}); } catch (e) { msg = String(e); }
export default {ok, msg};
"#);
    let v = build(&path, &Options::default()).unwrap();
    assert_eq!(v["ok"], json!({"a": 1, "b": 2}));
    assert!(v["msg"].as_str().unwrap().contains("\"rotation\" is set twice"), "{}", v["msg"]);
}
