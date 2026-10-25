//! The tracks above the shots: overlays (titles, captions, callouts, any layer) placed by shot, and sound hits.

use anyhow::{anyhow, ensure, Result};
use serde_json::{json, Value};

use super::{round, time, Shot};

/// An overlay is any layer; `"shot": "id", "at": 0.5` places it relative to that shot and lasts to the shot's end
/// unless it says otherwise.
pub fn overlays(list: &Value, shots: &[Shot]) -> Result<Vec<Value>> {
    let list = list.as_array().ok_or_else(|| anyhow!("\"overlays\" must be a list of layers"))?;
    list.iter()
        .enumerate()
        .map(|(i, ov)| {
            let mut ov = ov.as_object().ok_or_else(|| anyhow!("overlay {i} must be a layer object"))?.clone();
            if let Some(sid) = ov.remove("shot") {
                let sid = sid.as_str().ok_or_else(|| anyhow!("overlay {i}: \"shot\" is a shot id"))?;
                let shot = shots.iter().find(|s| s.id == sid).ok_or_else(|| anyhow!("overlay {i}: no shot {sid:?}"))?;
                let at = ov.remove("at").map_or(Ok(0.0), |a| time(&a, "overlay at"))?;
                ensure!(!ov.contains_key("start"), "overlay {i}: give \"shot\" + \"at\", or \"start\", not both");
                ov.insert("start".into(), round(shot.start + at));
                if !ov.contains_key("dur") && !ov.contains_key("end") {
                    ov.insert("dur".into(), round((shot.dur - at).max(0.01)));
                }
            }
            Ok(Value::Object(ov))
        })
        .collect()
}

/// `{"at": "map.start+2.1", "cue": "boom", "volume": 0.6}`: a kit cue (or a sound file) on an exact moment.
pub fn hits(list: &Value, volume: f64) -> Result<Vec<Value>> {
    let list = list.as_array().ok_or_else(|| anyhow!("\"hits\" must be a list"))?;
    list.iter()
        .enumerate()
        .map(|(i, hit)| {
            let cue = hit.get("cue").and_then(Value::as_str).ok_or_else(|| anyhow!("hit {i} needs \"cue\""))?.trim_start_matches("kit:");
            let at = hit.get("at").cloned().ok_or_else(|| anyhow!("hit {i} needs \"at\""))?;
            let src = if cue.contains('.') { cue.to_string() } else { format!("kit:{cue}") };
            Ok(json!({"type": "audio", "src": src, "start": at, "volume": hit.get("volume").cloned().unwrap_or(json!(volume))}))
        })
        .collect()
}
