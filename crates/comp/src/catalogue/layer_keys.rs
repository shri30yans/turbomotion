//! `comp catalogue layer <type>` and `comp catalogue effect <name>`: the keys a layer or effect accepts, with each key's
//! form, whether it is required, and what it does. The key lists are read from the parser's own structs
//! (`schema::fields`) and the docs from its source (`schema::docs`), so they are what the parser accepts.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use turbomotion_core::suggest::hint;

use crate::schema::{
    docs::{enum_named, struct_named},
    fields::{self, Fields},
    Kind,
};

/// The accepted spellings of the wrapper types that parse more than one form (`P`, `Pos`, ...), which no struct field
/// list can show.
const WRAPPER_FORMS: &[(&str, &str)] = &[
    ("P", "a number | an expression string (\"time * 90\", \"wiggle(2, 30)\") | {\"from\": a, \"to\": b, \"at\": s, \"dur\": s, \"ease\": e} | {\"keys\": [[seconds, value, ease?], ...]}"),
    ("Pos", "[x, y] or [x, y, z] (each entry a P) | {\"from\": [..], \"to\": [..], \"at\", \"dur\", \"ease\"} | {\"keys\": [[seconds, [x, y], ease?], ...]}"),
    ("Scale", "a number (1 = 100%, uniform) | [sx, sy] or [sx, sy, sz] (each entry a P) | a vector tween or keys"),
    ("Col", "\"#rrggbb\" | \"#rrggbbaa\" | a palette name (\"accent\") | [r, g, b] 0..255 | {\"from\": colour, \"to\": colour, \"at\", \"dur\", \"ease\"} | {\"keys\": [[seconds, colour, ease?], ...]}"),
    ("Rot", "a number (the Z Rotation in degrees) | [x, y, z] (each entry a P) | a vector tween or keys"),
    ("Orient", "[x, y, z] degrees (each entry a P) | a vector tween or keys"),
];

/// Types with a hand-written parser: no field list to read, so their forms are written out (and a test keeps the
/// list honest: every type a layer key names must resolve).
const CUSTOM_FORMS: &[(&str, &str)] = &[
    ("TextSource", "a string | a counter {from: number, to: number, at, dur, ease, decimals, prefix, suffix, sep} | a swap {from: string, to: string, at, dur, ease}"),
    ("Fill", "a colour | \"metal\" | {color?, gradient?: {type: linear|radial, from, to, center, radius, stops: [[0, colour], [1, colour]]}, metal?, opacity?, rule?: nonzero|evenodd}"),
    ("Perspective", "a number (viewer distance in px; smaller is stronger) | \"orthographic\""),
    ("TextPath", "an object with ONE geometry (d, points, ellipse, rect, polygon, star) or `layer` (the id of a shape layer), plus reverse, perpendicular, force_align, first_margin, last_margin, travel"),
    ("ParticleShape", "soft | circle | square | spark | star | ring | {\"path\": <one geometry: d, points, rect, ellipse, polygon, star>} (your own outline, fitted to the particle size)"),
    ("Mark", "an object {word*: string, nth, at, dur, ease, color, pad, verdict, rough}"),
];

/// Keys whose Rust type is a catch-all `Value` but which have a real form.
const KEY_FORMS: &[(&str, &str, &str)] = &[
    ("Common", "fx", "a list of effects, each {\"effect\": \"blur\", <its parameters>}; `turbomotion comp catalogue effect <name>` lists an effect's parameters, defaults and ranges"),
    ("Text", "preset", "a preset name (\"rise\") | {\"name\": \"rise\", \"by\": \"words\"|\"chars\"|\"lines\", \"at\": seconds, ...}; the names are `text_presets` in `turbomotion comp catalogue`"),
];

const NUMBERS: &[&str] = &["f64", "f32", "u8", "u16", "u32", "u64", "usize", "i8", "i16", "i32", "i64", "isize"];

/// How a value of Rust type `ty` is written in a composition.
pub fn form(ty: &str) -> String {
    let ty = ty.trim();
    if let Some(inner) = ty.strip_prefix("Option<").and_then(|t| t.strip_suffix('>')) {
        return form(inner);
    }
    if let Some(inner) = ty.strip_prefix("Box<").and_then(|t| t.strip_suffix('>')) {
        return form(inner);
    }
    if NUMBERS.contains(&ty) {
        return "number".into();
    }
    if let Some(inner) = ty.strip_prefix("Vec<").and_then(|t| t.strip_suffix('>')) {
        return format!("a list of {}", form(inner));
    }
    if let Some(body) = ty.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
        if let Some((elem, n)) = body.rsplit_once("; ") {
            return format!("[{}]", vec![form(elem); n.parse().unwrap_or(1)].join(", "));
        }
    }
    match ty {
        "bool" => "true | false".into(),
        "String" => "a string".into(),
        "Value" => "any JSON value".into(),
        "Map<String, Value>" => "an object".into(),
        "BTreeMap<String, String>" => "an object of strings".into(),
        _ => named_form(ty),
    }
}

