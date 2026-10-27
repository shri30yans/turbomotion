//! Evaluating shader plugin parameters: each knob becomes plain numbers at a time, ready for the renderer to bind by
//! name, and printable by `inspect`.

use anyhow::{anyhow, Result};
use serde::Serialize;

use super::{salt, Evaluator, LayerEnv};
use crate::{
    anim::prop::Prop,
    fxlib::{
        composite::{Channel, CompositePlan, MaskPlan},
        FxOp, ParamProp, PlanFx, PluginPlan,
    },
    plan::CompPlan,
};

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PluginState {
    pub name: String,
    /// Uniform values by name: floats, vec2 as two numbers, colours as `[r, g, b, a]` in 0..1.
    pub values: Vec<(String, Vec<f32>)>,
}

/// One effect of a chain at a moment: what it does, and how its result is laid over its input.
#[derive(Serialize, Debug, PartialEq)]
pub struct FxState {
    #[serde(flatten)]
    pub op: FxOpState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composite: Option<CompositeState>,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FxOpState {
    /// A platform effect; its numbers are evaluated by the effects library at draw time.
    Builtin,
    Plugin(PluginState),
}

/// The composite options with every value evaluated (`mix` limited to 0..1).
#[derive(Serialize, Debug, PartialEq)]
pub struct CompositeState {
    pub mix: f32,
    pub mask: MaskState,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MaskState {
    Everywhere,
    Alpha,
    Luma { lo: f32, hi: f32, softness: f32, invert: bool },
    Edges { strength: f32, width: f32 },
    /// `layer` is the index of the layer whose picture is the mask.
    Layer { layer: usize, channel: Channel, invert: bool },
}

pub fn chain(ev: &Evaluator, c: &CompPlan, layer: usize, fx: &[PlanFx], t_comp: f64, lt: f64) -> Result<Vec<FxState>> {
    fx.iter()
        .enumerate()
        .map(|(k, f)| {
            let owner = salt::effect(k);
            let op = match &f.op {
                FxOp::Builtin(_) => FxOpState::Builtin,
                FxOp::Plugin(p) => FxOpState::Plugin(plugin(ev, c, layer, p, t_comp, lt, owner)?),
            };
            let composite = f.composite.as_ref().map(|cp| composite(ev, c, layer, cp, t_comp, lt, owner)).transpose()?;
            Ok(FxState { op, composite })
        })
        .collect()
}

/// The composite options of the effect whose salt is `owner`; each option's random stream is named by its path.
fn composite(ev: &Evaluator, c: &CompPlan, layer: usize, plan: &CompositePlan, t_comp: f64, lt: f64, owner: u32) -> Result<CompositeState> {
    let options = salt::named(owner, "composite");
    let at = |p: &Prop, option: &str| -> Result<f32> {
        let env = LayerEnv::new(ev, c, layer, t_comp, 0).with_salt(salt::named(options, option));
        Ok(p.at(lt, &env)? as f32)
    };
    let mix = at(&plan.mix, "mix")?.clamp(0.0, 1.0);
    let mask = match &plan.mask {
        MaskPlan::Everywhere => MaskState::Everywhere,
        MaskPlan::Alpha => MaskState::Alpha,
        MaskPlan::Luma { lo, hi, softness, invert } => MaskState::Luma { lo: at(lo, "mask.lo")?, hi: at(hi, "mask.hi")?, softness: at(softness, "mask.softness")?.max(0.0), invert: *invert },
        MaskPlan::Edges { strength, width } => MaskState::Edges { strength: at(strength, "mask.strength")?.max(0.0), width: at(width, "mask.width")?.max(0.0) },
        MaskPlan::Layer { id, index, channel, invert } => MaskState::Layer { layer: index.ok_or_else(|| anyhow!("composite mask layer {id:?} was not resolved when the plan was made"))?, channel: *channel, invert: *invert },
    };
    Ok(CompositeState { mix, mask })
}

/// The plugin use `p` at a moment; each parameter draws its random numbers from a stream named under `owner`.
pub fn plugin(ev: &Evaluator, c: &CompPlan, layer: usize, p: &PluginPlan, t_comp: f64, lt: f64, owner: u32) -> Result<PluginState> {
    let def = ev.plan.fx.get(&p.name)?;
    let mut values = Vec::with_capacity(p.params.len());
    // the plan lists a use's parameters in the definition's order, so each has the declaration it was parsed against
    for ((name, prop), declared) in p.params.iter().zip(&def.params) {
        let env = LayerEnv::new(ev, c, layer, t_comp, 0).with_salt(salt::named(owner, name));
        let v: Vec<f32> = match prop {
            ParamProp::Num(n) => vec![n.at(lt, &env)?.clamp(declared.min.unwrap_or(f64::MIN), declared.max.unwrap_or(f64::MAX)) as f32],
            ParamProp::Vec(v) => v.at(lt, &env)?.into_iter().map(|x| x as f32).collect(),
            ParamProp::Col(col) => {
                let [r, g, b, a] = col.at(lt, &env)?;
                vec![r / 255.0, g / 255.0, b / 255.0, a]
            }
            ParamProp::Choice(i) => vec![*i as f32],
        };
        values.push((name.clone(), v));
    }
    Ok(PluginState { name: p.name.clone(), values })
}
