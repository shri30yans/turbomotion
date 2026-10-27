//! The catalogue entry for a repeater's palette cycle. It owns the wording and one runnable composition; the keys are
//! parsed in `schema::palette`, chosen in `anim::cycle` and turned into colours in `eval::palette`.

use serde_json::{json, Value};

pub fn entry() -> Value {
    json!({
        "summary": "A repeater can give every copy its own colour from a palette. Add `palette`, `cycle` and `paint` to a layer `repeat` (the original is copy 0, the others 1 to count - 1) or to a shape item's `repeat` (copy 0 is the item itself). Both repeaters read the same three keys. Copy i falls `i + offset` colours along the palette: `step` takes colour `floor(i + offset) mod n` (a whole offset moves every copy on by that many colours, a fractional one holds the colour until it reaches the next whole number, a negative one moves back); `gradient` blends the two colours either side of `(i + offset) mod n`, the last colour blending back into the first, in the film's working colour space (display-encoded values in an ordinary film, light in a `linear_light` film, so it matches a Skia gradient drawn in the same film); `ping_pong` runs the palette forwards then backwards, `0 1 .. n-1 n-2 .. 1 0 1 ..`, the end colours not doubled. `reverse` reads the palette from its last colour to its first before any of this. `offset` is a property (a number, an expression, {from, to, at, dur, ease} or keys), so it can animate: animate it from 0 to n over a loop to run the colours along the row. The colours in `palette` may be animated too. `paint` says which paint of a copy takes the colour: \"fill\" (default; for text and solid layers their own colour), \"stroke\" or \"both\". A shape item takes a fill colour only when its fill is one flat colour (a gradient or metal fill is refused, with `paint: \"stroke\"` available), and a stroke only when it has one. A layer repeater works on text, solid and shape layers (an image or a precomp has no colour to replace, and is refused when the film is planned); each copy carries the result as a `swatch` {palette, index, cycle, paint}, a layer key you can also write by hand. The per-copy `opacity` of the repeater applies to the coloured copy as before. The repeater has whole copies only: `count` and `copies` are integers, so there is no fractional copy to colour.",
        "keys": {
            "palette": "Required to colour copies: a list of at least one colour (hex, a palette name, an animated colour). Written inside `repeat`.",
            "cycle": "{mode, offset, reverse}. `mode` is \"step\" (default), \"gradient\" or \"ping_pong\". `offset` is how many colours the palette is shifted on, a property that may animate and may be negative or fractional (default 0). `reverse` reads the palette last colour first (default false).",
            "paint": "\"fill\" (default), \"stroke\" or \"both\": which paint of the copy takes the colour.",
        },
        "example": {"composition": {
            "width": 1920, "height": 1080, "fps": 30, "dur": 3, "bg": "#10141f",
            "layers": [
                {"id": "chip", "type": "solid", "color": "#ffffff", "size": [150, 150], "pos": [260, 340],
                    "repeat": {"count": 8, "offset": [200, 0], "palette": ["#e63946", "#f1fa8c", "#2a9d8f", "#264653"], "cycle": {"mode": "step", "offset": {"from": 0, "to": 4, "dur": 3, "ease": "none"}}}},
                {"id": "ramp", "type": "shape", "pos": [0, 0], "items": [
                    {"ellipse": [260, 720, 70, 70], "fill": "#ffffff", "stroke": {"color": "#ffffff", "width": 8},
                        "repeat": {"copies": 8, "offset": [200, 0], "opacity": 0.9, "palette": ["#e63946", "#2a9d8f", "#9b5de5"], "cycle": {"mode": "gradient", "offset": 0.5}, "paint": "both"}}]}
            ]
        }},
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::palette::Cycling;

    #[test]
    fn the_entry_names_every_key_the_parser_reads_and_its_example_plans_with_a_palette_on_both_repeaters() {
        let e = entry();
        let mut documented: Vec<&str> = e["keys"].as_object().unwrap().keys().map(String::as_str).collect();
        documented.sort_unstable();
        let mut parsed = crate::schema::palette::KEYS.to_vec();
        parsed.sort_unstable();
        assert_eq!(documented, parsed);
        assert_eq!(parsed, ["cycle", "paint", "palette"]);
        let layers = e["example"]["composition"]["layers"].as_array().unwrap();
        for repeat in [&layers[0]["repeat"], &layers[1]["items"][0]["repeat"]] {
            let mut map = repeat.as_object().cloned().unwrap();
            assert!(Cycling::take(&mut map).expect("the palette parses").is_some());
        }
        crate::plan::Plan::from_value(e["example"]["composition"].clone(), std::path::Path::new(".")).expect("the example plans");
    }
}
