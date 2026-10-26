//! The catalogue entry for keyframe handles and motion paths: what a key can say beyond a time, a value and a named ease.
//! Every form carries a composition that shows it, and the same tests that load every example run it.

use serde_json::{json, Value};

pub fn entry() -> Value {
    json!({
        "summary": "Keys that move the way After Effects' keys move. A key written as an object `{\"t\": seconds, \"v\": value}` can carry the two kinds of handle After Effects gives a keyframe. TEMPORAL handles (`ease_in`, `ease_out`) shape the speed of the value in time: each side is `\"linear\"`, `\"hold\"` (leaving only: the value stays until the next key), `\"auto\"` (the slope between the neighbouring keys, influence 16.667 %), `\"easy\"` (Easy Ease: speed 0, influence 33.33 %) or `{\"speed\": units per second, \"influence\": percent 0 to 100}`. They are the two inner control points of one cubic Bezier on the graph of value against time, exactly as After Effects' Keyframe Velocity dialog draws it; a speed that disagrees with the neighbouring key overshoots, and influences that overlap are kept as given. A key's `ease` is a named ease for the move into it; a segment takes that or handles, not both. `continuous: true` gives both sides one speed (write the speed on one side, only `influence` on the other). SPATIAL handles shape the path a position takes in space: a position written as `{\"keys\": [{\"t\", \"v\": [x, y]}, ..]}` is a motion path, a cubic Bezier through its keys. A key's `spatial` is `\"auto\"` (Auto Bezier: a smooth curve through the key, whose handle is a sixth of the way from the key before to the key after), `\"linear\"` (a straight corner), `\"bezier\"` (its own `in` and `out` handles, offsets from the key) or `\"continuous\"` (one handle given, the other its mirror); `in` or `out` alone mean `bezier`. The speed of a position is a speed ALONG the path in pixels per second, and the path is travelled by arc length, so an ease moves the layer evenly along a curve however the handles bend. A key with `\"roving\": true` has no time: its place on the path is fixed and its time is found, so the speed stays even from the fixed key before it to the fixed key after. `auto_orient: \"path\"` turns the layer to face along the path (a banked object): on a flat layer a rotation about z, on a 3D layer the depth axis along the direction of travel; it is refused together with `aim` or `along.orient`. `spatial` on the property sets the default handles of keys that give none. A vector that is not a position (scale, an effect's point) takes the same temporal handles per component and has no path. Colours interpolate their channels as written (0 to 255 and alpha), see the colour pipeline. A property written dimension by dimension, `[x, y]` or `{\"separate_dimensions\": [x, y]}`, keeps each dimension's own keys and expression. The old forms (`[[t, v, ease]]`, tweens, `loop`) are unchanged.",
        "key_fields": [
            {"field": "t", "form": "seconds", "doc": "When the key is. Keys must not go back in time."},
            {"field": "v", "form": "a number, [x, y], [x, y, z], a colour, true/false or text", "doc": "The value at the key; every key of a property has the same type."},
            {"field": "ease", "form": "a named ease", "doc": "The ease of the move into this key (the old third item of `[t, v, ease]`)."},
            {"field": "ease_in", "form": "linear | auto | easy | {speed, influence}", "doc": "How the value arrives at this key."},
            {"field": "ease_out", "form": "linear | hold | auto | easy | {speed, influence}", "doc": "How the value leaves this key. `hold` keeps the value until the next key."},
            {"field": "continuous", "form": "true | false", "doc": "Both sides share one speed (After Effects' Continuous Bezier)."},
            {"field": "spatial", "form": "auto | linear | bezier | continuous", "doc": "The motion path's handles at this key (a position only)."},
            {"field": "in", "form": "[dx, dy] or [dx, dy, dz]", "doc": "The path's handle pointing back along the path, an offset from the key."},
            {"field": "out", "form": "[dx, dy] or [dx, dy, dz]", "doc": "The path's handle pointing forward, an offset from the key."},
            {"field": "roving", "form": "true", "doc": "A key with no time (no `t`, no ease): its time follows from the keys around it."}
        ],
        "property_fields": [
            {"field": "keys", "doc": "The list of keys, as arrays `[t, v, ease?]` or objects."},
            {"field": "loop", "doc": "`cycle`, `pingpong` or `offset`: what the property does after its last key."},
            {"field": "spatial", "doc": "Default handles of the keys of a motion path: `auto` (default for a position), `linear` or `bezier`."},
            {"field": "auto_orient", "doc": "`\"path\"`: face along the motion path."},
            {"field": "separate_dimensions", "doc": "`[x, y, z?]`: one property per dimension, each with its own keys and expression."}
        ],
        "forms": [
            form("motion_path", "A position that follows a smooth curve through three keys and banks along it. Every key without handles is Auto Bezier; the first leaves and the last arrives with Easy Ease.",
                moving(json!({"keys": [{"t": 0, "v": [220, 520], "ease_out": "easy"}, {"t": 1.5, "v": [640, 180]}, {"t": 3, "v": [1060, 520], "ease_in": "easy"}], "auto_orient": "path"}), arrow())),
            form("bezier_handles", "A quarter circle from its own handles: `out` and `in` are offsets from the keys (55.23 % of the radius draws a circle), travelled at a steady pace by arc length.",
                moving(json!({"keys": [{"t": 0, "v": [900, 360], "spatial": "bezier", "out": [0, 138], "ease": "none"}, {"t": 3, "v": [640, 640], "spatial": "bezier", "in": [138, 0], "ease": "none"}]}), dot())),
            form("roving", "Two roving points between fixed keys at 0 and 3 seconds: the layer passes them at one steady speed with a linear ease, and never stops at them.",
                moving(json!({"keys": [{"t": 0, "v": [200, 560], "ease": "none"}, {"roving": true, "v": [500, 160]}, {"roving": true, "v": [820, 600]}, {"t": 3, "v": [1080, 200], "ease": "none"}]}), dot())),
            form("speed_and_influence", "Handles with a speed and an influence on a rotation: it leaves at 0 degrees per second with 80 % influence and arrives the same way, a long ease at both ends.",
                turning(json!({"keys": [{"t": 0, "v": 0, "ease_out": {"speed": 0, "influence": 80}}, {"t": 2.5, "v": 360, "ease_in": {"speed": 0, "influence": 80}}]}))),
            form("overshoot", "A leaving speed of 900 degrees per second with 50 % influence overshoots the final 180 and settles back to it.",
                turning(json!({"keys": [{"t": 0, "v": 0, "ease_out": {"speed": 900, "influence": 50}}, {"t": 2, "v": 180, "ease_in": {"speed": 0, "influence": 40}}]}))),
            form("continuous_speed", "A continuous key: the position leaves the middle key at the speed it arrived with (300 pixels per second), each side with its own influence.",
                moving(json!({"keys": [{"t": 0, "v": [200, 360], "spatial": "linear"}, {"t": 1.5, "v": [640, 360], "spatial": "linear", "continuous": true, "ease_in": {"speed": 300, "influence": 40}, "ease_out": {"influence": 60}}, {"t": 3, "v": [1080, 360], "spatial": "linear"}]}), dot())),
            form("hold", "A hold key: opacity steps from 1 to 0.2 to 1 at exactly the next key's time, with no ease between.",
                layer(json!({"pos": [640, 360], "opacity": {"keys": [{"t": 0, "v": 1, "ease_out": "hold"}, {"t": 1, "v": 0.2, "ease_out": "hold"}, {"t": 2, "v": 1}]}}), arrow())),
            form("separate_dimensions", "X and Y with keys of their own: x glides across at a steady speed while y rises and falls.",
                moving(json!({"separate_dimensions": [{"keys": [[0, 200], [3, 1080, "none"]]}, {"keys": [[0, 560], [1.5, 200, "sine.out"], [3, 560, "sine.in"]]}]}), dot())),
            form("colour_handles", "A colour with a hold at the first key and Easy Ease into the next: its channels change one by one as written.",
                json!({"type": "solid", "size": [640, 360], "pos": [640, 360], "color": {"keys": [{"t": 0, "v": "#2040ff", "ease_out": "hold"}, {"t": 1, "v": "#ff5030", "ease_out": "easy"}, {"t": 2.5, "v": "#ffd166", "ease_in": "easy"}]}}))
        ]
    })
}

