//! A transparent effect-work estimate, not a GPU benchmark or an invented render-time prediction.
use anyhow::Result;
use serde::Serialize;
use crate::{eval::Evaluator, fxlib::{FxOp, PlanFx}, plan::Plan};

/// Megapixels is a presentation unit, not a performance weighting coefficient.
const PIXELS_PER_MEGAPIXEL: f64 = 1_000_000.0;
#[derive(Serialize)]
pub struct Report {
    pub units: &'static str,
    pub method: &'static str,
    pub compositions: Vec<CompositionCost>,
}
#[derive(Serialize)]
pub struct CompositionCost {
    pub comp: String,
    pub layers: usize,
    pub effect_stages: usize,
    pub effect_surface_mpx: f64,
    pub glow_surface_mpx: f64,
}
fn glows(effects: &[PlanFx]) -> usize {
    effects.iter().filter(|fx| matches!(&fx.op, FxOp::Builtin(effect) if effect.name() == "glow")).count()
}
/// Sum each effect stage's largest visible surface from five deterministic local-time samples.
/// Reports every nested composition separately so shot costs do not disappear into a root precomp.
pub fn estimate(plan: &Plan) -> Result<Report> {
    let evaluator = Evaluator::new(plan);
    let mut compositions = Vec::new();
    for (name, comp) in &plan.comps {
        let full = f64::from(comp.width) * f64::from(comp.height);
        let mut surfaces = vec![0.0f64; comp.layers.len()];
        for fraction in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let time = (comp.dur * fraction).min((comp.dur - 1.0/f64::from(plan.fps)).max(0.0));
            for layer in evaluator.frame(name, time)?.layers {
                if layer.visible {
                    let area = layer.bounds.map_or(full, |[x0,y0,x1,y1]| (x1-x0).max(0.0)*(y1-y0).max(0.0));
                    surfaces[layer.index] = surfaces[layer.index].max(area);
                }
            }
        }
        let mut stages = comp.look.len();
        let mut effect_area = full * stages as f64;
        let mut glow_area = full * glows(&comp.look) as f64;
        for (layer, area) in comp.layers.iter().zip(surfaces) {
            stages += layer.fx.len();
            effect_area += area * layer.fx.len() as f64;
            glow_area += area * glows(&layer.fx) as f64;
        }
        compositions.push(CompositionCost { comp: name.clone(), layers: comp.layers.len(), effect_stages: stages,
            effect_surface_mpx: effect_area/PIXELS_PER_MEGAPIXEL, glow_surface_mpx: glow_area/PIXELS_PER_MEGAPIXEL });
    }
    compositions.sort_by(|a,b| b.effect_surface_mpx.total_cmp(&a.effect_surface_mpx).then(a.comp.cmp(&b.comp)));
    Ok(Report { units: "estimated surface megapixels per frame",
        method: "Largest visible bounds at five local-time samples, summed once per effect stage. Glow surface is called out separately. Excludes blur padding, shader complexity, internal passes and GPU/codec timing; nested comps are separate rows, not recursively double-counted.", compositions })
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn larger_glow_surfaces_and_more_effect_stages_rank_higher() {
        let make = |size, effects| Plan::from_value(json!({"width":1000,"height":1000,"dur":1,"layers":[
            {"type":"solid","color":"#fff","size":[size,size],"pos":[500,500],"fx":effects}]}),std::path::Path::new(".")).unwrap();
        let one=json!([{"effect":"glow","radius":20}]);
        let small=estimate(&make(100,one.clone())).unwrap();
        let big=estimate(&make(500,one)).unwrap();
        assert!(big.compositions[0].glow_surface_mpx>small.compositions[0].glow_surface_mpx*20.0);
        let double=estimate(&make(500,json!([{"effect":"glow","radius":20},{"effect":"blur","radius":5}]))).unwrap();
        assert_eq!(double.compositions[0].effect_stages,2);
        assert!((double.compositions[0].effect_surface_mpx-big.compositions[0].effect_surface_mpx*2.0).abs()<1e-9);
    }
}
