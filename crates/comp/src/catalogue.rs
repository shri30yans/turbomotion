//! The catalogue: everything a composition can contain, with a complete example for each, printed by
//! `turbomotion comp catalogue`. It is the agent's API reference and it cannot drift: a test loads every example
//! through the real planner, so an example that stops working fails the build.

use serde::Serialize;
use serde_json::{json, Value};
use turbomotion_core::effects;

use crate::eval::transit;

#[derive(Serialize)]
pub struct Entry {
    pub name: &'static str,
    pub summary: &'static str,
    /// A layer (for layer types and features) that works as written, to paste into `layers`.
    pub example: Value,
}

pub fn layers() -> Vec<Entry> {
    vec![
        Entry { name: "solid", summary: "A flat rectangle: backgrounds, flashes, mattes. Size defaults to the composition.", example: json!({"type": "solid", "color": "#101828"}) },
        Entry {
            name: "shape",
            summary: "Vector shapes with fill, gradient, stroke, dashes, trim (draw-on), morph and repeat. Geometry: rect, ellipse, polygon, star, d (SVG path), points.",
            example: json!({"type": "shape", "pos": [960, 540], "items": [
                {"rect": [-200, -120, 400, 240], "radius": 24, "fill": {"gradient": {"type": "linear", "from": [0, -120], "to": [0, 120], "stops": [[0, "#ff8a00"], [1, "#e52e71"]]}}},
                {"ellipse": [0, 0, 90, 90], "stroke": {"color": "#ffffff", "width": 8, "cap": "round"}, "trim": {"end": {"from": 0, "to": 1, "at": 0.3, "dur": 1, "ease": "power3.inOut"}}}
            ]}),
        },
        Entry {
            name: "null",
            summary: "An invisible transform to parent other layers to: one handle moves, scales or shakes a whole group.",
            example: json!({"type": "null", "id": "rig", "pos": [960, 540], "rotation": {"from": -4, "to": 4, "dur": 2, "ease": "sine.inOut"}}),
        },
        Entry {
            name: "precomp",
            summary: "Plays a nested composition from `comps` as one layer, with its own clock (offset, speed, time remap, loop). Everything a layer can do (effects, masks, mattes, 3D) works on it.",
            example: json!({"type": "precomp", "comp": "inner", "start": 0.5, "speed": 1, "pos": [960, 540], "scale": {"from": 0.8, "to": 1, "dur": 0.6, "ease": "expo.out"}}),
        },
        Entry {
            name: "adjustment",
            summary: "Applies its effects to everything below it (a grade, a glitch, a blur over the whole picture). Masks limit it to a region; opacity fades it.",
            example: json!({"type": "adjustment", "fx": [{"effect": "chromatic_aberration", "amount": 4}, {"effect": "vignette", "strength": 0.4}]}),
        },
        Entry {
            name: "texture",
            summary: "Paints a generator shader over the layer (noise, clouds, cells, stripes, grid, gradient, light_leak, or your own `effects/<name>.sksl` with `@kind generator`). Use it as a background, a matte, or blended over footage. `comp fx list` shows each one's parameters.",
            example: json!({"type": "texture", "shader": "clouds", "params": {"scale": 300, "high": "#9aa7c7"}, "blend": "screen", "opacity": 0.6}),
        },
        Entry {
            name: "particles",
            summary: "A particle system where every particle is a closed-form function of its age (so any frame renders alone). The emitter sits at the layer's position. Emit continuously (`rate`) and/or in `burst`s; `prewarm` starts it already running. Shapes: soft, circle, square, spark (streak along travel), star, ring. Use blend `add` or `screen` for light.",
            example: json!({"type": "particles", "pos": [960, 900], "blend": "add", "rate": 90, "life": [1, 2.5], "speed": [180, 420], "angle": -90, "spread": 40, "gravity": [0, 160], "drag": 0.4,
                "size": [4, 10], "shape": "spark", "trail": 3, "colors": [[0, "#ffffff"], [0.4, "#ffd36b"], [1, "#ff4a3d"]], "opacity_over_life": [[0, 1], [0.8, 1], [1, 0]]}),
        },
        Entry {
            name: "pack",
            summary: "Graphics owned by another crate and registered by the host (a globe, a map, a chart): {\"type\": \"pack\", \"name\": \"<pack>\", \"params\": {...}}. `comp catalogue` lists the packs this build has under `packs`. A pack paints a picture from its params and the layer's time; everything else (transform, effects, masks, mattes, 3D) works on it like any layer.",
            example: json!({"type": "solid", "color": "#000"}),
        },
        Entry {
            name: "camera",
            summary: "A 3D camera (the topmost active one is used). Layers with \"three_d\": true are projected through it. Default: 1:1 at z = 0.",
            example: json!({"type": "camera", "zoom": 1800, "pos": [{"from": 760, "to": 1160, "dur": 4, "ease": "sine.inOut"}, 540, -1800], "poi": [960, 540, 0]}),
        },
        Entry {
            name: "light",
            summary: "A light for 3D layers: ambient, point, directional or spot. 3D layers are shaded by every active light.",
            example: json!({"type": "light", "kind": "point", "pos": [400, 200, -500], "color": "#ffd9a0", "intensity": 1.2, "radius": 1400}),
        },
    ]
}

/// The enter / exit presets with a one-line description.
pub fn transitions() -> Vec<Value> {
    transit::PRESETS.iter().map(|(n, d)| json!({"preset": n, "summary": d, "example": {"enter": {"preset": n, "dur": 0.5}}})).collect()
}

