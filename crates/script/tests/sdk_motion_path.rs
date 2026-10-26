//! Motion paths and keyframe handles written in the SDK: they come out as the engine's JSON, the engine reads that JSON
//! back, and the numbers it evaluates are After Effects' (closed forms and measured speeds, not pictures).
mod common;
use std::fs;
use turbomotion_comp::{eval::Evaluator, plan::{Plan, ROOT}};
use turbomotion_script::{build, Options};

fn film(name: &str, body: &str) -> Result<(Plan, serde_json::Value), String> {
    let dir = common::scratch("sdk-motion-path", name);
    let path = dir.join("film.ts");
    let source = format!("import {{comp,text,solid,timeline,motionPath}} from 'turbomotion'; const f=comp({{width:400,height:400,fps:30,dur:8}}); {body} export default f;");
    fs::write(&path, source).unwrap();
    let json = build(&path, &Options::default()).map_err(|e| format!("{e:#}"))?;
    let plan = Plan::from_value(json.clone(), &dir).map_err(|e| format!("{e:#}"))?;
    Ok((plan, json))
}

fn ok(name: &str, body: &str) -> (Plan, serde_json::Value) {
    film(name, body).unwrap_or_else(|e| panic!("{e}"))
}

fn origin(plan: &Plan, t: f64) -> [f64; 2] {
    Evaluator::new(plan).frame(ROOT, t).unwrap().layers[0].origin
}

fn opacity(plan: &Plan, t: f64) -> f64 {
    Evaluator::new(plan).frame(ROOT, t).unwrap().layers[0].opacity
}

fn heading(plan: &Plan, t: f64) -> f64 {
    Evaluator::new(plan).local_props(plan.root(), &plan.root().layers[0], t, 0).unwrap().pose.heading
}

#[test]
fn a_motion_path_round_trips_from_the_sdk_through_json_into_the_engine() {
    let body = "f.add(solid('#fff',{size:[10,10],anchor:[0,0],pos:motionPath([{t:0,v:[50,50],spatial:'bezier',out:[100,0],ease:'none'},{t:2,v:[250,150],spatial:'bezier',in:[-100,0],ease:'none'}],{auto_orient:true})}));";
    let (plan, json) = ok("round-trip", body);
    let pos = &json["layers"][0]["pos"];
    assert_eq!(pos["auto_orient"], "path");
    assert_eq!(pos["keys"][0]["out"], serde_json::json!([100, 0]));
    assert_eq!(pos["keys"][1]["t"], 2);
    assert_eq!(origin(&plan, 0.0), [50.0, 50.0]);
    assert_eq!(origin(&plan, 2.0), [250.0, 150.0]);
    let pos = &plan.root().layers[0].def.common.pos.as_ref().unwrap().0;
    assert!(matches!(pos, turbomotion_comp::anim::prop::VecProp::Joined { auto_orient: true, .. }), "{pos:?}");
    // the layer leaves along +x: its heading at the start is 0 degrees
    assert!(heading(&plan, 0.0).abs() < 1e-6, "{}", heading(&plan, 0.0));
}

#[test]
fn a_path_tween_passes_its_points_at_an_even_speed_and_ends_on_time() {
    // three collinear points: the curve is the straight line, 300 px long, so half way through a linear ease is x = 150
    let (plan, json) = ok("path-tween", "const a=solid('#fff',{size:[10,10],anchor:[0,0]}); timeline().to(a,{path:[[0,100],[100,100],[300,100]],duration:2,ease:'none'},1); f.add(a);");
    assert_eq!(json["layers"][0]["pos"]["keys"][1]["roving"], true);
    assert_eq!(origin(&plan, 0.5), [0.0, 100.0], "before the tween the layer waits at the first point");
    for (t, x) in [(1.5, 75.0), (2.0, 150.0), (2.5, 225.0), (3.0, 300.0), (9.0, 300.0)] {
        let p = origin(&plan, t);
        assert!((p[0] - x).abs() < 1e-6 && (p[1] - 100.0).abs() < 1e-9, "t={t}: {p:?} not {x}");
    }
}

#[test]
fn a_path_tween_with_auto_orient_banks_the_layer_along_its_curve() {
    // a half circle through the top: the heading turns from 0 (east) through 90 (south in screen y-down) to 180
    let (plan, _) = ok("bank", "const a=text('A'); timeline().to(a,{path:{points:[[100,200],[200,100],[300,200]],autoOrient:true},duration:2,ease:'none'}); f.add(a);");
    let start = heading(&plan, 0.01);
    let end = heading(&plan, 1.99);
    assert!(start < -30.0 && start > -90.0, "leaves going up and to the right: {start}");
    assert!(end > 30.0 && end < 90.0, "arrives going down and to the right: {end}");
    assert!(heading(&plan, 1.0).abs() < 1e-6, "level at the top: {}", heading(&plan, 1.0));
}

#[test]
fn handles_on_a_tween_are_speed_and_influence_with_easy_ease_as_the_smoothstep() {
    let (plan, json) = ok("handles", "const a=text('A'); timeline().fromTo(a,{opacity:0},{opacity:1,duration:1,handles:{out:'easy',in:'easy'}}); f.add(a);");
    assert_eq!(json["layers"][0]["opacity"]["keys"][0]["ease_out"], "easy");
    assert!((opacity(&plan, 0.25) - (3.0 * 0.0625 - 2.0 * 0.015_625)).abs() < 1e-6, "{}", opacity(&plan, 0.25));
    assert!((opacity(&plan, 0.5) - 0.5).abs() < 1e-9);
    let (plan, _) = ok("handles-speed", "const a=text('A'); timeline().fromTo(a,{opacity:0},{opacity:1,duration:1,handles:{out:{speed:0,influence:100},in:{speed:0,influence:100}}}); f.add(a);");
    assert!((opacity(&plan, 0.5) - 0.5).abs() < 1e-9, "the stalled midpoint of 100/100 influence: {}", opacity(&plan, 0.5));
}

#[test]
fn what_the_engine_would_refuse_is_refused_where_it_was_written() {
    let e = |name: &str, body: &str| film(name, body).err().unwrap_or_else(|| panic!("{name} was accepted"));
    assert!(e("both", "const a=text('A'); timeline().to(a,{opacity:1,ease:'none',handles:{out:'easy'}}); f.add(a);").contains("give one"));
    assert!(e("influence", "const a=text('A'); timeline().to(a,{opacity:0,handles:{out:{speed:0,influence:150}}}); f.add(a);").contains("0 to 100"));
    assert!(e("hold-in", "const a=text('A'); timeline().to(a,{opacity:0,handles:{in:'hold'}}); f.add(a);").contains("cannot hold"));
    assert!(e("two", "const a=text('A'); const t=timeline(); t.to(a,{path:[[0,0],[1,1]]}); t.to(a,{path:[[1,1],[2,2]]},1); f.add(a);").contains("one path tween"));
    assert!(e("clash", "const a=text('A'); timeline().to(a,{path:[[0,0],[1,1]],x:5}); f.add(a);").contains("moves the layer's position"));
    assert!(e("order", "f.add(text('A',{pos:motionPath([{t:2,v:[0,0]},{t:1,v:[5,5]}])}));").contains("go back"));
    assert!(e("roving-end", "f.add(text('A',{pos:motionPath([{t:0,v:[0,0]},{roving:true,v:[5,5]}])}));").contains("between two keys"));
    assert!(e("typo", "f.add(text('A',{pos:{keys:[{t:0,v:[0,0]},{t:1,v:[5,5],ease_ot:'none'}]}}));").contains("Did you mean \"ease_out\""));
}
