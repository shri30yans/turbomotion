//! Repeater: `"repeat": {"count": 6, "offset": [40, 0], "rotation": 8, "scale": 0.9, "opacity": 0.8}` on any layer draws
//! `count` copies of it, each one step further along (After Effects' Repeater, for every layer kind). Copy `k` is the layer
//! under a null that sits `k` steps from the original, so the copy keeps the layer's own animation, enter, exit and effects
//! and nothing about its position has to be rewritten. The steps turn and scale about `origin` (the parent's origin when left
//! out), and `offset` and `rotation` may be expressions, so a step can change over time.
//!
//! Only `opacity` is applied to the copy itself, because a null's opacity does not reach its children.

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde_json::{json, Map, Value};

const FIELDS: &[&str] = &["count", "offset", "rotation", "scale", "opacity", "origin", "order"];
/// Most copies one repeater may make: a stack this tall is a mistake, not a look.
const MAX_COUNT: u64 = 256;

pub fn expand(root: &mut Value) -> Result<()> {
    if let Some(layers) = root.get_mut("layers") {
        layers_pass(layers).context("root")?;
    }
    if let Some(comps) = root.get_mut("comps").and_then(Value::as_object_mut) {
        for (name, c) in comps.iter_mut() {
            if let Some(layers) = c.get_mut("layers") {
                layers_pass(layers).with_context(|| format!("comp {name:?}"))?;
            }
        }
    }
    Ok(())
}

fn layers_pass(layers: &mut Value) -> Result<()> {
    let Some(list) = layers.as_array_mut() else { return Ok(()) };
    if !list.iter().any(|l| l.get("repeat").is_some()) {
        return Ok(());
    }
    let mut out = Vec::with_capacity(list.len());
    for (i, mut l) in list.drain(..).enumerate() {
        let Some(spec) = l.as_object_mut().and_then(|o| o.remove("repeat")) else {
            out.push(l);
            continue;
        };
        let spec = Spec::parse(&spec).with_context(|| format!("layer {i} repeat"))?;
        let mut copies = copies(&l, &spec).with_context(|| format!("layer {i} repeat"))?;
        if spec.above {
            out.push(l);
            out.extend(copies.into_iter().flatten());
        } else {
            copies.reverse();
            out.extend(copies.into_iter().flatten());
            out.push(l);
        }
    }
    *list = out;
    Ok(())
}

struct Spec {
    count: usize,
    offset: Vec<Value>,
    rotation: Vec<Value>,
    scale: f64,
    opacity: f64,
    origin: Vec<f64>,
    above: bool,
}

impl Spec {
    fn parse(spec: &Value) -> Result<Spec> {
        let s = spec.as_object().ok_or_else(|| anyhow!("repeat is an object like {{\"count\": 5, \"offset\": [40, 0]}}"))?;
        if let Some(k) = s.keys().find(|k| !FIELDS.contains(&k.as_str())) {
            bail!("unknown repeat field {k:?}; fields are {}", FIELDS.join(", "));
        }
        let count = s.get("count").and_then(Value::as_u64).ok_or_else(|| anyhow!("repeat needs \"count\": how many layers in all, the original included"))?;
        ensure!((2..=MAX_COUNT).contains(&count), "repeat count is {count}; use 2 to {MAX_COUNT}");
        let factor = |key: &str| -> Result<f64> {
            let v = s.get(key).map_or(Ok(1.0), |v| v.as_f64().ok_or_else(|| anyhow!("repeat {key} is a number: the factor each copy takes of the one before")))?;
            ensure!(v > 0.0, "repeat {key} is {v}; it must be above 0");
            Ok(v)
        };
        let above = match s.get("order").and_then(Value::as_str) {
            None | Some("below") => false,
            Some("above") => true,
            Some(other) => bail!("repeat order is \"below\" (later copies behind, the default) or \"above\", not {other:?}"),
        };
        Ok(Spec {
            count: count as usize,
            offset: step(s.get("offset"), "offset")?,
            rotation: step(s.get("rotation"), "rotation")?,
            scale: factor("scale")?,
            opacity: factor("opacity")?,
            origin: s.get("origin").map_or(Ok(vec![]), |v| numbers(v, "origin"))?,
            above,
        })
    }
}

