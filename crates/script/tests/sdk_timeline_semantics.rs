//! What a timeline means, checked through the engine: positions are the clock the layers live in whatever their `start`,
//! `set` is an exact step, and per-axis and scalar values of one property merge.
mod common;
use std::fs;
use turbomotion_comp::{eval::{Evaluator, LayerEnv}, plan::{Plan, ROOT}};
use turbomotion_script::{build, Options};

fn film(name: &str, body: &str) -> (Plan, serde_json::Value) {
    let dir = common::scratch("sdk-timeline", name);
    let path = dir.join("film.ts");
    let source = format!("import {{comp,text,timeline}} from 'turbomotion'; const f=comp({{width:200,height:200,fps:30,dur:8}}); {body} export default f;");
    fs::write(&path, source).unwrap();
    let json = build(&path, &Options::default()).unwrap();
    (Plan::from_value(json.clone(), &dir).unwrap(), json)
}

fn x_at(plan: &Plan, t: f64) -> f64 {
    Evaluator::new(plan).frame(ROOT, t).unwrap().layers[0].origin[0]
}

fn scale_at(plan: &Plan, t: f64) -> Vec<f64> {
    let evaluator = Evaluator::new(plan);
    plan.root().layers[0].def.common.scale.as_ref().unwrap().0.at(t, &LayerEnv::new(&evaluator, plan.root(), 0, t, 0)).unwrap()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn timeline_positions_are_shot_time_whatever_the_layer_start() {
    let (plan, _) = film("start-shift", "const a=text('A',{pos:[0,0],start:2,dur:5}); timeline().to(a,{x:100,duration:1,ease:'none'},3); f.add(a);");
    assert!(close(x_at(&plan, 2.5), 0.0));
    assert!(close(x_at(&plan, 3.5), 50.0), "{}", x_at(&plan, 3.5));
    assert!(close(x_at(&plan, 4.5), 100.0));
}

#[test]
fn interrupting_a_tween_on_a_delayed_layer_uses_the_same_clock() {
    let body = "const a=text('A',{pos:[0,0],start:2,dur:5}); const tl=timeline(); tl.to(a,{x:100,duration:1,ease:'none'},3); tl.to(a,{x:200,duration:1,ease:'none',overwrite:'auto'},3.5); f.add(a);";
    let (plan, _) = film("start-overwrite", body);
    assert!(close(x_at(&plan, 3.5), 50.0));
    assert!(close(x_at(&plan, 4.0), 125.0), "{}", x_at(&plan, 4.0));
    assert!(close(x_at(&plan, 4.5), 200.0));
}

#[test]
fn a_start_the_sdk_cannot_read_is_an_error_only_when_the_layer_is_tweened() {
    let dir = common::scratch("sdk-timeline", "string-start");
    let path = dir.join("film.ts");
    let run = |tweened: bool| {
        let tween = if tweened { "timeline().to(a,{x:10,duration:1},0);" } else { "" };
        fs::write(&path, format!("import {{comp,text,timeline}} from 'turbomotion'; const f=comp({{width:100,height:100,fps:30,dur:3}}); const a=text('A',{{pos:[0,0],start:'1s'}}); {tween} f.add(a); export default f;")).unwrap();
        build(&path, &Options::default())
    };
    assert!(run(false).is_ok());
    let message = run(true).unwrap_err().to_string();
    assert!(message.contains("cannot convert to layer time") && message.contains("\"1s\""), "{message}");
}

#[test]
fn set_is_an_exact_step_held_by_the_engine_not_an_epsilon() {
    let (plan, json) = film("set-exact", "const a=text('A',{pos:[0,0],start:0.5,dur:7}); timeline().set(a,{x:100},1); f.add(a);");
    assert!(close(x_at(&plan, 0.9999), 0.0));
    assert!(close(x_at(&plan, 1.0), 100.0));
    assert_eq!(json["layers"][0]["pos"][0]["keys"], serde_json::json!([[0, 0], [0.5, 100, "hold"]]), "layer time of shot second 1 on a layer that starts at 0.5");
}

#[test]
fn a_step_already_over_when_the_layer_starts_is_its_first_value() {
    let (plan, json) = film("set-before-start", "const a=text('A',{pos:[0,0],start:2,dur:5}); timeline().set(a,{x:100},1); f.add(a);");
    assert!(close(x_at(&plan, 2.0), 100.0));
    assert_eq!(json["layers"][0]["pos"][0]["keys"].as_array().unwrap().len(), 1);
}

#[test]
fn a_set_where_a_tween_ends_steps_on_that_instant() {
    let body = "const a=text('A',{pos:[0,0]}); const tl=timeline(); tl.to(a,{x:50,duration:1,ease:'none'},0); tl.set(a,{x:200},1); f.add(a);";
    let (plan, _) = film("set-after-tween", body);
    assert!(close(x_at(&plan, 0.5), 25.0));
    assert!(x_at(&plan, 0.9999) < 50.0);
    assert!(close(x_at(&plan, 1.0), 200.0));
}

#[test]
fn a_scalar_tween_on_a_per_axis_scale_applies_to_both_axes() {
    let (plan, _) = film("scale-scalar-on-vector", "const a=text('A',{pos:[0,0],scale:[2,1]}); timeline().from(a,{scale:3,duration:1,ease:'none'},0); f.add(a);");
    assert_eq!(scale_at(&plan, 0.0), vec![3.0, 3.0]);
    assert_eq!(scale_at(&plan, 0.5), vec![2.5, 2.0]);
    assert_eq!(scale_at(&plan, 1.0), vec![2.0, 1.0]);
}

#[test]
fn a_per_axis_tween_on_a_scalar_scale_starts_equal_on_both_axes() {
    let (plan, _) = film("scale-vector-on-scalar", "const a=text('A',{pos:[0,0],scale:2}); timeline().to(a,{scale:[4,6],duration:1,ease:'none'},0); f.add(a);");
    assert_eq!(scale_at(&plan, 0.0), vec![2.0, 2.0]);
    assert_eq!(scale_at(&plan, 1.0), vec![4.0, 6.0]);
}

#[test]
fn scale_lists_of_different_lengths_are_named_in_the_error() {
    let dir = common::scratch("sdk-timeline", "scale-lengths");
    let path = dir.join("film.ts");
    fs::write(&path, "import {comp,text,timeline} from 'turbomotion'; const f=comp({width:100,height:100,fps:30,dur:3}); const a=text('A',{pos:[0,0],scale:[2,1]}); timeline().to(a,{scale:[1,2,3],duration:1},0); f.add(a); export default f;").unwrap();
    let message = build(&path, &Options::default()).unwrap_err().to_string();
    assert!(message.contains("lists of different lengths"), "{message}");
}
