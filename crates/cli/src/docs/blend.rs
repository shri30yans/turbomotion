//! The blend modes page: every mode of the engine's table, by group, each with the composition that shows it.

use std::fmt::Write as _;

use serde_json::Value;

use super::{json_block, page, str_of};

pub fn page_of(c: &Value) -> String {
    let b = &c["blend_modes"];
    let mut s = page("Blend modes", str_of(b, "summary"));
    let aliases: Vec<String> = b["aliases"].as_array().into_iter().flatten().map(|a| format!("`{}` is `{}`", str_of(a, "alias"), str_of(a, "mode"))).collect();
    let _ = writeln!(s, "Default `{}`. Other names: {}.\n", str_of(b, "default"), aliases.join(", "));
    let modes = b["modes"].as_array().map_or(&[][..], Vec::as_slice);
    for group in b["groups"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        let _ = writeln!(s, "## {group}\n");
        for m in modes.iter().filter(|m| str_of(m, "group") == group) {
            let _ = writeln!(s, "### `{}`\n\n{}\n", str_of(m, "name"), str_of(m, "summary"));
            json_block(&mut s, &m["example"]["composition"]);
        }
    }
    s
}
