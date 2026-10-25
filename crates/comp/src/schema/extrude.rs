//! Extrusion: a flat 3D text, shape or solid layer gains depth. `"extrude": {"depth": 60, "color": "#3a2a10"}` on a
//! `three_d` layer adds copies of it stepping back in z, coloured from `color` (near) to `far` (default: `color` at
//! 55 % brightness), behind the face. Copies keep the layer's animation, enter and exit, so the solid moves as one;
//! under a camera move or a `rot_y` the sides show. This is the slice method After Effects plug-ins used before
//! its ray-traced renderer: cheap, exact in silhouette, and it lights like the face does.

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde_json::{json, Map, Value};

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
    if !list.iter().any(|l| l.get("extrude").is_some()) {
        return Ok(());
    }
    let mut out = Vec::with_capacity(list.len());
    for (i, l) in list.drain(..).enumerate() {
        match l.get("extrude") {
            Some(spec) => {
                let spec = spec.clone();
                let made = slices(&l, &spec).with_context(|| format!("layer {i} extrude"))?;
                out.extend(made);
                let mut face = l;
                face.as_object_mut().expect("checked by slices").remove("extrude");
                out.push(face);
            }
            None => out.push(l),
        }
    }
    *list = out;
    Ok(())
}

fn slices(layer: &Value, spec: &Value) -> Result<Vec<Value>> {
    let o = layer.as_object().ok_or_else(|| anyhow!("a layer is an object"))?;
    let s = spec.as_object().ok_or_else(|| anyhow!("extrude is {{\"depth\": 40, \"color\": \"#222\"}}"))?;
    for k in s.keys() {
        ensure!(["depth", "steps", "color", "far"].contains(&k.as_str()), "unknown extrude field {k:?}; fields are depth, steps, color, far");
    }
    ensure!(o.get("three_d").and_then(Value::as_bool) == Some(true), "an extruded layer needs \"three_d\": true (depth only shows in 3D)");
    let kind = o.get("type").and_then(Value::as_str).unwrap_or("");
    ensure!(["text", "shape", "solid"].contains(&kind), "extrude works on text, shape and solid layers, not {kind:?}");
    let pos = o.get("pos").ok_or_else(|| anyhow!("an extruded layer needs \"pos\""))?;
    let depth = s.get("depth").and_then(Value::as_f64).unwrap_or(40.0);
    ensure!(depth > 0.0, "extrude depth must be positive");
    let steps = s.get("steps").and_then(Value::as_u64).map_or_else(|| (depth / 2.5).round().clamp(6.0, 64.0) as usize, |n| n as usize).max(1);
    let near = s.get("color").cloned().unwrap_or(json!("#20242c"));
    let far = s.get("far").cloned().or_else(|| hex(&near).map(|c| json!(to_hex(c.map(|x| x * 0.55))))).unwrap_or_else(|| near.clone());

    let mut out = Vec::with_capacity(steps);
    // far slices first; the renderer also depth-sorts consecutive 3D layers
    for k in (1..=steps).rev() {
        let u = k as f64 / steps as f64;
        let dz = depth * u;
        let color = match (hex(&near), hex(&far)) {
            (Some(a), Some(b)) => json!(to_hex([0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * u))),
            _ => near.clone(),
        };
        let mut c: Map<String, Value> = o.clone();
        c.remove("extrude");
        c.remove("mark");
        c.remove("reflect");
        c.remove("fx");
        if let Some(id) = o.get("id").and_then(Value::as_str) {
            c.insert("id".into(), json!(format!("{id}~d{k}")));
        }
        c.insert("layout".into(), json!("ignore"));
        c.insert("pos".into(), offset_z(pos, dz).context("pos")?);
        recolor(&mut c, kind, &color);
        out.push(Value::Object(c));
    }
    Ok(out)
}

fn recolor(c: &mut Map<String, Value>, kind: &str, color: &Value) {
    match kind {
        "text" => {
            c.insert("color".into(), color.clone());
            c.remove("fill");
            c.remove("runs");
            c.remove("stroke");
        }
        "solid" => {
            c.insert("color".into(), color.clone());
        }
        _ => {
            if let Some(items) = c.get_mut("items").and_then(Value::as_array_mut) {
                for it in items.iter_mut().filter_map(Value::as_object_mut) {
                    if it.contains_key("fill") {
                        it.insert("fill".into(), color.clone());
                    }
                    if let Some(st) = it.get_mut("stroke").and_then(Value::as_object_mut) {
                        st.insert("color".into(), color.clone());
                    }
                }
            }
        }
    }
}