/// A step is a number or an expression, or a list of them, one per axis.
fn step(v: Option<&Value>, key: &str) -> Result<Vec<Value>> {
    let ok = |x: &Value| x.is_number() || x.is_string();
    match v {
        None => Ok(vec![]),
        Some(x) if ok(x) => Ok(vec![x.clone()]),
        Some(Value::Array(a)) if !a.is_empty() && a.len() <= 3 && a.iter().all(ok) => Ok(a.clone()),
        Some(other) => bail!("repeat {key} is a number, an expression string, or up to three of them; got {other}"),
    }
}

fn numbers(v: &Value, key: &str) -> Result<Vec<f64>> {
    let a = v.as_array().filter(|a| (2..=3).contains(&a.len())).ok_or_else(|| anyhow!("repeat {key} is [x, y] or [x, y, z]"))?;
    a.iter().map(|n| n.as_f64().ok_or_else(|| anyhow!("repeat {key} holds numbers"))).collect()
}

/// `k` steps of `step`: a number multiplies, an expression is wrapped so it can change over time.
fn times(k: usize, step: &Value, base: f64) -> Value {
    match step {
        Value::String(s) => json!(format!("{base} + {k} * ({s})")),
        n => json!(base + k as f64 * n.as_f64().unwrap_or(0.0)),
    }
}

/// For each copy, its null and the copy itself, nearest the original first.
fn copies(layer: &Value, spec: &Spec) -> Result<Vec<[Value; 2]>> {
    let o = layer.as_object().ok_or_else(|| anyhow!("a layer is an object"))?;
    let id = o.get("id").and_then(Value::as_str).ok_or_else(|| anyhow!("a repeated layer needs an \"id\" (the copies are named from it)"))?;
    let three_d = o.get("three_d").and_then(Value::as_bool) == Some(true);
    let dims = if three_d { 3 } else { 2 };
    ensure!(spec.origin.is_empty() || spec.origin.len() == dims, "repeat origin has {} numbers but the layer is {}", spec.origin.len(), if three_d { "three_d: use [x, y, z]" } else { "flat: use [x, y]" });
    ensure!(spec.offset.len() <= dims, "repeat offset has {} numbers but the layer is {}", spec.offset.len(), if three_d { "three_d" } else { "flat: use [x, y]" });
    ensure!(three_d || spec.rotation.len() <= 1, "repeat rotation of a flat layer is one number (degrees)");
    let origin: Vec<f64> = if spec.origin.is_empty() { vec![0.0; dims] } else { spec.origin.clone() };
    let opacity = o.get("opacity").cloned();
    let mut out = Vec::with_capacity(spec.count - 1);
    for k in 1..spec.count {
        let wrapper_id = format!("{id}~r{k}");
        let mut w = Map::new();
        w.insert("type".into(), json!("null"));
        w.insert("id".into(), json!(wrapper_id));
        w.insert("three_d".into(), json!(three_d));
        if let Some(p) = o.get("parent") {
            w.insert("parent".into(), p.clone());
        }
        w.insert("anchor".into(), json!(origin));
        let pos: Vec<Value> = (0..dims).map(|a| spec.offset.get(a).map_or_else(|| json!(origin[a]), |s| times(k, s, origin[a]))).collect();
        w.insert("pos".into(), Value::Array(pos));
        if !spec.rotation.is_empty() {
            let turn = |s: &Value| times(k, s, 0.0);
            let rotation = if three_d { Value::Array((0..3).map(|a| spec.rotation.get(a).map_or(json!(0), turn)).collect()) } else { turn(&spec.rotation[0]) };
            w.insert("rotation".into(), rotation);
        }
        if spec.scale != 1.0 {
            w.insert("scale".into(), json!(spec.scale.powi(k as i32)));
        }
        let mut c = o.clone();
        c.insert("id".into(), json!(format!("{id}~c{k}")));
        c.insert("parent".into(), json!(wrapper_id));
        if spec.opacity != 1.0 {
            c.insert("opacity".into(), fade(opacity.as_ref(), spec.opacity.powi(k as i32))?);
        }
        out.push([Value::Object(w), Value::Object(c)]);
    }
    Ok(out)
}

