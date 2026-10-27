//! The catalogue entry for the text layer's `box` key. It owns the wording and one runnable composition; the key is
//! parsed in `schema::text_box`, measured in `text::bounds` and drawn by `render::text`.

use serde_json::{json, Value};

pub fn entry() -> Value {
    json!({
        "summary": "A filled rectangle fitted to a text layer's characters, drawn behind the glyphs inside the same layer: it moves, scales, turns, fades and takes the layer's effects with the text, in either `effect_space`. It is as wide as the characters showing plus the padding on both sides and as tall as the lines it covers (ascent to descent) plus the padding above and below, measured from the real shaped text (kerning, tracking, line breaks, the font's own metrics). It follows what is showing: a typewriter bar grows with every character, a masked rise grows as the line comes out from behind its mask, a tracking-in widens it, and a text with nothing showing has no box. Whitespace has no extent, so a box hugs the first and last visible character of a line. The same measurement is `source_rect_at_time(t, include_extents)` in the engine, the `bounds` of the layer's `content` in `comp inspect`, and `bounds` in the SDK's `measure()`.",
        "keys": {
            "fit": "Required: \"text\", the only choice. The box is fitted to the measured characters.",
            "fill": "Required: the box's colour (a colour, a palette name or an animated colour).",
            "padding": "[x, y] pixels between the characters and the box: x on the left and right, y above and below. Each is a property (a number, an expression, {from, to, at, dur, ease} or keys), so it can animate; neither may be negative (default [0, 0]).",
            "radius": "Corner radius in pixels, held to half the box's shorter side, so a large one makes a pill (default 0).",
            "min_width": "The box is never narrower than this many pixels, padding included (default 0).",
            "align": "Which side of a box widened by `min_width` keeps its place: \"left\" grows to the right, \"right\" to the left, \"center\" both ways (default: the text layer's own `align`).",
            "per_line": "true draws one box for each line that has characters showing, each as wide as its own line; false (default) draws one box round the whole block.",
        },
        "example": {"composition": {
            "width": 1920, "height": 1080, "fps": 30, "dur": 3, "bg": "#10141f",
            "layers": [
                {"id": "typed", "type": "text", "text": "Make it move", "font": "Avenir Next", "weight": 800, "size": 96, "color": "#ffffff", "pos": [300, 400],
                    "preset": {"name": "typewriter", "at": 0.2, "dur": 1.4},
                    "box": {"fit": "text", "padding": [{"from": 56, "to": 36, "at": 0.2, "dur": 1.4, "ease": "power2.out"}, 24], "radius": 28, "fill": "#e63946", "min_width": 360}},
                {"id": "lines", "type": "text", "text": "Two lines,\nEach with its own bar", "font": "Avenir Next", "weight": 700, "size": 64, "color": "#10141f", "pos": [300, 700],
                    "box": {"fit": "text", "padding": [28, 14], "radius": 10, "fill": "#e2c48a", "per_line": true}}
            ]
        }},
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::text_box::TextBox;

    #[test]
    fn the_entry_names_every_key_the_parser_reads_and_its_example_plans_with_a_box_on_each_text() {
        let e = entry();
        let documented: Vec<&str> = e["keys"].as_object().unwrap().keys().map(String::as_str).collect();
        for key in ["fit", "fill", "padding", "radius", "min_width", "align", "per_line"] {
            assert!(documented.contains(&key), "the entry lacks {key}");
        }
        let layers = e["example"]["composition"]["layers"].as_array().unwrap();
        for l in layers {
            let parsed: TextBox = crate::schema::de::from_value_at(l["box"].clone()).expect("the example's box parses");
            parsed.validate().expect("and is valid");
        }
        crate::plan::Plan::from_value(e["example"]["composition"].clone(), std::path::Path::new(".")).expect("the example plans");
    }
}
