//! 2D moves of a whole shot over its life: push, pull, drift, Ken Burns, handheld, punch-ins.

use anyhow::{anyhow, bail, ensure, Result};
use serde_json::{json, Map, Value};

use super::{catalogue::MOVES, round, time};

pub fn apply_move(layer: &mut Map<String, Value>, mv: &Value, dur: f64, w: f64, h: f64) -> Result<()> {
    let (kind, o) = match mv {
        Value::String(s) => (s.clone(), Map::new()),
        Value::Object(o) => (o.get("type").and_then(Value::as_str).ok_or_else(|| anyhow!("a move object needs \"type\""))?.to_string(), o.clone()),
        other => bail!("a move is a name or {{\"type\": ...}}, not {other}"),
    };
    ensure!(MOVES.iter().any(|m| m.0 == kind), "unknown move {kind:?}; moves are {}", MOVES.iter().map(|m| m.0).collect::<Vec<_>>().join(", "));
    let amount = o.get("amount").and_then(Value::as_f64).unwrap_or(if kind == "punch" { 0.1 } else { 0.06 });
    let ease = o.get("ease").and_then(Value::as_str).unwrap_or("sine.inOut").to_string();
    let tween = |from: f64, to: f64| json!({"from": round(from), "to": round(to), "at": 0, "dur": round(dur), "ease": ease});
    let (cx, cy) = (w / 2.0, h / 2.0);
    match kind.as_str() {
        "push_in" => {
            layer.insert("scale".into(), tween(1.0, 1.0 + amount));
        }
        "pull_out" => {
            layer.insert("scale".into(), tween(1.0 + amount, 1.0));
        }
        "drift_left" | "drift_right" | "drift_up" | "drift_down" | "ken_burns" => {
            let travel = amount * 0.5;
            let (dx, dy) = match kind.as_str() {
                "drift_left" | "ken_burns" => (-travel * w, 0.0),
                "drift_right" => (travel * w, 0.0),
                "drift_up" => (0.0, -travel * h),
                _ => (0.0, travel * h),
            };
            layer.insert("pos".into(), json!([tween(cx - dx / 2.0, cx + dx / 2.0), tween(cy - dy / 2.0, cy + dy / 2.0)]));
            let s = if kind == "ken_burns" { tween(1.0 + amount, 1.0 + 2.0 * amount) } else { json!(round(1.0 + amount)) };
            layer.insert("scale".into(), s);
        }
        "handheld" => {
            let a = (amount * 100.0).max(1.0);
            layer.insert("pos".into(), json!([format!("{cx} + wiggle(0.6, {a:.2})"), format!("{cy} + wiggle(0.5, {:.2})", a * 0.8)]));
            layer.insert("rotation".into(), json!(format!("wiggle(0.4, {:.3})", a * 0.04)));
            layer.insert("scale".into(), json!(round(1.0 + amount * 0.6)));
        }
        "punch" => {
            let at = o.get("at").and_then(Value::as_array).ok_or_else(|| anyhow!("punch needs \"at\": [seconds into the shot, ...]"))?;
            let mut keys = vec![json!([0, 1, "none"])];
            for (k, t) in at.iter().enumerate() {
                let t = time(t, "punch at")?;
                ensure!(t >= 0.0 && t < dur, "punch at {t}s is outside the shot (0..{dur}s)");
                let lvl = |n: usize| round(1.0 + amount * n as f64);
                keys.push(json!([round(t), lvl(k), "none"]));
                keys.push(json!([round((t + 0.28).min(dur)), lvl(k + 1), "expo.out"]));
            }
            layer.insert("scale".into(), Value::Array(keys));
        }
        _ => unreachable!("checked above"),
    }
    Ok(())
}
