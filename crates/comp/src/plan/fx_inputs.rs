//! Resolving what effects read: the layer ids that plugin `@input … layer` parameters and composite masks name become indices, and
//! everything that would make a layer depend on itself (a matte, a clip, an effect input or mask) is rejected here, once.

use std::collections::HashMap;

use anyhow::{anyhow, bail, ensure, Result};

use super::LayerPlan;
use crate::{
    fxlib::{FxOp, Library, PlanFx},
    schema::Kind,
};

/// Layers with nothing to look at cannot be an effect's picture.
fn has_picture(kind: &Kind) -> bool {
    !matches!(kind, Kind::Audio(_) | Kind::Null | Kind::Camera(_) | Kind::Light(_) | Kind::Adjustment)
}

/// Fill in the layer index of every layer input in `layers`' effects and in the composition's `look`, then reject
/// loops and effects that cannot work where they are.
pub fn resolve(layers: &mut [LayerPlan], look: &mut [PlanFx], ids: &HashMap<String, usize>, lib: &Library) -> Result<()> {
    let known = || {
        let mut v: Vec<_> = ids.keys().cloned().collect();
        v.sort();
        if v.is_empty() { "none (give layers an \"id\")".to_string() } else { v.join(", ") }
    };
    let has_pictures: Vec<bool> = layers.iter().map(|l| has_picture(&l.def.kind)).collect();
    let resolve_one = |owner: Option<usize>, label: &str, fx: &mut [PlanFx]| -> Result<()> {
        for f in fx {
            let name = f.name().to_string();
            for r in f.layer_refs_mut() {
                let &index = ids.get(r.id).ok_or_else(|| anyhow!("{label}: effect {name:?} {} reads layer {:?}, which does not exist; layer ids are {}", r.what, r.id, known()))?;
                ensure!(owner != Some(index), "{label}: effect {name:?} {} reads the layer it is on", r.what);
                ensure!(has_pictures[index], "{label}: effect {name:?} {} reads layer {:?}, which has no picture (audio, null, camera, light and adjustment layers draw nothing)", r.what, r.id);
                *r.index = Some(index);
            }
        }
        Ok(())
    };
    for i in 0..layers.len() {
        let label = layers[i].label.clone();
        resolve_one(Some(i), &label, &mut layers[i].fx)?;
        ensure!(!matches!(layers[i].def.kind, Kind::Adjustment) || !reads_history(&layers[i].fx, lib)?, "{label}: an adjustment layer has no picture of its own to remember, so its effects cannot use history inputs");
    }
    resolve_one(None, "look", look)?;
    ensure!(!reads_history(look, lib)?, "look: a composition's look cannot use history inputs (put the effect on a layer)");
    reject_backdrop(layers, look)?;
    reject_loops(layers)
}

fn reject_backdrop(layers: &[LayerPlan], look: &[PlanFx]) -> Result<()> {
    for l in layers {
        if uses_backdrop(&l.fx) {
            ensure!(
                !matches!(l.def.kind, Kind::Adjustment),
                "{}: the backdrop effect samples what is behind a layer's picture; put it on a shape, solid, text or image layer",
                l.label
            );
        }
    }
    ensure!(
        !uses_backdrop(look),
        "look: the backdrop effect belongs on a layer, not on the composition's look"
    );
    Ok(())
}

fn uses_backdrop(fx: &[PlanFx]) -> bool {
    fx.iter().any(|f| f.name() == "backdrop")
}

fn reads_history(fx: &[PlanFx], lib: &Library) -> Result<bool> {
    for f in fx {
        if let FxOp::Plugin(p) = &f.op {
            if lib.get(&p.name)?.history().is_some() {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// What a layer's picture is built from: its matte, its clip and the layers its effects read.
fn sources(layer: &LayerPlan) -> Vec<usize> {
    layer.matte.map(|(m, _)| m).into_iter().chain(layer.clip_to).chain(layer.fx.iter().flat_map(PlanFx::layers_read)).collect()
}

fn reject_loops(layers: &[LayerPlan]) -> Result<()> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Open,
        Done,
    }
    fn visit(layers: &[LayerPlan], at: usize, marks: &mut HashMap<usize, Mark>, path: &mut Vec<usize>) -> Result<()> {
        match marks.get(&at) {
            Some(Mark::Done) => return Ok(()),
            Some(Mark::Open) => {
                let from = path.iter().position(|&p| p == at).unwrap_or(0);
                let names: Vec<&str> = path[from..].iter().chain([&at]).map(|&p| layers[p].label.as_str()).collect();
                bail!("layers read each other in a loop (matte, clip_to or an effect input): {}", names.join(" -> "));
            }
            None => {}
        }
        marks.insert(at, Mark::Open);
        path.push(at);
        for next in sources(&layers[at]) {
            visit(layers, next, marks, path)?;
        }
        path.pop();
        marks.insert(at, Mark::Done);
        Ok(())
    }
    let mut marks = HashMap::new();
    for i in 0..layers.len() {
        visit(layers, i, &mut marks, &mut vec![])?;
    }
    Ok(())
}
