//! Isolate a shot before timeline expansion, preserving its camera, effects, overlays and local sound hits.
use anyhow::{anyhow, ensure, Result};
use serde_json::{json, Value};

/// Shot previews start at zero and omit film-level layers and incoming transitions.
/// Other named precompositions and asset configuration remain available.
pub fn isolate(film: &mut Value, id: &str) -> Result<()> {
    let timeline = film.get("timeline").and_then(Value::as_object)
        .ok_or_else(|| anyhow!("--shot needs a film.timeline({{ shots: [...] }}); this film has no timeline"))?;
    let shots = timeline.get("shots").and_then(Value::as_array)
        .ok_or_else(|| anyhow!("timeline.shots must be a list"))?;
    let available = shots.iter().filter_map(|s| s["id"].as_str()).collect::<Vec<_>>().join(", ");
    let mut shot = shots.iter().find(|s| s["id"].as_str() == Some(id)).cloned()
        .ok_or_else(|| anyhow!("unknown shot {id:?}; available shots: {available}"))?;
    let dur = shot["dur"].as_f64().ok_or_else(|| anyhow!("shot {id:?}.dur must be seconds"))?;
    ensure!(dur.is_finite() && dur > 0.0, "shot {id:?}.dur must be positive seconds");
    let object = shot.as_object_mut().ok_or_else(|| anyhow!("shot must be an object"))?;
    object.remove("in");
    object.insert("start".into(), json!(0));
    let mut isolated = timeline.clone();
    isolated.remove("transition");
    isolated.insert("shots".into(), json!([shot]));
    if let Some(overlays) = isolated.get_mut("overlays").and_then(Value::as_array_mut) {
        overlays.retain(|o| o["shot"].as_str() == Some(id));
    }
    let prefix = format!("{id}.start");
    if let Some(hits) = isolated.get_mut("hits").and_then(Value::as_array_mut) {
        hits.retain(|h| h["at"].as_str().is_some_and(|at| at == prefix || at.starts_with(&format!("{prefix}+"))));
    }
    let root = film.as_object_mut().ok_or_else(|| anyhow!("film must be an object"))?;
    root.remove("layers");
    root.insert("dur".into(), json!(dur));
    root.insert("timeline".into(), Value::Object(isolated));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shot_selection_preserves_local_content_and_omits_other_shots() {
        let mut film = json!({"width": 320, "height": 240, "fps": 30, "dur": 99, "layers": [{"id":"film"}],
            "timeline": {"transition":{"type":"dissolve","dur":0.5}, "sfx":false,
                "shots":[{"id":"first","dur":2},{"id":"second","start":20,"dur":3,"in":"dissolve","layers":[]}],
                "overlays":[{"shot":"first"},{"shot":"second"}],
                "hits":[{"at":"first.start+1"},{"at":"second.start+0.5"},{"at":20}]}});
        isolate(&mut film, "second").unwrap();
        assert_eq!(film["dur"].as_f64(), Some(3.0));
        assert!(film.get("layers").is_none());
        assert_eq!(film["timeline"]["shots"][0]["start"], 0);
        assert!(film["timeline"].get("transition").is_none());
        assert!(film["timeline"]["shots"][0].get("in").is_none());
        assert_eq!(film["timeline"]["overlays"].as_array().unwrap().len(), 1);
        assert_eq!(film["timeline"]["hits"].as_array().unwrap().len(), 1);
        assert!(isolate(&mut film, "missing").unwrap_err().to_string().contains("available shots: second"));
    }
}
