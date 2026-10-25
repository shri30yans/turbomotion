//! Numeric fidelity for exact interrupted easing across scalar, vector and colour properties.
mod common;
use std::fs;
use turbomotion_comp::{eval::{Content,Evaluator,LayerEnv},plan::{Plan,ROOT}};
use turbomotion_script::{build,Options};
fn film(source: &str) -> Plan {
    let dir=common::scratch("overwrite-tests","interrupt");
    let path=dir.join("film.ts");
    fs::write(&path,source).unwrap();
    Plan::from_value(build(&path,&Options::default()).unwrap(),&dir).unwrap()
}
#[test]
fn interruption_preserves_original_easing_and_uses_live_values_for_new_tweens() {
    let plan=film("import {comp,text,timeline} from 'turbomotion'; const f=comp({width:500,height:500,fps:30,dur:3}); const a=text('A',{pos:[0,0],scale:[1,2],color:'#000000'}); const tl=timeline(); tl.to(a,{x:100,scale:[3,6],color:'#ffffff',duration:2,ease:'power2.in'},0); tl.to(a,{x:200,scale:[5,10],color:'#ff0000',duration:1,ease:'none',overwrite:'auto'},1); f.add(a); export default f;");
    let evaluator=Evaluator::new(&plan);
    let before=evaluator.frame(ROOT,0.5).unwrap();
    assert!((before.layers[0].origin[0]-1.5625).abs()<1e-9);
    let during=evaluator.frame(ROOT,1.5).unwrap();
    assert!((during.layers[0].origin[0]-106.25).abs()<1e-9);
    let Content::Text(text)=&during.layers[0].content else {panic!("expected text")};
    assert!((text.color[0]-143.4375).abs()<1e-5);
    assert!((text.color[1]-15.9375).abs()<1e-5);
    let scales=plan.root().layers[0].def.common.scale.as_ref().unwrap().0.at(1.5,&LayerEnv::new(&evaluator,plan.root(),0,1.5,0)).unwrap();
    assert_eq!(scales,vec![3.125,6.25]);
    let json=build(&plan.dir.join("film.ts"),&Options::default()).unwrap();
    assert!(json["layers"][0]["scale"][0].as_str().unwrap().contains("ease("));
    let end=evaluator.frame(ROOT,2.5).unwrap();
    assert!((end.layers[0].origin[0]-200.0).abs()<1e-9);
}
#[test]
fn overlap_without_explicit_overwrite_remains_an_error() {
    let dir=common::scratch("overwrite-tests","overlap-error");
    let path=dir.join("film.ts");fs::write(&path,"import {comp,text,timeline} from 'turbomotion'; const f=comp({width:100,height:100,fps:30,dur:3});const a=text('A',{pos:[0,0]});const tl=timeline();tl.to(a,{x:1,duration:2},0);tl.to(a,{x:2,duration:1},1);f.add(a);export default f;").unwrap();
    assert!(build(&path,&Options::default()).unwrap_err().to_string().contains("overlap"));
}
