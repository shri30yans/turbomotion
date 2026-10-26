//! The catalogue entry for the composition key `"color"`. It owns the wording and one runnable composition; the
//! behaviour is in `schema::color` (the key), `turbomotion_core::color` (the conversions) and `render` (where the
//! surfaces are tagged).

use serde_json::{json, Value};

use crate::schema::color::{Depth, Gamut};

pub fn entry() -> Value {
    json!({
        "summary": "What the numbers of a composition's pictures mean. Left out, nothing changes: every picture, colour and effect works on display-encoded sRGB numbers as it always did, and the render is byte-for-byte what it was. With `linear_light`, pictures are kept as light: blends, blurs, glows, gradients and resampling are computed on light, so 50% white over black is the mid-grey a lamp at half power gives (level 188 on screen, not 128), a blur across a black/white edge is half as bright as the pair on average (188, not 128), and an additive blend adds light. Colour literals, gradients, images and video are converted into the working profile as they are drawn, and the finished frame is converted to the output profile when it leaves, with alpha kept as coverage throughout (it is never encoded). Effects that move or spread light (blur, glow, shadow, exposure, the warps) run in light; effects written against what the eye sees (brightness, contrast, hue, tint, grain, grade) always run on display numbers, converted in and out around them, so their sliders feel the same in either mode. A shader plugin says which it wants with `// @space display|linear`. A nested composition takes the root's values field by field unless it says its own; `space` is the finished frame's profile, so only the root has it. The encoder tags the file with the output profile, so a player shows the colours the film was made with.",
        "keys": {
            "linear_light": "true keeps pictures in linear light (default false: display-encoded numbers).",
            "working_profile": format!("The gamut pictures are kept in: {} (default srgb). Images with an embedded profile and video tagged with their primaries keep their colours when it is wider than the file's.", names(&Gamut::NAMES)),
            "space": format!("Root only. The gamut the finished frame is encoded in, with the sRGB curve, and the file is tagged with: {} (default srgb).", names(&Gamut::NAMES)),
            "depth": format!("Numeric depth of every picture: {} (half-float, the only one, and what the engine always used).", names(&Depth::NAMES)),
        },
        "example": {"composition": {
            "width": 1280, "height": 720, "fps": 30, "dur": 2, "bg": "#000000", "color": {"linear_light": true},
            "layers": [
                {"type": "solid", "color": "#ffffff", "size": [640, 720], "pos": [320, 360]},
                {"type": "solid", "color": "#ffffff", "size": [1280, 720], "opacity": 0.5, "fx": [{"effect": "blur", "radius": 24}]}
            ]
        }},
    })
}

fn names(names: &[&str]) -> String {
    names.iter().map(|n| format!("`{n}`")).collect::<Vec<_>>().join(" or ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_names_the_keys_the_parser_reads_and_its_example_loads_as_linear_light() {
        let e = entry();
        let keys: Vec<&str> = e["keys"].as_object().unwrap().keys().map(String::as_str).collect();
        let parsed: crate::schema::ColorBlock = serde_json::from_value(json!({"linear_light": true, "working_profile": "srgb", "space": "srgb", "depth": "half"})).unwrap();
        assert_eq!(parsed.depth, Depth::Half);
        for key in ["linear_light", "working_profile", "space", "depth"] {
            assert!(keys.contains(&key), "the entry lacks {key}");
        }
        let plan = crate::plan::Plan::from_value(e["example"]["composition"].clone(), std::path::Path::new(".")).unwrap();
        assert!(plan.comps[crate::plan::ROOT].color.is_managed());
    }
}
