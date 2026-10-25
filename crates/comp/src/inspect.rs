//! Select an evaluated layer inside nested precompositions without guessing their local time.
use anyhow::{anyhow, bail, Result};
use crate::eval::{Content, Frame, LayerState};

/// Return the layer and its containing frame. Bounds and frame time use that composition's coordinates.
pub fn select<'a>(frame: &'a Frame, path: &str) -> Result<(&'a Frame, &'a LayerState)> {
    // Preserve exact root ids, including ids containing slashes.
    if let Some(layer) = frame.layers.iter().find(|layer| layer.id.as_deref() == Some(path)) {
        return Ok((frame, layer));
    }
    let (id, tail) = path.split_once('/').unwrap_or((path, ""));
    let layer = frame.layers.iter().find(|layer| layer.id.as_deref() == Some(id))
        .ok_or_else(|| anyhow!("no layer {id:?} in comp {:?}; available ids: {}", frame.comp,
            frame.layers.iter().filter_map(|layer| layer.id.as_deref()).collect::<Vec<_>>().join(", ")))?;
    match &layer.content {
        Content::Precomp { frame: Some(inner) } if !tail.is_empty() => select(inner, tail),
        Content::Precomp { frame: None } => bail!("layer {id:?} is inactive at {}s in comp {:?}; choose --t inside its active range", frame.t, frame.comp),
        _ => bail!("layer {id:?} has no nested composition; use its id without /{tail}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{eval::Evaluator, plan::{Plan, ROOT}};
    use serde_json::json;
    #[test]
    fn nested_selection_uses_evaluated_speed_offset_and_local_coordinates() {
        let plan=Plan::from_value(json!({"dur":5,"comps":{"card":{"dur":5,"layers":[{"id":"disc","type":"solid","color":"#fff"}]}},
            "layers":[{"id":"s9","type":"precomp","comp":"card","start":1,"dur":3,"speed":2,"offset":0.5}]}), std::path::Path::new(".")).unwrap();
        let evaluator=Evaluator::new(&plan);
        let frame=evaluator.frame(ROOT,2.0).unwrap();
        let (inner,layer)=select(&frame,"s9/disc").unwrap();
        assert_eq!(inner.comp,"card");
        assert_eq!(inner.t,2.5);
        assert_eq!(layer.id.as_deref(),Some("disc"));
        assert!(select(&frame,"s9/missing").err().unwrap().to_string().contains("disc"));
        let inactive=evaluator.frame(ROOT,0.0).unwrap();
        assert!(select(&inactive,"s9/disc").err().unwrap().to_string().contains("inactive"));
    }
}
