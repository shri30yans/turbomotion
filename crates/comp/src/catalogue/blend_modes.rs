//! The catalogue entry for blend modes: the layer key `blend` and an effect's `composite.blend`. Every row is read from
//! the engine's table (`turbomotion_core::effects::blend`), so a mode cannot be added without being documented, and each
//! one carries a composition that shows it.

use serde_json::{json, Value};
use turbomotion_core::effects::blend::{Group, Mode, ALIASES, MODES};

pub fn entry() -> Value {
    json!({
        "summary": "How a layer combines with what is below it: the `blend` key of any layer, and `composite.blend` of any effect (the effect's result over its input). Default `normal`. The modes are After Effects': the same names and the same definitions, with every mode's number given by the W3C Compositing and Blending formulas where After Effects follows them. A mode works on the numbers the pictures hold, so a film blends display-encoded sRGB numbers by default (as After Effects does in an 8 or 16-bit project) and blends light with `color: {linear_light: true}`: multiply, screen and overlay then give different pictures from the same layers. The layer's opacity scales its alpha before the mode sees it. Dissolve and dancing dissolve keep each pixel whole or drop it, by a fixed pattern of pixels: the layer's alpha times its opacity is the chance a pixel is kept, the pattern is the same on every render, and the dancing one draws a new pattern every frame. The stencil and silhouette modes cut what is below the layer: a stencil keeps the backdrop where the layer is opaque (or bright) and clears everything else in the frame, including where the layer is not; a silhouette cuts where the layer is. An unknown name is refused with the nearest mode.",
        "default": "normal",
        "groups": Group::ALL.map(Group::name),
        "aliases": ALIASES.iter().map(|(alias, blend)| json!({"alias": alias, "mode": blend.name()})).collect::<Vec<_>>(),
        "modes": MODES.iter().map(mode).collect::<Vec<_>>(),
    })
}

fn mode(m: &Mode) -> Value {
    json!({"name": m.name, "group": m.group.name(), "summary": m.summary, "example": example(m.name)})
}

/// One composition showing a mode: a ground of two colours, with a warm disc laid over it in the mode.
fn example(name: &str) -> Value {
    json!({"composition": {
        "width": 1280, "height": 720, "fps": 30, "dur": 2, "bg": "#101828",
        "layers": [
            {"type": "shape", "items": [{"rect": [0, 0, 1280, 720], "fill": {"gradient": {"type": "linear", "from": [0, 0], "to": [1280, 0], "stops": [[0, "#1d2b64"], [1, "#f8cdda"]]}}}]},
            {"type": "shape", "pos": [640, 360], "blend": name, "opacity": 0.8, "items": [{"ellipse": [0, 0, 280, 280], "fill": "#e8a33d"}]}
        ]
    }})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_lists_every_mode_of_the_table_in_order_each_with_a_composition_that_loads_in_that_mode() {
        let e = entry();
        let names: Vec<&str> = e["modes"].as_array().unwrap().iter().map(|m| m["name"].as_str().unwrap()).collect();
        assert_eq!(names, MODES.map(|m| m.name));
        for m in e["modes"].as_array().unwrap() {
            let plan = crate::plan::Plan::from_value(m["example"]["composition"].clone(), std::path::Path::new(".")).unwrap_or_else(|err| panic!("{}: {err:#}", m["name"]));
            let layer = &plan.comps[crate::plan::ROOT].layers[1];
            assert_eq!(layer.def.common.blend.map(|b| b.name()), m["name"].as_str(), "the example's disc is in the mode it documents");
        }
        assert_eq!(e["aliases"], json!([{"alias": "linear_dodge", "mode": "add"}]));
    }
}
