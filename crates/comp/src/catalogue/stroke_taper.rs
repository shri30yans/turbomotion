//! The catalogue entry for a shape item's `taper`. It owns the wording and one runnable composition; the key is parsed
//! in `schema::taper`, its numbers are read in `eval::shape::taper` and the outline is built in `shape::stroke`.

use serde_json::{json, Value};

pub fn entry() -> Value {
    json!({
        "summary": "`taper` on a shape item makes its stroke thinner toward either end of the outline: a variable-width stroke. It is drawn as the filled region between the two offset curves of the path (with the stroke's own `cap` and `join`; corners are joined, tight bends do not leave holes, and the edges stay within a fiftieth of a pixel of the true offset curve). `start` is the first point of the outline and `end` the last. Each end has a `length` (how far from the end the stroke takes to reach its full width) and a `width` (how wide it is at the end itself, default 0: a point). A length is pixels, or `{\"fraction\": f}` of the outline's length; a width is pixels, or `{\"fraction\": f}` of the stroke's `width`; each is a property that may animate. A length cannot be negative and is held to half the outline, so the two ends never overlap: a taper of 5000 px on a 400 px path is 200 px at each end and the stroke is full width at the middle only. `ease` (a GSAP name, default linear) shapes the width from the tip to the full width and is the same at both ends, measured from each tip. Between the tapers the stroke is its full `width`. A closed outline is tapered from its start point: `start` runs along the first stretch and `end` back along the last stretch before the start point, so with both given the stroke pinches at the start point from both sides, and with only one the stroke steps from the tip's width to full there. Several contours (a path with more than one `M`) are each tapered along their own length. A tapered stroke follows the trimmed outline: with `trim`, the taper is measured along the part that is drawn. The item needs a `stroke` and cannot be dashed; the colour, opacity, `cap` and `join` are the stroke's; a repeater's palette colours it like any stroke.",
        "keys": {
            "start": "{length, width}: the taper at the first point of the outline.",
            "end": "{length, width}: the taper at the last point of the outline (on a closed outline, the stretch before the start point).",
            "length": "Pixels, or {\"fraction\": f} of the outline's length: how far from the end the stroke takes to reach its full width. 0 or more; held to half the outline.",
            "width": "Pixels, or {\"fraction\": f} of the stroke's width: how wide the stroke is at the end itself (default 0). 0 or more.",
            "ease": "A GSAP ease (default linear): how the width rises from the tip to the full width, measured from each tip.",
        },
        "example": {"composition": {
            "width": 1920, "height": 1080, "fps": 30, "dur": 3, "bg": "#10141f",
            "layers": [
                {"id": "arrow", "type": "shape", "items": [
                    {"d": "M 260 800 C 560 300 1060 1000 1560 380", "stroke": {"color": "#e63946", "width": 56, "cap": "round", "join": "round"},
                        "taper": {"start": {"length": 360, "width": 0}, "end": {"length": {"fraction": 0.12}, "width": {"fraction": 0.3}}, "ease": "power2.out"},
                        "trim": {"end": {"from": 0, "to": 1, "dur": 1.5, "ease": "power2.inOut"}}},
                    {"ellipse": [1560, 380, 150, 150], "stroke": {"color": "#f1fa8c", "width": 36}, "taper": {"start": {"length": 120}, "end": {"length": 120}}}]}
            ]
        }},
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::taper::Taper;

    #[test]
    fn the_entry_names_every_key_the_parser_reads_and_its_example_plans_with_a_taper_on_each_item() {
        let e = entry();
        let documented: Vec<&str> = e["keys"].as_object().unwrap().keys().map(String::as_str).collect();
        for key in ["start", "end", "length", "width", "ease"] {
            assert!(documented.contains(&key), "the entry lacks {key}");
        }
        for item in e["example"]["composition"]["layers"][0]["items"].as_array().unwrap() {
            let taper: Taper = crate::schema::de::from_value_at(item["taper"].clone()).expect("the example's taper parses");
            taper.validate().expect("and is valid");
        }
        crate::plan::Plan::from_value(e["example"]["composition"].clone(), std::path::Path::new(".")).expect("the example plans");
    }
}