fn named_form(ty: &str) -> String {
    if let Some((_, f)) = WRAPPER_FORMS.iter().chain(CUSTOM_FORMS).find(|(n, _)| *n == ty) {
        return (*f).to_string();
    }
    if ty == "Blend" {
        let modes = fields::variants_of::<turbomotion_core::effects::Blend>().map_or(Vec::new(), |v| v.keys.to_vec());
        return format!("one of {}", modes.iter().map(|m| format!("{m:?}")).collect::<Vec<_>>().join(" | "));
    }
    if let Some(e) = enum_named(ty) {
        return format!("one of {}", e.variants.iter().map(|v| format!("{:?}", v.name)).collect::<Vec<_>>().join(" | "));
    }
    if let Some(s) = struct_named(ty) {
        let keys: Vec<String> = s.fields.iter().map(|f| if f.required { format!("{}*", f.key) } else { f.key.to_string() }).collect();
        let marked = if s.fields.iter().any(|f| f.required) { " (* required)" } else { "" };
        return format!("an object {{{}}}{marked}", keys.join(", "));
    }
    ty.to_string()
}

/// One key as a catalogue row.
fn row(struct_name: &str, key: &str) -> Value {
    let field = struct_named(struct_name).and_then(|s| s.fields.iter().find(|f| f.key == key));
    let form_override = KEY_FORMS.iter().find(|(s, k, _)| *s == struct_name && *k == key).map(|(_, _, f)| (*f).to_string());
    match field {
        Some(f) => json!({"key": key, "form": form_override.unwrap_or_else(|| form(f.ty)), "required": f.required, "doc": f.doc}),
        None => json!({"key": key, "form": "", "required": false, "doc": ""}),
    }
}

fn rows(fields: Fields) -> Vec<Value> {
    fields.keys.iter().map(|k| row(fields.name, k)).collect()
}

/// The keys every layer accepts, each with its form: the catalogue's `layer_common_keys`.
pub fn common_keys() -> Vec<Value> {
    rows(fields::common())
}

/// Everything about one layer type.
pub fn layer(name: &str) -> Result<Value> {
    let entry = super::layers().into_iter().find(|e| e.name == name).ok_or_else(|| anyhow!("no layer type {name:?}.{} The types are {}", hint(name, Kind::NAMES), Kind::NAMES.join(", ")))?;
    let own = fields::of_kind(name).ok_or_else(|| anyhow!("layer type {name:?} has no key list"))?;
    let mut keys = rows(own);
    if name == "shape" {
        keys = vec![json!({"key": "items", "form": "a list of items; each names ONE outline (rect [x, y, w, h], ellipse [cx, cy, rx, ry], polygon [cx, cy, radius, sides], star [cx, cy, outer, inner, points], d \"M..\", points [[x, y], ..]) plus how it is painted", "required": true, "doc": "The pieces of the shape, in paint order; see `item_keys`."})];
    }
    Ok(json!({
        "type": name,
        "summary": entry.summary,
        "example": entry.example,
        "keys": keys,
        "item_keys": if name == "shape" { crate::schema::shape_item_keys().into_iter().map(|(k, part)| json!({"key": k, "part": part})).collect() } else { Vec::new() },
        "common_keys": common_keys(),
        "note": "`keys` are this type's own; `common_keys` are accepted by every layer. A key marked required has no default.",
    }))
}

/// Everything about one effect or generator (platform or plugin): parameters with defaults and ranges, and an example.
pub fn effect(lib: &crate::fxlib::Library, name: &str) -> Result<Value> {
    let all = lib.descriptors();
    let names: Vec<&str> = all.iter().map(|d| d.name.as_str()).collect();
    let d = all.iter().find(|d| d.name == name).ok_or_else(|| anyhow!("no effect or generator {name:?}.{} `turbomotion comp fx list` shows every one", hint(name, &names)))?;
    Ok(serde_json::to_value(d)?)
}
