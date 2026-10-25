//! SVG in, shape layer out. Icons, logos and diagrams drawn elsewhere become vector shape items that animate like any
//! other (`trim` to draw them on, `fill` to recolour, `morph_to` between two). Supported: `path`, `rect`, `circle`,
//! `ellipse`, `line`, `polyline`, `polygon`, `g`, fill/stroke/opacity as attributes or `style`, `viewBox`, and
//! `translate` transforms. Anything else is skipped and reported in the warnings so nothing vanishes silently.

use std::collections::BTreeMap;

use anyhow::{bail, ensure, Result};
use serde_json::{json, Value};

pub struct Imported {
    pub layer: Value,
    pub warnings: Vec<String>,
}

#[derive(Clone, Default)]
struct Style {
    fill: Option<String>,
    stroke: Option<String>,
    stroke_width: Option<f64>,
    opacity: f64,
    dx: f64,
    dy: f64,
}

/// `fit` is the width in pixels the artwork should have in the composition; `id` names the layer.
pub fn import(svg: &str, id: &str, fit: f64) -> Result<Imported> {
    ensure!(fit > 0.0, "the target width must be positive");
    let tags = scan(svg)?;
    let root = tags.iter().find(|t| t.name == "svg").ok_or_else(|| anyhow::anyhow!("no <svg> element found"))?;
    let (vx, vy, vw, vh) = match root.attrs.get("viewBox").map(|v| numbers(v)) {
        Some(v) if v.len() == 4 && v[2] > 0.0 && v[3] > 0.0 => (v[0], v[1], v[2], v[3]),
        _ => {
            let w = root.attrs.get("width").and_then(|v| number(v)).unwrap_or(100.0);
            let h = root.attrs.get("height").and_then(|v| number(v)).unwrap_or(100.0);
            (0.0, 0.0, w, h)
        }
    };
    let mut warnings = vec![];
    let mut items = vec![];
    let mut stack: Vec<Style> = vec![Style { fill: Some("#000000".into()), opacity: 1.0, ..Style::default() }];
    for t in &tags {
        if t.closing {
            if t.name == "g" && stack.len() > 1 {
                stack.pop();
            }
            continue;
        }
        let mut style = stack.last().cloned().unwrap_or_default();
        apply(&mut style, t, &mut warnings);
        if t.name == "g" {
            if !t.self_closing {
                stack.push(style);
            }
            continue;
        }
        let geometry = match t.name.as_str() {
            "path" => t.attrs.get("d").map(|d| json!({"d": d})),
            "rect" => {
                let g = |k: &str| t.attrs.get(k).and_then(|v| number(v)).unwrap_or(0.0);
                let mut v = json!({"rect": [g("x"), g("y"), g("width"), g("height")]});
                let r = t.attrs.get("rx").or_else(|| t.attrs.get("ry")).and_then(|v| number(v)).unwrap_or(0.0);
                if r > 0.0 {
                    v["radius"] = json!(r);
                }
                Some(v)
            }
            "circle" => {
                let g = |k: &str| t.attrs.get(k).and_then(|v| number(v)).unwrap_or(0.0);
                Some(json!({"ellipse": [g("cx"), g("cy"), g("r"), g("r")]}))
            }
            "ellipse" => {
                let g = |k: &str| t.attrs.get(k).and_then(|v| number(v)).unwrap_or(0.0);
                Some(json!({"ellipse": [g("cx"), g("cy"), g("rx"), g("ry")]}))
            }
            "line" => {
                let g = |k: &str| t.attrs.get(k).and_then(|v| number(v)).unwrap_or(0.0);
                style.fill = None;
                Some(json!({"d": format!("M {} {} L {} {}", g("x1"), g("y1"), g("x2"), g("y2"))}))
            }
            "polyline" | "polygon" => t.attrs.get("points").map(|p| {
                let n = numbers(p);
                let pts: Vec<Value> = n.chunks_exact(2).map(|c| json!([c[0], c[1]])).collect();
                json!({"points": pts, "closed": t.name == "polygon"})
            }),
            "svg" | "defs" | "title" | "desc" | "metadata" | "style" => None,
            other => {
                warnings.push(format!("skipped <{other}>: not supported"));
                None
            }
        };
        let Some(Value::Object(mut item)) = geometry else { continue };
        if style.fill.is_none() && style.stroke.is_none() {
            continue;
        }
        if let Some(f) = &style.fill {
            item.insert("fill".into(), json!(f));
        }
        if let Some(s) = &style.stroke {
            item.insert("stroke".into(), json!({"color": s, "width": style.stroke_width.unwrap_or(1.0)}));
        }
        if style.opacity < 1.0 {
            item.insert("opacity".into(), json!(style.opacity));
        }
        if style.dx != 0.0 || style.dy != 0.0 {
            item.insert("offset".into(), json!([style.dx, style.dy]));
        }
        items.push(Value::Object(item));
    }
    if items.is_empty() {
        bail!("the SVG has no shapes this importer understands{}", if warnings.is_empty() { String::new() } else { format!(" ({})", warnings.join("; ")) });
    }
    let scale = fit / vw;
    let layer = json!({
        "type": "shape", "id": id, "items": items,
        "anchor": [vx + vw / 2.0, vy + vh / 2.0], "pos": [fit / 2.0, vh * scale / 2.0], "scale": scale,
    });
    warnings.sort();
    warnings.dedup();
    Ok(Imported { layer, warnings })
}

