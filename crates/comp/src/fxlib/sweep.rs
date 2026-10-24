//! `comp fx test`: render a plugin with each of its knobs swept across its range, as one contact sheet, so the agent
//! sees what every parameter does before it uses the effect.

use std::path::Path;

use anyhow::{ensure, Result};
use serde_json::{json, Map, Value};

use super::{Def, Kind, Ty};

pub const STEPS: usize = 5;

pub struct Sweep {
    /// The composition to render; sample it at `times`.
    pub comp: Value,
    pub times: Vec<f64>,
    /// One line per sheet cell: what was swept and the value.
    pub legend: Vec<String>,
}

/// The values a float parameter is tried at.
fn range(d: f64, min: Option<f64>, max: Option<f64>) -> Vec<f64> {
    match (min, max) {
        (Some(lo), Some(hi)) => (0..STEPS).map(|i| lo + (hi - lo) * i as f64 / (STEPS - 1) as f64).collect(),
        _ if d == 0.0 => (0..STEPS).map(|i| i as f64 / (STEPS - 1) as f64).collect(),
        _ => [0.0, 0.5, 1.0, 1.5, 2.0].iter().map(|k| d * k).collect(),
    }
}

/// A value that holds `v` around the middle of second `slot` and moves between slots quickly.
fn held(slot: usize, v: Value, prev: Option<&mut Vec<Value>>) -> Vec<Value> {
    let s = slot as f64;
    let mut keys = prev.map(std::mem::take).unwrap_or_default();
    keys.push(json!([s + 0.02, v]));
    keys.push(json!([s + 0.98, v]));
    keys
}

pub fn build(def: &Def, on: Option<&Path>) -> Result<Sweep> {
    let floats: Vec<usize> = def.params.iter().enumerate().filter(|(_, p)| p.ty == Ty::Float).map(|(i, _)| i).collect();
    let rows = floats.len().max(1);
    let slots = rows * STEPS;
    let mut keys: Vec<Vec<Value>> = vec![vec![]; def.params.len()];
    let mut legend = vec![];
    for slot in 0..slots {
        let (row, col) = (slot / STEPS, slot % STEPS);
        let swept = floats.get(row).copied();
        for (k, p) in def.params.iter().enumerate() {
            let v = match p.ty {
                Ty::Float => {
                    let x = if Some(k) == swept { range(p.default[0], p.min, p.max)[col] } else { p.default[0] };
                    if Some(k) == swept {
                        legend.push(format!("t={:.2}s  {} = {}", slot as f64 + 0.5, p.name, (x * 1000.0).round() / 1000.0));
                    }
                    json!(x)
                }
                Ty::Vec2 => json!(p.default),
                Ty::Color => json!(format!("#{:02x}{:02x}{:02x}", p.default[0] as u8, p.default[1] as u8, p.default[2] as u8)),
            };
            let mut cur = std::mem::take(&mut keys[k]);
            keys[k] = held(slot, v, Some(&mut cur));
        }
        if swept.is_none() {
            legend.push(format!("t={:.2}s  (no float parameters; only time changes)", slot as f64 + 0.5));
        }
    }
    let mut params = Map::new();
    for (k, p) in def.params.iter().enumerate() {
        params.insert(p.name.clone(), Value::Array(std::mem::take(&mut keys[k])));
    }
    let (w, h) = (640, 360);
    let layers = match def.kind {
        Kind::Generator => vec![json!({"type": "texture", "shader": def.name, "params": params})],
        Kind::Effect => {
            let mut fx = params.clone();
            fx.insert("effect".into(), json!(def.name));
            let subject = match on {
                Some(p) => {
                    let abs = std::fs::canonicalize(p).map_err(|e| anyhow::anyhow!("{}: {e}", p.display()))?;
                    json!([{"type": "image", "src": abs.to_string_lossy(), "fit": "cover"}])
                }
                None => test_card(),
            };
            ensure!(subject.is_array(), "internal: test card is a list of layers");
            return Ok(Sweep {
                comp: json!({"width": w, "height": h, "fps": 30, "dur": slots, "bg": "#0b0b10", "comps": {"subject": {"width": w, "height": h, "dur": slots, "layers": subject}},
                    "layers": [{"type": "precomp", "comp": "subject", "fx": [Value::Object(fx)]}]}),
                times: (0..slots).map(|s| s as f64 + 0.5).collect(),
                legend,
            });
        }
    };
    Ok(Sweep { comp: json!({"width": w, "height": h, "fps": 30, "dur": slots, "bg": "#0b0b10", "layers": layers}), times: (0..slots).map(|s| s as f64 + 0.5).collect(), legend })
}

/// A picture with edges, flat colour, a gradient, fine detail and type: enough to show what most effects do.
fn test_card() -> Value {
    json!([
        {"type": "texture", "shader": "gradient", "params": {"from": "#1d3a7a", "to": "#e8703a", "angle": 60}},
        {"type": "shape", "pos": [320, 180], "items": [
            {"rect": [-250, -130, 140, 90], "radius": 10, "fill": "#f4efe4"},
            {"ellipse": [60, -20, 150, 150], "fill": "#2fd6a1"},
            {"star": [190, 40, 70, 30, 5], "fill": "#ffd23f"},
            {"ellipse": [-100, 70, 90, 90], "stroke": {"color": "#ffffff", "width": 6}}
        ]},
        {"type": "text", "text": "Aa 123", "font": "Avenir Next", "weight": 800, "size": 64, "color": "#ffffff", "pos": [320, 300], "align": "center"}
    ])
}
