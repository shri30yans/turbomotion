//! `effect_space: "layer"`: which layers run their effects and masks on their own pixels, and what cannot.
//!
//! A layer takes the layer-space path when it has anything to run there (effects or masks). One with neither is painted
//! straight through its transform as before: the picture is the same and a second resampling would only soften it.
//! Adjustment layers act on the composite below them, so they stay in screen space whatever the key says.

use anyhow::{anyhow, Result};

use crate::{
    fxlib::{composite::MaskPlan, FxOp, Library, PlanFx},
    schema::{EffectSpace, Kind},
};

/// Whether a layer with this kind, effects and masks runs them in layer space.
pub fn applies(space: EffectSpace, kind: &Kind, fx: &[PlanFx], masks: usize) -> bool {
    space == EffectSpace::Layer && !matches!(kind, Kind::Adjustment) && (!fx.is_empty() || masks > 0)
}

/// What stops a layer's effects and masks from running on its own pixels, one message per cause, naming the effect.
/// Empty when the layer can take the layer-space path.
pub fn problems(kind: &Kind, fx: &[PlanFx], masks: usize, lib: &Library) -> Vec<String> {
    let mut out = vec![];
    if matches!(kind, Kind::Mesh(_)) && (!fx.is_empty() || masks > 0) {
        out.push("a mesh is projected vertex by vertex and has no flat picture of its own, so its effects and masks cannot run in layer space; use \"effect_space\": \"screen\" or put the effects on a precomp that holds the mesh".to_string());
    }
    for (k, f) in fx.iter().enumerate() {
        if let Err(e) = effect_problem(f, lib) {
            out.push(format!("fx {k} ({}): {e}", f.name()));
        }
    }
    out
}

fn effect_problem(f: &PlanFx, lib: &Library) -> Result<()> {
    if f.composite.as_ref().is_some_and(|c| matches!(c.mask, MaskPlan::Layer { .. })) {
        return Err(anyhow!("its composite mask is another layer's picture, which lives in screen space; use \"effect_space\": \"screen\" for this layer's effects"));
    }
    match &f.op {
        FxOp::Builtin(e) => match e.needs_screen() {
            Some(why) => Err(anyhow!("{why}; use \"effect_space\": \"screen\"")),
            None => Ok(()),
        },
        FxOp::Plugin(p) => {
            let def = lib.get(&p.name)?;
            if let Some(i) = def.inputs.first() {
                return Err(anyhow!("it reads {:?}, another layer's picture or its own past frames, which have no layer-space form yet; use \"effect_space\": \"screen\" for this layer's effects", i.name));
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::plan::Plan;

    fn plan(layers: serde_json::Value, space: &str) -> anyhow::Result<Plan> {
        Plan::from_value(json!({"width": 64, "height": 48, "dur": 1, "effect_space": space, "layers": layers}), std::path::Path::new("."))
    }

    fn error(layers: serde_json::Value) -> String {
        format!("{:#}", plan(layers, "layer").err().expect("rejected in layer space"))
    }

    #[test]
    fn only_layers_with_effects_or_masks_take_the_layer_space_path() {
        let p = plan(
            json!([
                {"type": "solid", "color": "#fff", "size": [8, 8]},
                {"type": "solid", "color": "#fff", "size": [8, 8], "fx": [{"effect": "blur", "radius": 2}]},
                {"type": "solid", "color": "#fff", "size": [8, 8], "masks": [{"path": {"rect": [0, 0, 4, 4]}}]},
                {"type": "adjustment", "fx": [{"effect": "blur", "radius": 2}]}
            ]),
            "layer",
        )
        .unwrap();
        let in_layer: Vec<bool> = p.root().layers.iter().map(|l| l.in_layer_space).collect();
        assert_eq!(in_layer, [false, true, true, false]);
        let screen = plan(json!([{"type": "solid", "color": "#fff", "size": [8, 8], "fx": [{"effect": "blur", "radius": 2}]}]), "screen").unwrap();
        assert!(!screen.root().layers[0].in_layer_space);
    }

    #[test]
    fn a_nested_comp_takes_the_roots_space_unless_it_says_its_own() {
        let p = Plan::from_value(
            json!({"effect_space": "layer", "comps": {"a": {"layers": []}, "b": {"effect_space": "screen", "layers": []}}, "layers": [{"type": "precomp", "comp": "a"}, {"type": "precomp", "comp": "b"}]}),
            std::path::Path::new("."),
        )
        .unwrap();
        use crate::schema::EffectSpace::{Layer, Screen};
        assert_eq!((p.root().effect_space, p.comps["a"].effect_space, p.comps["b"].effect_space), (Layer, Layer, Screen));
    }

    #[test]
    fn backdrop_is_refused_in_layer_space_and_says_why() {
        let e = error(json!([{"type": "solid", "color": "#fff", "fx": [{"effect": "backdrop", "blur": 4}]}]));
        assert!(e.contains("fx 0 (backdrop)") && e.contains("composited behind the layer") && e.contains("\"effect_space\": \"screen\""), "{e}");
    }

    #[test]
    fn a_mesh_with_effects_is_refused_in_layer_space() {
        let e = error(json!([{"type": "mesh", "geometry": {"kind": "text", "text": "M", "size": 60, "bevel": 2}, "fx": [{"effect": "blur", "radius": 2}]}]));
        assert!(e.contains("mesh") && e.contains("layer space"), "{e}");
    }

    #[test]
    fn another_layers_picture_as_a_composite_mask_is_refused() {
        let e = error(json!([
            {"type": "solid", "id": "m", "color": "#fff", "size": [8, 8]},
            {"type": "solid", "color": "#fff", "fx": [{"effect": "blur", "radius": 2, "composite": {"mask": {"kind": "layer", "layer": "m"}}}]}
        ]));
        assert!(e.contains("composite mask") && e.contains("screen space"), "{e}");
    }

    #[test]
    fn a_plugin_that_reads_another_layer_is_refused_and_one_that_does_not_is_accepted() {
        let e = error(json!([
            {"type": "solid", "id": "m", "color": "#fff", "size": [8, 8]},
            {"type": "solid", "color": "#fff", "fx": [{"effect": "displacement_map", "map": "m"}]}
        ]));
        assert!(e.contains("fx 0 (displacement_map)") && e.contains("\"map\"") && e.contains("another layer"), "{e}");
        assert!(plan(json!([{"type": "solid", "color": "#fff", "fx": [{"effect": "duotone"}]}]), "layer").is_ok());
    }

    #[test]
    fn the_same_composition_is_fine_in_screen_space() {
        assert!(plan(json!([{"type": "solid", "color": "#fff", "fx": [{"effect": "backdrop", "blur": 4}]}]), "screen").is_ok());
    }
}