fn apply(style: &mut Style, t: &Tag, warnings: &mut Vec<String>) {
    let mut set = |k: &str, v: &str| match k {
        "fill" => style.fill = paint(v),
        "stroke" => style.stroke = paint(v),
        "stroke-width" => style.stroke_width = number(v),
        "opacity" => style.opacity *= number(v).unwrap_or(1.0).clamp(0.0, 1.0),
        "fill-opacity" | "stroke-opacity" | "transform" | "style" => {}
        _ => {}
    };
    for (k, v) in &t.attrs {
        set(k, v);
    }
    if let Some(s) = t.attrs.get("style") {
        for decl in s.split(';') {
            if let Some((k, v)) = decl.split_once(':') {
                set(k.trim(), v.trim());
            }
        }
    }
    if let Some(tr) = t.attrs.get("transform") {
        let tr = tr.trim();
        if let Some(args) = tr.strip_prefix("translate(").and_then(|r| r.strip_suffix(')')) {
            let n = numbers(args);
            style.dx += n.first().copied().unwrap_or(0.0);
            style.dy += n.get(1).copied().unwrap_or(0.0);
        } else {
            warnings.push(format!("ignored transform {tr:?} (only translate is supported; apply the others in the artwork)"));
        }
    }
}

/// `none`/`currentColor`/`url(...)` → no usable paint; hex, `rgb()` and common names → a colour string.
fn paint(v: &str) -> Option<String> {
    let v = v.trim();
    match v {
        "" | "none" | "transparent" | "currentColor" => None,
        _ if v.starts_with("url(") => None,
        _ if v.starts_with('#') => Some(if v.len() == 4 { format!("#{}", v[1..].chars().flat_map(|c| [c, c]).collect::<String>()) } else { v.to_string() }),
        _ if v.starts_with("rgb(") => {
            let n = numbers(v.trim_start_matches("rgb(").trim_end_matches(')'));
            (n.len() >= 3).then(|| format!("#{:02x}{:02x}{:02x}", n[0] as u8, n[1] as u8, n[2] as u8))
        }
        "black" => Some("#000000".into()),
        "white" => Some("#ffffff".into()),
        "red" => Some("#ff0000".into()),
        "green" => Some("#008000".into()),
        "blue" => Some("#0000ff".into()),
        "gray" | "grey" => Some("#808080".into()),
        "yellow" => Some("#ffff00".into()),
        "orange" => Some("#ffa500".into()),
        _ => None,
    }
}

fn number(v: &str) -> Option<f64> {
    v.trim().trim_end_matches("px").parse().ok()
}

fn numbers(v: &str) -> Vec<f64> {
    v.split(|c: char| c == ',' || c.is_whitespace()).filter(|s| !s.is_empty()).filter_map(|s| s.trim_end_matches("px").parse().ok()).collect()
}

struct Tag {
    name: String,
    attrs: BTreeMap<String, String>,
    closing: bool,
    self_closing: bool,
}

