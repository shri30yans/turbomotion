//! 3D camera moves placed inside a shot: orbit, dolly, dolly zoom, crane, truck.

use anyhow::{anyhow, ensure, Result};
use serde_json::{json, Value};

use super::{catalogue::CAMERA_MOVES, round};

pub fn camera_layer(c: &Value, dur: f64, w: f64, h: f64) -> Result<Value> {
    let o = c.as_object().ok_or_else(|| anyhow!("camera is {{\"move\": \"orbit\", ...}}"))?;
    let kind = o.get("move").and_then(Value::as_str).ok_or_else(|| anyhow!("camera needs \"move\""))?;
    ensure!(CAMERA_MOVES.iter().any(|m| m.0 == kind), "unknown camera move {kind:?}; moves are {}", CAMERA_MOVES.iter().map(|m| m.0).collect::<Vec<_>>().join(", "));
    let ease = o.get("ease").and_then(Value::as_str).unwrap_or("sine.inOut");
    let z = 1.5 * w;
    let (cx, cy) = (w / 2.0, h / 2.0);
    let amount = o.get("amount").and_then(Value::as_f64);
    let tw = |from: f64, to: f64| json!({"from": round(from), "to": round(to), "at": 0, "dur": round(dur), "ease": ease});
    let mut cam = json!({"type": "camera", "zoom": round(z)});
    match kind {
        "orbit" => {
            let deg = o.get("deg").and_then(Value::as_f64).unwrap_or(30.0);
            let a = format!("radians(ease(\"{ease}\", time, 0, {dur:.4}, {:.4}, {:.4}))", -deg / 2.0, deg / 2.0);
            cam["pos"] = json!([format!("{cx} + {z} * sin({a})"), cy, format!("-{z} * cos({a})")]);
        }
        "dolly_in" | "dolly_out" => {
            let k = amount.unwrap_or(0.3);
            let (a, b) = if kind == "dolly_in" { (-z, -z * (1.0 - k)) } else { (-z * (1.0 - k), -z) };
            cam["pos"] = json!([cx, cy, tw(a, b)]);
            cam["zoom"] = json!(round(z * (1.0 - k / 2.0)));
        }
        "dolly_zoom" => {
            let k = amount.unwrap_or(0.45);
            cam["pos"] = json!([cx, cy, tw(-z, -z * (1.0 - k))]);
            cam["zoom"] = tw(z, z * (1.0 - k));
        }
        "crane_up" | "crane_down" => {
            let k = amount.unwrap_or(0.35) * h;
            let (a, b) = if kind == "crane_up" { (cy + k, cy - k) } else { (cy - k, cy + k) };
            cam["pos"] = json!([cx, tw(a, b), -z]);
        }
        _ => {
            let k = amount.unwrap_or(0.18) * w;
            let (a, b) = if kind == "truck_left" { (cx + k, cx - k) } else { (cx - k, cx + k) };
            cam["pos"] = json!([tw(a, b), cy, -z]);
            cam["poi"] = json!([tw(a, b), cy, 0]);
        }
    }
    if let Some(dof) = o.get("dof") {
        cam["dof"] = dof.clone();
    }
    Ok(cam)
}
