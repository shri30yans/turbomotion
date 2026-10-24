//! Evaluating shader plugin parameters: each knob becomes plain numbers at a time, ready for the renderer to bind by
//! name, and printable by `inspect`.

use anyhow::Result;
use serde::Serialize;

use super::{Evaluator, LayerEnv};
use crate::{
    fxlib::{ParamProp, PlanFx, PluginPlan, Ty},
    plan::CompPlan,
};

#[derive(Serialize, Clone, Debug)]
pub struct PluginState {
    pub name: String,
    /// Uniform values by name: floats, vec2 as two numbers, colours as `[r, g, b, a]` in 0..1.
    pub values: Vec<(String, Vec<f32>)>,
}

#[derive(Serialize, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FxState {
    /// A platform effect; its numbers are evaluated by the effects library at draw time.
    Builtin,
    Plugin(PluginState),
}

pub fn chain(ev: &Evaluator, c: &CompPlan, layer: usize, fx: &[PlanFx], t_comp: f64, lt: f64) -> Result<Vec<FxState>> {
    fx.iter()
        .enumerate()
        .map(|(k, f)| match f {
            PlanFx::Builtin(_) => Ok(FxState::Builtin),
            PlanFx::Plugin(p) => plugin(ev, c, layer, p, t_comp, lt, 100 + 8 * k as u32).map(FxState::Plugin),
        })
        .collect()
}

pub fn plugin(ev: &Evaluator, c: &CompPlan, layer: usize, p: &PluginPlan, t_comp: f64, lt: f64, salt: u32) -> Result<PluginState> {
    let def = ev.plan.fx.get(&p.name)?;
    let mut values = Vec::with_capacity(p.params.len());
    for (k, (name, prop)) in p.params.iter().enumerate() {
        let env = LayerEnv::new(ev, c, layer, t_comp, 0).with_salt(salt + k as u32);
        let meta = def.params.iter().find(|m| &m.name == name);
        let v: Vec<f32> = match (prop, meta.map(|m| m.ty)) {
            (ParamProp::Num(n), _) => {
                let mut x = n.at(lt, &env)?;
                if let Some(m) = meta {
                    x = x.clamp(m.min.unwrap_or(f64::MIN), m.max.unwrap_or(f64::MAX));
                }
                vec![x as f32]
            }
            (ParamProp::Vec(v), _) => v.at(lt, &env)?.into_iter().map(|x| x as f32).collect(),
            (ParamProp::Col(col), Some(Ty::Color) | None) => {
                let [r, g, b, a] = col.at(lt, &env)?;
                vec![r / 255.0, g / 255.0, b / 255.0, a]
            }
            (ParamProp::Col(_), Some(_)) => unreachable!("planned against the definition"),
        };
        values.push((name.clone(), v));
    }
    Ok(PluginState { name: p.name.clone(), values })
}
