//! The catalogue: everything a composition can contain, with a complete example for each, printed by
//! `turbomotion comp catalogue`. It is the agent's API reference and it cannot drift: a test loads every example
//! through the real planner, so an example that stops working fails the build.

mod blend_modes;
mod color;
mod effect_space;
mod expression_support;
pub mod layer_keys;
mod motion_keys;
mod palette_cycle;
mod property_examples;
mod property_forms;
mod stroke_taper;
mod text_box;

use serde::Serialize;
use serde_json::{json, Value};

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
            name: "text",
            summary: "Text with real shaping. `font` is a family or a file; `align` left|center|right (the layer's pos is the alignment edge, vertically centred); `width` wraps. Characters the designed font does not have take a fallback face and are painted in the same colour as the rest of the line. `preset` is a named per-character animation (see `text_presets`); `animators` build your own (GSAP staggers or After Effects range selectors). `text` can also be a counter {from, to, at, dur, ease, decimals, sep, prefix, suffix}. Animator props include per-character 3D (`z`, `rotation_x|y|z`, `scale_z`, seen through `perspective`), `fill_hue|saturation|brightness` and `stroke_width`; an animator's `cipher` {charset, rate, seed, settle} decodes noise into the real characters. Animator `props` are DELTAS added to each character's rest state: `opacity: -1` hides it, a positive opacity changes nothing (rest is already full), `scale: -1` shrinks it to nothing. A `selector` {start, end, offset, shape, smooth, random} gives weight 1 to the characters whose place along the text (0 to 1) lies between `start + offset` and `end + offset`, so a window that never reaches a character leaves the text untouched: animate `offset` (for example {from: -0.2, to: 1, dur: 2}) to sweep a window along the line. `check` warns (animator-never-applies, animator-opacity-delta) when an animator can change nothing. `path` {an ellipse/points/d/... geometry or a `layer` id, reverse, perpendicular, force_align, first_margin, last_margin, travel} sets the text along a path (`travel` 0..1 is one exact lap of a closed path); `surface` {kind: cylinder|sphere, radius, turn, back} wraps it on a drum or ball; `marquee` {speed, gap, repeat} scrolls a seamless row; `outline_only` with `stroke_trim` [start, end] draws the glyph outlines on. `box` {fit: \"text\", padding, radius, fill, min_width, align, per_line} draws a rectangle behind the glyphs that is as wide as the characters showing (see `text_box`).",
            example: json!({"type": "text", "text": "Make it move\nlike you mean it", "font": "Avenir Next", "weight": 800, "role": "title", "color": "#ffffff", "pos": [960, 540], "align": "center", "preset": {"name": "reveal", "at": 0.3}, "mark": {"word": "move", "at": 1.0}}),
        },
        Entry {
            name: "image",
            summary: "Your own picture. `fit` cover|contain|fill|none, `focus` keeps the subject when cropping, `camera` is a Ken Burns move {from: {zoom, center}, to, ease}, `parallax` {depth: <grey image, white = near>, strength} gives a 2.5D move. `sampling` nearest|linear overrides automatic cubic enlargement / mipmapped reduction. EXIF orientation is applied. Size the source to at least the pixels shown (check warns about upscaling).",
            example: json!({"type": "image", "src": "photo.jpg", "fit": "cover", "focus": [0.6, 0.5], "camera": {"from": {"zoom": 1.0, "center": [0.45, 0.5]}, "to": {"zoom": 1.3, "center": [0.6, 0.5]}, "ease": "sine.inOut"}}),
        },
        Entry {
            name: "video",
            summary: "A video file streamed through ffmpeg. `src_in` starts into the file, `speed`, `loop`, `fit`, `size`; its sound is mixed unless `mute`. Combine with masks and mattes for rounded or shaped clips.",
            example: json!({"type": "video", "src": "clip.mp4", "size": [640, 360], "loop": true, "mute": true}),
        },
        Entry {
            name: "audio",
            summary: "A sound on the timeline: `volume` (a property), `fade_in`, `fade_out`, `src_in`, `speed`. Mixed in Rust (soft-clipped) and muxed into the render. Other layers can react: `amp(\"id\")` is its loudness, `onset(\"id\")` spikes on hits.",
            example: json!({"type": "audio", "id": "music", "src": "music.wav", "volume": 0.8, "fade_out": 1.0}),
        },
        Entry {
            name: "mesh",
            summary: "A solid: a triangle mesh from a `geometry` generator, lit by the composition's lights (ambient, point, spot, directional; `material` sets specular and shininess) and drawn through its camera. Needs \"three_d\": true. Geometry kinds: `text` (glyph outlines extruded, with an optional chamfer; one layer per letter to stagger a word), `box`, `sphere`, `torus`, `cylinder` (`top: 0` makes a cone) and `extrude` (an SVG path pushed into depth). It turns about its own middle, so `rotation` tumbles or spins it (`\"rotation\": [0, \"time*90\", 0]` is a turntable). `color` is the front and a curved skin, `side` the walls, `back` the back; `side` defaults to a darkened `color`.",
            example: json!({"type": "mesh", "three_d": true, "geometry": {"kind": "text", "text": "M", "font": "Avenir Next", "weight": 800, "size": 300, "depth": 60, "bevel": 6}, "color": "#e8402f", "side": "#8a2017",
                "material": {"specular": 0.8, "shininess": 40}, "pos": [960, 540, 0], "rotation": [20, {"from": -90, "to": 0, "dur": 0.8, "ease": "back.out(1.4)"}, 0]}),
        },
        Entry {
            name: "shape",
            summary: "Vector shapes with fill, gradient, stroke, dashes, trim (draw-on), morph and repeat. Geometry: rect, ellipse, polygon, star, d (SVG path), points. A fill is a colour or a gradient (linear {from, to}, radial {center, radius}). Every item animates its own properties (`trim`, `rotation`, `scale`, `opacity`, rect/ellipse sizes); `anchor` is the item's pivot for rotation and scale, in the item's own units from the shape's origin. `taper` thins the stroke toward its ends (see `stroke_taper`); an item's `repeat` takes a `palette` (see `palette_cycle`).",
            example: json!({"type": "shape", "pos": [960, 540], "items": [
                {"rect": [-200, -120, 400, 240], "radius": 24, "fill": {"gradient": {"type": "linear", "from": [0, -120], "to": [0, 120], "stops": [[0, "#ff8a00"], [1, "#e52e71"]]}}},
                {"ellipse": [0, 0, 90, 90], "stroke": {"color": "#ffffff", "width": 8, "cap": "round"}, "trim": {"end": {"from": 0, "to": 1, "at": 0.3, "dur": 1, "ease": "power3.inOut"}}},
                {"ellipse": [300, 0, 160, 160], "fill": {"gradient": {"type": "radial", "center": [0, 0], "radius": 90, "stops": [[0, "#ffe9a8"], [0.6, "#ff8a00"], [1, "#e52e71"]]}}},
                {"rect": [-60, 200, 120, 40], "anchor": [60, 20], "rotation": {"from": -20, "to": 20, "dur": 1.2, "ease": "sine.inOut"}, "fill": "#8be9fd"}
            ]}),
        },
        Entry {
            name: "null",
            summary: "An invisible transform to parent other layers to: one handle moves, scales or shakes a whole group.",
            example: json!({"type": "null", "id": "rig", "pos": [960, 540], "rotation": {"from": -4, "to": 4, "dur": 2, "ease": "sine.inOut"}}),
        },
        Entry {
            name: "precomp",
            summary: "Plays a nested composition from `comps` as one layer, with its own clock (offset, speed, time remap, loop). `sampling` nearest|linear controls transformed pixels (default linear); use nearest for integer-scaled pixel art. Everything a layer can do (effects, masks, mattes, 3D) works on it.",
            example: json!({"type": "precomp", "comp": "inner", "start": 0.5, "speed": 1, "pos": [960, 540], "scale": {"from": 0.8, "to": 1, "dur": 0.6, "ease": "expo.out"}}),
        },
        Entry {
            name: "adjustment",
            summary: "Applies its effects to everything below it (a grade, a glitch, a blur over the whole picture). Masks limit it to a region; opacity fades it.",
            example: json!({"type": "adjustment", "fx": [{"effect": "chromatic_aberration", "amount": 4}, {"effect": "vignette", "strength": 0.4}]}),
        },
        Entry {
            name: "texture",
            summary: "Paints a generator shader over the layer (noise, clouds, cells, stripes, grid, gradient, aura (a soft multi-point mesh-gradient backdrop with animatable point positions and colours), light_leak, bokeh, floor, or your own `effects/<name>.sksl` with `@kind generator`). Use it as a background, a matte, or blended over footage. `comp fx list` shows each one's parameters.",
            example: json!({"type": "texture", "shader": "clouds", "params": {"scale": 300, "high": "#9aa7c7"}, "blend": "screen", "opacity": 0.6}),
        },
        Entry {
            name: "particles",
            summary: "A particle system where every particle is a closed-form function of its age (so any frame renders alone). The emitter sits at the layer's position. Emit continuously (`rate`) and/or in `burst`s; `prewarm` starts it already running. Shapes: soft, circle, square, spark (streak along travel), star, ring, or your own outline `{\"path\": {\"d\": \"M0 -1 L1 1 L-1 1 Z\"}}` (any path geometry, fitted to the particle's size and turned by its spin: a snowflake, a leaf). Use blend `add` or `screen` for light.",
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
        "layers": ["ref(\"layer id\", \"x|y|z|rotation|opacity|scale|sx|sy\")", "vel(\"layer id\", \"x|y|z|rotation|opacity|scale|sx|sy\") units per second"],
        "operators": "+ - * / % ^ == != < <= > >= && || ! and a ? b : c",
    })
}

