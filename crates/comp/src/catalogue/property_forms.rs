//! The forms a property can be written in, with the mistakes agents make most often.

use serde_json::{json, Value};

pub fn entry() -> Value {
    json!({
            "constant": 40,
            "expression": "wiggle(2, 30)",
            "keys": [[0, 0], [0.6, 100, "expo.out"], [2, 100]],
            "tween": {"from": 0, "to": 100, "at": 1, "dur": 0.6, "ease": "back.out(1.4)"},
            "loop": {"keys": [[0, 0], [1, 360, "none"]], "loop": "cycle"},
            "keys_plus_expression": {"value": [[0, 0], [1, 100]], "expr": "value + wiggle(3, 5)"},
            "vector": "pos, scale, anchor, effect and texture vec2 parameters are vectors. Four spellings, all runnable under runnable_property_examples:",
            "vector_constant": [960, 540],
            "vector_per_axis": [{"from": 0.8, "to": 1.2, "at": 0, "dur": 1}, {"keys": [[0, 1], [1, 0.6, "expo.out"]]}],
            "vector_tween": {"from": [0.8, 1.2], "to": [1.2, 0.8], "at": 0, "dur": 1, "ease": "sine.inOut"},
            "vector_keys": {"keys": [[0, [100, 200]], [1, [400, 200], "expo.out"], [2, [400, 500]]]},
            "colour_tween": {"from": "#2040ff", "to": "#ff5030", "at": 0, "dur": 1, "ease": "none"},
            "colour_keys": {"keys": [[0, "#2040ff"], [1, "#ff5030", "none"], [2, "#ffffff", "sine.inOut"]]},
            "shape_item": "Items in a shape take the same forms: {\"rect\": [x, y, w, h], \"anchor\": [cx, cy], \"rotation\": {tween}, \"scale\": [vector], \"trim\": {\"end\": {tween}}, \"fill\": colour-or-animated-colour}",
            "common_mistakes": {
                "a list of tween objects as keys": "WRONG: \"opacity\": [{\"from\": 0, \"to\": 1}, {\"from\": 1, \"to\": 0, \"at\": 1}]. RIGHT: one tween, or keys: [[0, 0], [0.5, 1], [1, 0]]",
                "vector keys without the wrapper": "WRONG: \"pos\": [[0, [0, 0]], [1, [100, 50]]]. RIGHT: \"pos\": {\"keys\": [[0, [0, 0]], [1, [100, 50]]]}",
                "a vector given to a scalar": "rotation, opacity and size are single numbers; give them keys of numbers, not of vectors",
                "keys out of order": "key times must not decrease; a later tween on the same property starts where the earlier one ends"
            }
        })
}
