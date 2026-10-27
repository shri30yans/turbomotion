//! Building an effect from its parameters, and running a list of them over a layer.

use anyhow::{anyhow, Context, Result};
use serde_json::{Map, Value};
use skia_safe::Canvas;

use super::{resolve_all, Ctx, Effect, Entry, Space, CATALOGUE};
use crate::{layer::Layer, shaders::Shaders};

impl Effect {
    /// The effect `name` with `given` parameters: unknown names and parameters are errors that list the valid ones, a
    /// parameter left out takes its declared default, and values are held inside their declared range.
    pub fn build(name: &str, given: &Map<String, Value>) -> Result<Effect> {
        let entry = entry(name).ok_or_else(|| anyhow!("no platform effect {name:?}; they are: {}", CATALOGUE.iter().map(|e| e.effect).collect::<Vec<_>>().join(", ")))?;
        let resolved = resolve_all(name, entry.params, given)?;
        let effect: Effect = serde_json::from_value(Value::Object(resolved)).with_context(|| format!("effect {name:?}"))?;
        effect.check().with_context(|| format!("effect {name:?}"))?;
        Ok(effect)
    }
}

/// The catalogue entry called `name`.
pub fn entry(name: &str) -> Option<&'static Entry> {
    CATALOGUE.iter().find(|e| e.effect == name)
}

/// Run `chain` over `layer` at time `t`, in order. The picture is the whole layer.
pub fn apply(shaders: &Shaders, like: &Canvas, layer: Layer, chain: &[Effect], t: f64) -> Result<Layer> {
    run_chain(&Ctx::new(shaders, like, t), layer, chain)
}

/// As [`apply`] for a picture that is a window onto the layer (see [`Space`]).
pub fn apply_in(shaders: &Shaders, like: &Canvas, layer: Layer, chain: &[Effect], t: f64, space: Space) -> Result<Layer> {
    run_chain(&Ctx { space: Some(space), ..Ctx::new(shaders, like, t) }, layer, chain)
}

fn run_chain(ctx: &Ctx, layer: Layer, chain: &[Effect]) -> Result<Layer> {
    chain.iter().try_fold(layer, |layer, effect| effect.run(ctx, &layer).with_context(|| format!("effect {:?}", effect.name())))
}