/// Every effect and generator as its descriptor: the platform's own, then the shipped plugins.
fn effects() -> Vec<crate::fxlib::Descriptor> {
    crate::fxlib::Library::builtin().expect("the built-in pack compiles").descriptors()
}

pub fn all() -> Value {
    let effects = effects();
    json!({
        "expression_support": expression_support::expression_support(&effects),
        "layers": layers(),
        "layer_common_keys": layer_keys::common_keys(),
        "enter_exit_presets": transitions(),
        "eases": eases(),
        "expressions": expression_functions(),
        "effects": effects,
        "effect_composite": crate::fxlib::composite::catalogue(),
        "effect_space": effect_space::entry(),
        "text_box": text_box::entry(),
        "palette_cycle": palette_cycle::entry(),
        "stroke_taper": stroke_taper::entry(),
        "color": color::entry(),
        "blend_modes": blend_modes::entry(),
        "keyframe_motion": motion_keys::entry(),
        "text_presets": ["rise", "reveal", "rise_words", "fade_up", "blur_in", "pop", "slam", "drop", "typewriter", "scramble", "tracking_in", "wave_in", "spin_in", "elastic", "fade_out", "rise_out"],
        "tokens": {
            "palette": "Named colours any kind can use: {\"accent\": \"#e2c48a\", \"ink\": \"#f4f7ff\", \"plate\": \"#10161f\"}. Write \"color\": \"accent\" or \"$accent\".",
            "type": "Text styles per role. A number is still a size; an object may add font, weight, color, tracking. Fonts live here, not on look.",
            "image": "Default treatment for pictures: {\"treat\": [\"grain\", \"vignette\"]}. Applied as fx when the image layer has none.",
            "surface": "Named plate recipes for shapes: {\"card\": {\"fill\": \"plate\", \"stroke\": {\"color\": \"accent\", \"width\": 1.5}, \"radius\": 18}}, then \"surface\": \"card\".",
            "look": "Finishing on the finished frame (grain, vignette). Not type, not images.",
            "example": {
                "palette": {"accent": "#e2c48a", "ink": "#f4f7ff"},
                "type": {"display": {"size": 140, "font": "Avenir Next Condensed", "weight": 800, "color": "accent"}, "title": {"size": 48, "font": "Avenir Next", "color": "ink"}, "body": 28},
                "layers": [
                    {"type": "text", "role": "display", "fill": "metal", "pos": [960, 440], "align": "center", "text": {"from": 0, "to": 827, "dur": 1.4, "prefix": "£", "suffix": "m"}},
                    {"type": "text", "role": "title", "pos": [960, 640], "align": "center", "text": "A future is a bet on a price", "mark": {"word": "future", "at": 0.6}, "preset": "reveal"}
                ]
            }
        },
        "sound_kit": crate::media::kit::kit().catalogue(),
        "timeline": timeline(),
        "extrude": {"summary": "Depth for a three_d text, shape or solid layer: copies stepping back in z, coloured from `color` to `far`. Show it with a camera move or a Y Rotation. `fx` apply to the front face only; the slices behind it stay plain colour, so an outline ring lies over the first slices and hides the sides at shallow angles.",
            "example": {"type": "text", "text": "88888", "size": 220, "weight": 800, "align": "center", "color": "#ffd166", "three_d": true, "pos": [960, 540], "rotation": [0, 18, 0], "extrude": {"depth": 60, "color": "#e8590c", "far": "#c2410c"}}},
        "repeat": {"summary": "Repeater for any layer: `count` copies in all, each one step further along (`offset`, `rotation`, `scale` and `opacity` per copy), turning and scaling about `origin`. Each copy keeps the layer's animation and effects; `offset` and `rotation` may be expressions. Needs an `id`. Later copies sit behind the original unless `order` is \"above\". Add `palette`, `cycle` and `paint` to give each copy its own colour (see `palette_cycle`).",
            "example": {"type": "solid", "id": "slab", "color": "#8a5cff", "size": [400, 240], "pos": [760, 500], "repeat": {"count": 6, "offset": [36, -24], "opacity": 0.85}}},
        "packs": crate::pack::catalogue().into_iter().map(|(n, s)| json!({"name": n, "summary": s})).collect::<Vec<_>>(),
        "make_a_new_effect": "Write effects/<name>.sksl beside comp.json: header comments `// @effect name`, `// @kind effect|generator`, `// @summary ...`, `// @param name float|vec2|color default [min max] -- doc`, `// @param name choice default a,b,c -- doc` (written as a name), `// @input name layer` (another layer's picture, named by id in the use), `// @input name history=N [depth=D]` (the effect's own output N frames ago), `// @pass name [scale=S]` (several passes, each sees the earlier ones by name), `// @space display|linear` (the encoding the shader computes in; see the colour key), `// @example {json}`, then `half4 main(float2 xy)`. You get `image` (effects), `resolution`, `time` and a library (hash21, vnoise, fbm, worley, rot2, luma, hsv2rgb, straight, premul). Try it with `comp fx test <name> --out sheet.png`; then use it like any effect: {\"effect\": \"name\", \"param\": value}.",
        "time_literals": ["1.5s", "45f", "beat:8", "bar:2", "marker:drop", "title.end", "title.start", "prev.end", "comp.end", "any of those followed by +0.25 or -3f"],
        "runnable_property_examples": property_examples::examples(),
        "property_forms": property_forms::entry(),
        "colour_forms": ["#rrggbb", "#rrggbbaa", "#rgb", "white", "accent (a palette token)", [255, 128, 0], [255, 128, 0, 0.5], {"from": "#000", "to": "#f00", "dur": 1}],
    })
}