/// The layer's opacity times a constant, for every form of opacity that can be multiplied.
fn fade(opacity: Option<&Value>, factor: f64) -> Result<Value> {
    Ok(match opacity {
        None => json!(factor),
        Some(Value::Number(n)) => json!(n.as_f64().unwrap_or(1.0) * factor),
        Some(Value::String(s)) => json!(format!("({s}) * {factor}")),
        Some(other) => bail!("repeat opacity cannot fade an opacity of {other}; write the layer's opacity as a number or an expression"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(layer: Value) -> Result<Vec<Value>> {
        let mut v = json!({"layers": [layer]});
        expand(&mut v)?;
        Ok(v["layers"].as_array().unwrap().clone())
    }

    fn card() -> Value {
        json!({"type": "solid", "id": "card", "color": "#fff", "pos": [100, 100]})
    }

    fn with(spec: Value) -> Value {
        let mut l = card();
        l["repeat"] = spec;
        l
    }

    #[test]
    fn copies_sit_under_nulls_that_step_away_from_the_original() {
        let out = run(with(json!({"count": 3, "offset": [40, 10], "rotation": 5, "scale": 0.5, "origin": [960, 540]}))).unwrap();
        let ids: Vec<&str> = out.iter().map(|l| l["id"].as_str().unwrap()).collect();
        // later copies first (they sit behind), the original last and on top
        assert_eq!(ids, ["card~r2", "card~c2", "card~r1", "card~c1", "card"]);
        let w = &out[2];
        assert_eq!(w["type"], "null");
        assert_eq!(w["pos"], json!([1000.0, 550.0]));
        assert_eq!(w["anchor"], json!([960.0, 540.0]));
        assert_eq!(w["rotation"], json!(5.0));
        assert_eq!(w["scale"], json!(0.5));
        assert_eq!(out[3]["parent"], "card~r1");
        assert_eq!(out[3]["pos"], json!([100, 100]));
        assert!(out[4].get("repeat").is_none() && out[4]["id"] == "card");
    }

    #[test]
    fn order_above_puts_the_copies_over_the_original_and_opacity_falls_off() {
        let out = run(with(json!({"count": 3, "offset": [10, 0], "order": "above", "opacity": 0.5}))).unwrap();
        assert_eq!(out[0]["id"], "card");
        assert_eq!(out[2]["opacity"], json!(0.5));
        assert_eq!(out[4]["opacity"], json!(0.25));
    }

    #[test]
    fn a_step_may_be_an_expression_and_a_copy_keeps_the_layers_parent_and_opacity_expression() {
        let mut l = with(json!({"count": 2, "rotation": "time * 10", "opacity": 0.5}));
        l["parent"] = json!("rig");
        l["opacity"] = json!("smooth(0, 1, time)");
        let out = run(l).unwrap();
        assert_eq!(out[0]["rotation"], json!("0 + 1 * (time * 10)"));
        assert_eq!(out[0]["parent"], "rig");
        assert_eq!(out[1]["parent"], "card~r1");
        assert_eq!(out[1]["opacity"], json!("(smooth(0, 1, time)) * 0.5"));
    }

    #[test]
    fn three_d_layers_get_three_d_nulls() {
        let mut l = with(json!({"count": 2, "offset": [0, 0, -50], "rotation": [0, 15, 0]}));
        l["three_d"] = json!(true);
        let out = run(l).unwrap();
        assert_eq!(out[0]["three_d"], true);
        assert_eq!(out[0]["pos"], json!([0.0, 0.0, -50.0]));
        assert_eq!(out[0]["rotation"], json!([0.0, 15.0, 0.0]));
    }

    #[test]
    fn mistakes_are_named() {
        let refused = |spec: Value| format!("{:#}", run(with(spec)).unwrap_err());
        assert!(refused(json!({"offset": [1, 0]})).contains("count"));
        assert!(refused(json!({"count": 1})).contains("2 to"));
        assert!(refused(json!({"count": 3, "ofset": [1, 0]})).contains("ofset"));
        assert!(refused(json!({"count": 3, "scale": 0})).contains("above 0"));
        assert!(refused(json!({"count": 3, "order": "front"})).contains("order"));
        assert!(refused(json!({"count": 3, "offset": [1, 2, 3]})).contains("flat"));
        let mut l = card();
        l.as_object_mut().unwrap().remove("id");
        l["repeat"] = json!({"count": 2});
        assert!(format!("{:#}", run(l).unwrap_err()).contains("needs an \"id\""));
    }
}