pub fn eases() -> Vec<&'static str> {
    vec![
        "none", "power1.in/out/inOut", "power2.*", "power3.*", "power4.*", "sine.*", "expo.*", "circ.*", "back.*(overshoot)", "elastic.*(amplitude, period)", "bounce.*", "steps(n)",
        "cubic(x1, y1, x2, y2)", "spring(damping_ratio)", "smooth", "wiggle(n)", "rough(strength, points, seed)", "slow(ratio, power)", "hold",
    ]
}

pub fn expression_functions() -> Value {
    json!({
        "variables": ["time (layer seconds)", "comp_time", "value (the keyframed value)", "index", "fps", "frame", "width", "height", "dur", "progress (0..1 of the layer's life)", "bpm", "pi", "tau"],
        "math": ["sin", "cos", "tan", "asin", "acos", "atan", "atan2", "abs", "min", "max", "floor", "ceil", "round", "sqrt", "pow", "exp", "ln", "sign", "fract", "mod", "step", "pingpong", "clamp(x, lo, hi)", "lerp(a, b, t)", "radians", "degrees"],
        "motion": ["linear(x, x0, x1, y0, y1)", "smooth(x0, x1, x)", "ease(\"expo.out\", x, x0, x1, y0, y1)", "wiggle(freq, amp, octaves?)", "noise(x)", "noise2(x, y)", "random(seed)"],
        "rhythm": ["beat() 0..1 within the beat", "bar() 0..1 within the bar", "beat_index()", "marker(\"name\") seconds since the marker", "amp(\"audio layer id\") loudness 0..1", "onset(\"audio layer id\") seconds since the latest kick or hit"],
        "layers": ["ref(\"layer id\", \"x|y|z|rotation|opacity|scale|sx|sy\")"],
        "operators": "+ - * / % ^ == != < <= > >= && || ! and a ? b : c",
    })
}

pub fn effects_catalogue() -> Vec<effects::Entry> {
    effects::catalogue()
}

fn shader_plugins() -> Value {
    let lib = crate::fxlib::Library::load(std::path::Path::new("")).expect("the built-in pack compiles");
    Value::Array(lib.defs().map(|d| json!({"name": d.name, "kind": d.kind, "summary": d.summary, "params": d.params.iter().map(|p| format!("{} {:?} = {:?}", p.name, p.ty, p.default)).collect::<Vec<_>>()})).collect())
}

pub fn all() -> Value {
    json!({
        "layers": layers(),
        "enter_exit_presets": transitions(),
        "eases": eases(),
        "expressions": expression_functions(),
        "effects": effects_catalogue(),
        "shader_plugins": shader_plugins(),
        "packs": crate::pack::catalogue().into_iter().map(|(n, s)| json!({"name": n, "summary": s})).collect::<Vec<_>>(),
        "make_a_new_effect": "Write effects/<name>.sksl beside comp.json: header comments `// @effect name`, `// @kind effect|generator`, `// @summary ...`, `// @param name float|vec2|color default [min max] -- doc`, then `half4 main(float2 xy)`. You get `image` (effects), `resolution`, `time` and a library (hash21, vnoise, fbm, worley, rot2, luma, hsv2rgb, straight, premul). Try it with `comp fx test <name> --out sheet.png`; then use it like any effect: {\"effect\": \"name\", \"param\": value}.",
        "time_literals": ["1.5s", "45f", "beat:8", "bar:2", "marker:drop", "title.end", "title.start", "prev.end", "comp.end", "any of those followed by +0.25 or -3f"],
        "property_forms": {
            "constant": 40,
            "expression": "wiggle(2, 30)",
            "keys": [[0, 0], [0.6, 100, "expo.out"], [2, 100]],
            "tween": {"from": 0, "to": 100, "at": 1, "dur": 0.6, "ease": "back.out(1.4)"},
            "loop": {"keys": [[0, 0], [1, 360, "none"]], "loop": "cycle"},
            "keys_plus_expression": {"value": [[0, 0], [1, 100]], "expr": "value + wiggle(3, 5)"}
        },
        "colour_forms": ["#rrggbb", "#rrggbbaa", "#rgb", "white", [255, 128, 0], [255, 128, 0, 0.5], {"from": "#000", "to": "#f00", "dur": 1}],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::Plan;

    fn load(layers: Vec<Value>) -> anyhow::Result<Plan> {
        Plan::from_value(json!({"dur": 5, "comps": {"inner": {"dur": 2, "layers": []}}, "layers": layers}), std::path::Path::new("."))
    }

    #[test]
    fn every_layer_example_loads() {
        for e in layers() {
            load(vec![e.example.clone()]).unwrap_or_else(|err| panic!("{}: {err:#}", e.name));
        }
    }

    #[test]
    fn every_enter_exit_preset_example_loads() {
        for p in transitions() {
            load(vec![json!({"type": "solid", "color": "#fff", "enter": p["example"]["enter"], "exit": p["example"]["enter"]})]).unwrap_or_else(|err| panic!("{}: {err:#}", p["preset"]));
        }
    }

    #[test]
    fn every_ease_name_parses() {
        for e in ["none", "power1.in", "power4.inOut", "sine.out", "expo.in", "circ.out", "back.out(1.7)", "elastic.out(1, 0.4)", "bounce.inOut", "steps(4)", "cubic(0.2, 0, 0, 1)", "spring(0.5)", "smooth", "wiggle(5)", "rough(1, 20, 3)", "slow(0.7, 0.7)", "hold"] {
            crate::anim::ease::Ease::parse(e).unwrap_or_else(|err| panic!("{e}: {err:#}"));
        }
        let _ = crate::anim::expr::Expr::parse("wiggle(2, 30)").unwrap();
    }
}
