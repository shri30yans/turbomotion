//! The catalogue entry for the composition key `"effect_space"`. It owns the wording and one runnable composition; the
//! behaviour is in `plan::layer_space` and `render::graph`.

use serde_json::{json, Value};

use crate::schema::EffectSpace;

pub fn entry() -> Value {
    json!({
        "summary": "Where a layer's effects and masks run. \"screen\" (the default) paints every layer at the size of the frame with its transform applied and then runs its effects on that picture, so a 10 px blur is 10 screen pixels at any layer scale and content outside the frame never reaches an effect. \"layer\" is After Effects' order: the layer's own pixels first (the buffer grows by what its effects spread: blur, glow, drop shadow and the other effects that declare bounds), then its masks and effects, then its position, scale and rotation. A blur doubles with the layer's scale, a directional blur turns with it, and a bright shape half outside the frame still blurs into it. Layers without effects or masks, adjustment layers and `look` are unchanged. Effects that read the picture behind the layer (`backdrop`), another layer or the layer's own past (plugin layer and history inputs) and a mesh with effects have no meaning before the transform: `check` refuses them in a layer-space composition, naming the effect. A composition's own `effect_space` overrides the root's. A shader plugin declares how far it spreads with `// @bounds none | expand=<expr> | expand_x=<expr> expand_y=<expr>`; the `fx-needs-bounds` lint warns about a plugin that does not say.",
        "values": EffectSpace::NAMES,
        "default": EffectSpace::default().name(),
        "example": {"composition": {
            "width": 1280, "height": 720, "fps": 30, "dur": 3, "bg": "#10141f", "effect_space": "layer",
            "layers": [{
                "type": "text", "text": "Soft focus", "size": 120, "weight": 800, "color": "#ffffff", "pos": [640, 360], "align": "center",
                "scale": {"from": 1, "to": 1.6, "dur": 2.5, "ease": "sine.inOut"},
                "fx": [{"effect": "blur", "radius": 12}]
            }]
        }},
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_names_the_values_the_parser_reads_and_its_example_asks_for_layer_space() {
        let e = entry();
        assert_eq!(e["values"], json!(["screen", "layer"]));
        assert_eq!(e["default"], "screen");
        assert_eq!(e["example"]["composition"]["effect_space"], "layer");
        let plan = crate::plan::Plan::from_value(e["example"]["composition"].clone(), std::path::Path::new(".")).unwrap();
        assert!(plan.comps[crate::plan::ROOT].layers[0].in_layer_space, "the example's text layer runs its blur in layer space");
    }
}