/// The edit timeline's vocabulary: shot transitions, 2D frame moves and 3D camera moves, with an example.
fn timeline() -> Value {
    use crate::schema::timeline::{CAMERA_MOVES, MOVES, TRANSITIONS};
    json!({
        "summary": "A film as tracks: `shots` run back to back with a transition between each (overlapping by its length); `overlays` sit above, placed by shot; `hits` put a sound on a moment. Each shot becomes a precomp layer named by its id (and a marker).",
        "shot_fields": "id, dur, start (locks a cut), in (transition into it), layers | src | comp | block (+vars), bg, speed (number or keys: a speed ramp), offset, move, camera, fx, layout",
        "transitions": TRANSITIONS.iter().map(|(n, d, cue)| json!({"type": n, "summary": d, "sound": cue})).collect::<Vec<_>>(),
        "moves": MOVES.iter().map(|(n, d)| json!({"move": n, "summary": d})).collect::<Vec<_>>(),
        "camera_moves": CAMERA_MOVES.iter().map(|(n, d)| json!({"move": n, "summary": d})).collect::<Vec<_>>(),
        "example": {"timeline": {
            "transition": {"type": "push_left", "dur": 0.6},
            "shots": [
                {"id": "hook", "dur": 3, "layers": [{"type": "text", "text": "£827M", "role": "display", "pos": [960, 540], "align": "center"}], "move": "push_in"},
                {"id": "map", "dur": 4, "src": "map.png", "in": "pan_out_left", "speed": [[0, 1], [1.5, 0.3, "sine.inOut"], [3, 1]]},
                {"id": "deep", "dur": 4, "layers": [], "in": {"type": "zoom_through", "dur": 0.8}, "camera": {"move": "orbit", "deg": 24}}
            ],
            "overlays": [{"shot": "map", "at": 0.5, "dur": 2.5, "type": "precomp", "block": "lower_third", "vars": {"name": "Singapore"}}],
            "hits": [{"at": "deep.start+0.8", "cue": "boom"}]
        }}
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
        let media = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/comp/03-media");
        for e in layers() {
            if matches!(e.name, "image" | "video" | "audio") {
                // media examples need files; the generated ones may not exist in a fresh checkout
                if media.join("photo.jpg").exists() && media.join("clip.mp4").exists() {
                    let mut ex = e.example.clone();
                    ex["src"] = serde_json::json!(match e.name { "image" => "photo.jpg", "video" => "clip.mp4", _ => "beat.wav" });
                    Plan::from_value(json!({"layers": [ex]}), &media).unwrap_or_else(|err| panic!("{}: {err:#}", e.name));
                }
                continue;
            }
            load(vec![e.example.clone()]).unwrap_or_else(|err| panic!("{}: {err:#}", e.name));
        }
    }

    #[test]
    fn every_vector_and_colour_form_in_the_catalogue_parses() {
        use crate::anim::prop::{ColorProp, VecProp};
        let forms = all()["property_forms"].clone();
        for k in ["vector_constant", "vector_per_axis", "vector_tween", "vector_keys"] {
            VecProp::parse(&forms[k], 2..=3).unwrap_or_else(|e| panic!("{k}: {e:#}"));
        }
        for k in ["colour_tween", "colour_keys"] {
            ColorProp::parse(&forms[k]).unwrap_or_else(|e| panic!("{k}: {e:#}"));
        }
        for (what, bad) in forms["common_mistakes"].as_object().unwrap() {
            assert!(bad.as_str().unwrap().contains("RIGHT") || bad.as_str().unwrap().len() > 20, "{what}");
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
