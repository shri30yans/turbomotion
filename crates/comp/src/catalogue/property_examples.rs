//! Complete, media-free compositions for the property forms most often guessed incorrectly.
use serde_json::{json, Value};
pub fn examples() -> Vec<Value> {
    let forms = [
        ("constant", json!({"type":"solid","color":"#fff","opacity":0.7})),
        ("scalar tween", json!({"type":"solid","color":"#fff","opacity":{"from":0,"to":1,"at":0.2,"dur":0.8,"ease":"power2.out"}})),
        ("scalar keys", json!({"type":"solid","color":"#fff","opacity":{"keys":[[0,0],[1,1,"none"],[2,0,"sine.inOut"]]}})),
        ("scalar expression", json!({"type":"solid","color":"#fff","rotation":"sin(time*3)*20"})),
        ("loop", json!({"type":"solid","color":"#fff","rotation":{"keys":[[0,0],[1,360,"none"]],"loop":"cycle"}})),
        ("keys plus expression", json!({"type":"solid","color":"#fff","rotation":{"value":[[0,0],[1,90,"none"]],"expr":"value+sin(time*3)*5"}})),
        ("vector tween", json!({"type":"solid","color":"#fff","scale":{"from":[0.8,1.2],"to":[1.2,0.8],"dur":1,"ease":"sine.inOut"}})),
        ("vector keys", json!({"type":"solid","color":"#fff","pos":{"keys":[[0,[100,200]],[1,[400,200],"expo.out"]]}})),
        ("per-axis scale tween", json!({"type":"solid","color":"#fff","scale":[{"from":0.8,"to":1.2,"dur":1},{"keys":[[0,1],[1,0.6,"expo.out"]]}]})),
        ("texture with vector and colour keys", json!({"type":"texture","shader":"aura","params":{"p1":{"keys":[[0,[0.2,0.3]],[2,[0.6,0.5],"sine.inOut"]]},"c2":{"keys":[[0,"#1fb5e0"],[2,"#3be6c0"]]},"drift":0.1}})),
        ("shape item tweens", json!({"type":"shape","pos":[320,180],"items":[{"rect":[-100,-20,200,40],"anchor":[0,0],"scale":[{"from":0.5,"to":1,"dur":1},1],"trim":{"end":{"from":0,"to":1,"dur":1}},"stroke":{"color":"#ffffff","width":4},"fill":{"from":"#2040ff","to":"#ff5030","dur":1}}]})),
        ("glass panel: an adjustment layer limited by a mask blurs what is behind it", json!([
            {"type":"texture","shader":"aura"},
            {"type":"text","text":"Behind glass","font":"Avenir Next","weight":800,"size":90,"color":"#ffffff","pos":[320,180],"align":"center"},
            {"type":"adjustment","masks":[{"path":{"rect":[120,100,400,160],"radius":28}}],"fx":[{"effect":"blur","radius":14},{"effect":"brightness","amount":0.08}]}
        ])),
        ("colour tween", json!({"type":"solid","color":{"from":"#2040ff","to":"#ff5030","dur":1}})),
        ("colour keys", json!({"type":"solid","color":{"keys":[[0,"#2040ff"],[1,"#ff5030","none"],[2,"#ffffff","sine.inOut"]]}})),
        ("colour channel expressions", json!({"type":"solid","color":{"channels":["128+127*sin(time)",64,255,1]}})),
        ("radial gradient", json!({"type":"shape","pos":[320,180],"items":[{"rect":[-200,-100,400,200],"fill":{"gradient":{"type":"radial","center":[0,0],"radius":220,"stops":[[0,"#ffdd80"],[1,"#8040ff"]]}}}]})),
        ("shape item pivot", json!({"type":"shape","pos":[320,180],"items":[{"rect":[0,0,200,100],"anchor":[100,50],"rotation":{"from":-30,"to":30,"dur":1},"fill":"#ffaa40"}]})),
    ];
    forms.into_iter().map(|(name,layer)| json!({"name":name,"composition":{"width":640,"height":360,"fps":30,"dur":3,"layers":if layer.is_array(){layer}else{json!([layer])}}})).collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{eval::Evaluator,plan::{Plan,ROOT}};
    #[test]
    fn every_property_example_loads_and_evaluates() {
        for entry in examples() {
            let plan=Plan::from_value(entry["composition"].clone(),std::path::Path::new(".")).unwrap_or_else(|error|panic!("{}: {error:#}",entry["name"]));
            let evaluator=Evaluator::new(&plan);
            for time in [0.0,0.5,1.5,2.5] {
                evaluator.frame(ROOT,time).unwrap_or_else(|error|panic!("{} at {time}: {error:#}",entry["name"]));
            }
        }
    }
}