fn hex(v: &Value) -> Option<[f64; 3]> {
    let s = v.as_str()?.strip_prefix('#')?;
    let s: String = if s.len() == 3 { s.chars().flat_map(|c| [c, c]).collect() } else { s.chars().take(6).collect() };
    if s.len() != 6 {
        return None;
    }
    let p = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok().map(f64::from);
    Some([p(0)?, p(2)?, p(4)?])
}

fn to_hex(c: [f64; 3]) -> String {
    let b = |x: f64| x.round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", b(c[0]), b(c[1]), b(c[2]))
}

/// The same position pushed `dz` further from the camera, for every position form the schema accepts.
fn offset_z(pos: &Value, dz: f64) -> Result<Value> {
    Ok(match pos {
        Value::Array(parts) if parts.len() == 2 => json!([parts[0], parts[1], dz]),
        Value::Array(parts) if parts.len() == 3 => json!([parts[0], parts[1], add(&parts[2], dz)?]),
        Value::Object(o) if o.contains_key("from") => {
            let mut o = o.clone();
            for k in ["from", "to"] {
                let v = o.get(k).and_then(Value::as_array).ok_or_else(|| anyhow!("a moving pos needs \"from\" and \"to\" vectors"))?;
                let z = v.get(2).and_then(Value::as_f64).unwrap_or(0.0) + dz;
                o.insert(k.into(), json!([v[0], v[1], z]));
            }
            Value::Object(o)
        }
        Value::Object(o) if o.contains_key("keys") => {
            let mut o = o.clone();
            let keys = o.get("keys").and_then(Value::as_array).ok_or_else(|| anyhow!("keys must be a list"))?;
            let keys: Vec<Value> = keys
                .iter()
                .map(|k| {
                    let mut k = k.as_array().cloned().ok_or_else(|| anyhow!("vector keys look like [t, [x, y]]"))?;
                    let v = k.get(1).and_then(Value::as_array).ok_or_else(|| anyhow!("vector keys look like [t, [x, y]]"))?;
                    let z = v.get(2).and_then(Value::as_f64).unwrap_or(0.0) + dz;
                    k[1] = json!([v[0], v[1], z]);
                    Ok(Value::Array(k))
                })
                .collect::<Result<_>>()?;
            o.insert("keys".into(), Value::Array(keys));
            Value::Object(o)
        }
        other => bail!("pos {other} has a form extrude cannot offset; write it as [x, y, z]"),
    })
}

fn add(p: &Value, dz: f64) -> Result<Value> {
    Ok(match p {
        Value::Number(n) => json!(n.as_f64().unwrap_or(0.0) + dz),
        Value::String(s) => json!(format!("({s}) + {dz}")),
        Value::Array(keys) => Value::Array(
            keys.iter()
                .map(|k| {
                    let mut k = k.as_array().cloned().ok_or_else(|| anyhow!("keys look like [t, v]"))?;
                    k[1] = json!(k[1].as_f64().unwrap_or(0.0) + dz);
                    Ok(Value::Array(k))
                })
                .collect::<Result<_>>()?,
        ),
        Value::Object(o) if o.contains_key("from") => {
            let mut o = o.clone();
            for k in ["from", "to"] {
                let v = o.get(k).and_then(Value::as_f64).ok_or_else(|| anyhow!("tween needs numbers"))?;
                o.insert(k.into(), json!(v + dz));
            }
            Value::Object(o)
        }
        other => bail!("a z of {other} cannot be offset; use a number, an expression, keys or a tween"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slices_step_back_behind_the_face() {
        let mut v = json!({"layers": [{"type": "text", "id": "t", "text": "88888", "three_d": true, "pos": [960, 540], "color": "#ffffff",
            "extrude": {"depth": 30, "steps": 3, "color": "#808080"}}]});
        expand(&mut v).unwrap();
        let ls = v["layers"].as_array().unwrap();
        assert_eq!(ls.len(), 4);
        assert_eq!(ls[0]["pos"], json!([960, 540, 30.0]));
        assert_eq!(ls[2]["pos"], json!([960, 540, 10.0]));
        assert_eq!(ls[0]["color"], "#464646");
        assert_eq!(ls[3]["id"], "t");
        assert!(ls[3].get("extrude").is_none());
    }

    #[test]
    fn flat_layers_are_refused() {
        let mut v = json!({"layers": [{"type": "text", "text": "x", "pos": [0, 0], "extrude": {}}]});
        assert!(format!("{:#}", expand(&mut v).unwrap_err()).contains("three_d"));
    }
}
