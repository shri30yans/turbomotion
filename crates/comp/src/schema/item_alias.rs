//! Flat spellings of a shape item's nested look properties, so a property reads the same whether it is written once or
//! tweened: `fill_opacity` is `fill.opacity`, `trim_end` is `trim.end`.
//!
//! The SDK tweens items by these flat names (`tl.to(ring, { items: { 0: { trim_end: 1 } } })`). The table is the single list
//! of them; `nest` folds a flat item into the nested one the rest of the schema reads, so both forms evaluate identically.

use anyhow::{bail, Result};
use serde_json::{Map, Value};

/// (flat name, group, field in the group).
pub const ALIASES: &[(&str, &str, &str)] = &[
    ("trim_start", "trim", "start"),
    ("trim_end", "trim", "end"),
    ("trim_offset", "trim", "offset"),
    ("stroke_color", "stroke", "color"),
    ("stroke_width", "stroke", "width"),
    ("stroke_opacity", "stroke", "opacity"),
    ("dash_offset", "stroke", "dash_offset"),
    ("fill_color", "fill", "color"),
    ("fill_opacity", "fill", "opacity"),
];

pub fn is_alias(key: &str) -> bool {
    ALIASES.iter().any(|(flat, ..)| *flat == key)
}

/// A group (`fill`, `stroke`, `trim`) as the object its fields live in. A bare fill is a colour or `"metal"`.
fn group_object(group: &str, v: &Value) -> Result<Map<String, Value>> {
    match v {
        Value::Object(m) => Ok(m.clone()),
        other if group == "fill" && other.as_str() == Some("metal") => Ok(Map::from_iter([("metal".to_string(), Value::Bool(true))])),
        other if group == "fill" => Ok(Map::from_iter([("color".to_string(), other.clone())])),
        other => bail!("{group:?} must be an object to combine with its flat fields, found {other}"),
    }
}

/// The item with every flat alias moved into its group. Setting a field both ways is an error.
pub fn nest(item: &Map<String, Value>) -> Result<Map<String, Value>> {
    let mut out: Map<String, Value> = item.iter().filter(|(k, _)| !is_alias(k)).map(|(k, v)| (k.clone(), v.clone())).collect();
    for (flat, group, field) in ALIASES {
        let Some(value) = item.get(*flat) else { continue };
        let mut members = out.get(*group).map(|g| group_object(group, g)).transpose()?.unwrap_or_default();
        if members.insert((*field).to_string(), value.clone()).is_some() {
            bail!("{flat:?} and {group}.{field} are the same property; set it once");
        }
        out.insert((*group).to_string(), Value::Object(members));
    }
    Ok(out)
}