/// A deliberately small XML tag scanner: enough for SVG exported by design tools, with comments and declarations skipped.
fn scan(src: &str) -> Result<Vec<Tag>> {
    let mut tags = vec![];
    let mut rest = src;
    while let Some(i) = rest.find('<') {
        rest = &rest[i + 1..];
        if let Some(r) = rest.strip_prefix("!--") {
            rest = r.split_once("-->").map_or("", |(_, after)| after);
            continue;
        }
        if rest.starts_with('?') || rest.starts_with('!') {
            rest = rest.split_once('>').map_or("", |(_, after)| after);
            continue;
        }
        let Some(end) = tag_end(rest) else { bail!("an SVG tag is never closed with '>'") };
        let body = &rest[..end];
        rest = &rest[end + 1..];
        let (closing, body) = body.strip_prefix('/').map_or((false, body), |b| (true, b));
        let (self_closing, body) = body.strip_suffix('/').map_or((false, body), |b| (true, b));
        let name_end = body.find(char::is_whitespace).unwrap_or(body.len());
        let name = body[..name_end].rsplit(':').next().unwrap_or("").to_string();
        tags.push(Tag { name, attrs: attrs(&body[name_end..]), closing, self_closing });
    }
    Ok(tags)
}

/// Index of the `>` that ends the tag, ignoring any inside quoted attribute values.
fn tag_end(s: &str) -> Option<usize> {
    let mut quote = None;
    for (i, c) in s.char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), c) if c == q => quote = None,
            (None, '>') => return Some(i),
            _ => {}
        }
    }
    None
}

fn attrs(mut s: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    while let Some(eq) = s.find('=') {
        let key = s[..eq].trim().rsplit(|c: char| c.is_whitespace()).next().unwrap_or("").to_string();
        let after = s[eq + 1..].trim_start();
        let Some(q) = after.chars().next().filter(|c| *c == '"' || *c == '\'') else { break };
        let Some(close) = after[1..].find(q) else { break };
        out.insert(key, after[1..1 + close].to_string());
        s = &after[close + 2..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ICON: &str = r##"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24">
  <!-- a comment with <rect> inside -->
  <g fill="#f00" transform="translate(2, 3)">
    <circle cx="12" cy="12" r="5"/>
    <path d="M0 0 L10 10" fill="none" stroke="rgb(0,128,255)" stroke-width="2"/>
  </g>
  <rect x="1" y="1" width="4" height="4" rx="1" style="fill:#0f0;opacity:0.5"/>
  <text>skip me</text>
</svg>"##;

    #[test]
    fn imports_shapes_styles_and_reports_what_it_skipped() {
        let out = import(ICON, "icon", 240.0).unwrap();
        let items = out.layer["items"].as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["ellipse"], json!([12.0, 12.0, 5.0, 5.0]));
        assert_eq!(items[0]["fill"], "#ff0000", "short hex is expanded");
        assert_eq!(items[0]["offset"], json!([2.0, 3.0]));
        assert!(items[1].get("fill").is_none(), "fill=none draws only the stroke");
        assert_eq!(items[1]["stroke"], json!({"color": "#0080ff", "width": 2.0}));
        assert_eq!(items[2]["fill"], "#00ff00");
        assert_eq!(items[2]["opacity"], json!(0.5));
        assert_eq!(items[2]["radius"], json!(1.0));
        assert_eq!(out.layer["scale"], json!(10.0), "24 units to 240 px is 10x");
        assert_eq!(out.warnings, vec!["skipped <text>: not supported".to_string()]);
    }

    #[test]
    fn imported_layer_is_a_valid_shape_layer() {
        let out = import(ICON, "icon", 240.0).unwrap();
        crate::schema::parse_layer(0, &out.layer).expect("the schema accepts the importer's output");
    }

    #[test]
    fn rejects_empty_and_non_svg_input() {
        assert!(import("<html></html>", "x", 100.0).is_err());
        assert!(import(r#"<svg viewBox="0 0 1 1"><text/></svg>"#, "x", 100.0).is_err());
        assert!(import("<svg", "x", 100.0).is_err());
    }
}
