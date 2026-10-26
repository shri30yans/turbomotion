//! The keyframe motion page: the fields of a key and of a property, and one composition for every form.

use std::fmt::Write as _;

use serde_json::Value;

use super::{json_block, page, str_of};

pub fn page_of(c: &Value) -> String {
    let m = &c["keyframe_motion"];
    let mut s = page("Keyframe handles and motion paths", str_of(m, "summary"));
    let _ = writeln!(s, "## A key\n\n| field | form | what it does |\n|---|---|---|");
    for f in m["key_fields"].as_array().into_iter().flatten() {
        let _ = writeln!(s, "| `{}` | {} | {} |", str_of(f, "field"), str_of(f, "form").replace('|', "\\|"), str_of(f, "doc"));
    }
    let _ = writeln!(s, "\n## A property of keys\n\n| field | what it does |\n|---|---|");
    for f in m["property_fields"].as_array().into_iter().flatten() {
        let _ = writeln!(s, "| `{}` | {} |", str_of(f, "field"), str_of(f, "doc"));
    }
    let _ = writeln!(s, "\n## Forms\n");
    for f in m["forms"].as_array().into_iter().flatten() {
        let _ = writeln!(s, "### `{}`\n\n{}\n", str_of(f, "name"), str_of(f, "summary"));
        json_block(&mut s, &f["example"]["composition"]);
    }
    s
}