/// A form: its name, what it shows, and a layer that shows it in a film of its own.
fn form(name: &str, summary: &str, layer: Value) -> Value {
    json!({"name": name, "summary": summary, "example": {"composition": {"width": 1280, "height": 720, "fps": 30, "dur": 3, "bg": "#10131c", "layers": [layer]}}})
}

fn layer(extra: Value, item: Value) -> Value {
    let mut l = json!({"type": "shape", "items": [item]});
    l.as_object_mut().expect("an object").extend(extra.as_object().expect("an object").clone());
    l
}

/// A layer whose position is `pos`.
fn moving(pos: Value, item: Value) -> Value {
    layer(json!({"pos": pos}), item)
}

/// A layer at the middle of the frame whose rotation is `rotation`.
fn turning(rotation: Value) -> Value {
    layer(json!({"pos": [640, 360], "rotation": rotation}), arrow())
}

fn arrow() -> Value {
    json!({"points": [[-60, -30], [60, 0], [-60, 30], [-30, 0]], "closed": true, "fill": "#ffd166"})
}

fn dot() -> Value {
    json!({"ellipse": [-30, -30, 60, 60], "fill": "#ffd166"})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{eval::Evaluator, plan::{Plan, ROOT}};

    #[test]
    fn every_form_loads_and_evaluates() {
        for f in entry()["forms"].as_array().unwrap() {
            let plan = Plan::from_value(f["example"]["composition"].clone(), std::path::Path::new(".")).unwrap_or_else(|e| panic!("{}: {e:#}", f["name"]));
            for t in [0.0, 0.7, 1.5, 2.9] {
                Evaluator::new(&plan).frame(ROOT, t).unwrap_or_else(|e| panic!("{} at {t}: {e:#}", f["name"]));
            }
        }
    }

    #[test]
    fn the_key_fields_are_the_ones_a_key_object_accepts() {
        let e = entry();
        let documented: Vec<&str> = e["key_fields"].as_array().unwrap().iter().map(|f| f["field"].as_str().unwrap()).collect();
        let accepted = crate::anim::prop::KEY_FIELDS;
        assert_eq!(documented, accepted, "the catalogue lists exactly what the parser accepts");
    }
}
