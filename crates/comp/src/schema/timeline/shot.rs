//! One shot to one `precomp` layer: its source (layers, a media file, a comp or a block), speed, move and camera.

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde_json::{json, Map, Value};

use super::{camera::camera_layer, moves::apply_move, round, speed::integrate, time, Shot};
use crate::anim::prop::Prop;

const SHOT_KEYS: &[&str] = &["id", "start", "dur", "in", "comp", "block", "vars", "layers", "src", "fit", "focus", "bg", "speed", "offset", "move", "camera", "fx", "layout", "loop", "blend", "masks", "opacity"];

/// Layer id of the camera a shot gets when it has 3D layers and none of its own.
pub const DEFAULT_CAMERA: &str = "default_camera";

/// A shot with `three_d` layers but no camera would draw them flat; give it the standard camera (z = 0 is 1:1, negative z
/// is toward the lens, positive z away).
fn default_camera(layers: &[Value]) -> Option<Value> {
    let is = |l: &Value, key: &str, want: &Value| l.get(key) == Some(want);
    let has_3d = layers.iter().any(|l| is(l, "three_d", &json!(true)));
    let has_camera = layers.iter().any(|l| is(l, "type", &json!("camera")));
    (has_3d && !has_camera).then(|| json!({"type": "camera", "id": DEFAULT_CAMERA}))
}

pub fn shot_layer(shot: &Shot, w: f64, h: f64, fps: f64, comps: &mut Vec<(String, Value)>) -> Result<Map<String, Value>> {
    let d = &shot.def;
    if let Some(message) = crate::schema::fields::stray_keys(&format!("shot {:?}", shot.id), d.keys(), SHOT_KEYS) {
        bail!("{message}");
    }
    let offset = d.get("offset").map_or(Ok(0.0), |v| time(v, "offset"))?;
    // the speed curve, as source seconds per output second
    let speed = d.get("speed").cloned().unwrap_or(json!(1));
    let (remap, src_dur) = match &speed {
        Value::Number(n) => {
            let s = n.as_f64().unwrap_or(1.0);
            ensure!(s > 0.0, "a constant speed must be positive (use keys with 0 to freeze)");
            (None, offset + shot.dur * s)
        }
        v => {
            let prop = Prop::parse(v).context("speed")?;
            let (keys, total) = integrate(&prop, shot.dur, fps, offset)?;
            (Some(keys), total)
        }
    };
    let src_dur = src_dur.max(1.0 / fps);

    let mut layer = Map::new();
    layer.insert("type".into(), json!("precomp"));
    layer.insert("id".into(), json!(shot.id));
    layer.insert("start".into(), round(shot.start));
    layer.insert("dur".into(), round(shot.dur));
    layer.insert("pos".into(), json!([w / 2.0, h / 2.0]));

    let camera = d.get("camera").map(|c| camera_layer(c, shot.dur, w, h)).transpose().context("camera")?;
    let own = format!("shot:{}", shot.id);
    if let Some(layers) = d.get("layers") {
        let mut ls = layers.as_array().ok_or_else(|| anyhow!("\"layers\" must be a list"))?.clone();
        let camera = camera.or_else(|| default_camera(&ls));
        if let Some(cam) = camera {
            ls.insert(0, cam);
        }
        let mut c = json!({"width": w as u32, "height": h as u32, "dur": round(src_dur + 1.0 / fps), "layers": ls});
        if let Some(bg) = d.get("bg") {
            c["bg"] = bg.clone();
        }
        comps.push((own.clone(), c));
        layer.insert("comp".into(), json!(own));
    } else if let Some(src) = d.get("src").and_then(Value::as_str) {
        let lower = src.to_lowercase();
        let kind = if [".mp4", ".mov", ".m4v", ".webm", ".mkv"].iter().any(|x| lower.ends_with(x)) { "video" } else { "image" };
        let mut media = json!({"type": kind, "src": src, "size": [w, h], "pos": [w / 2.0, h / 2.0], "fit": d.get("fit").cloned().unwrap_or(json!("cover"))});
        if let Some(f) = d.get("focus") {
            media["focus"] = f.clone();
        }
        if kind == "video" {
            media["mute"] = json!(true);
        }
        let mut ls = vec![media];
        if let Some(cam) = camera {
            ls.insert(0, cam);
        }
        comps.push((own.clone(), json!({"width": w as u32, "height": h as u32, "dur": round(src_dur + 1.0 / fps), "layers": ls})));
        layer.insert("comp".into(), json!(own));
    } else if let Some(name) = d.get("comp").and_then(Value::as_str) {
        ensure!(camera.is_none(), "a 3D \"camera\" move needs the shot's own \"layers\" (put the camera in comp {name:?} instead)");
        layer.insert("comp".into(), json!(name));
        if let Some(v) = d.get("vars") {
            layer.insert("vars".into(), v.clone());
        }
    } else if let Some(b) = d.get("block") {
        ensure!(camera.is_none(), "a 3D \"camera\" move needs the shot's own \"layers\"");
        layer.insert("block".into(), b.clone());
        layer.insert("vars".into(), d.get("vars").cloned().unwrap_or(json!({})));
    } else {
        bail!("a shot needs one of \"layers\", \"src\", \"comp\" or \"block\"");
    }

    match remap {
        Some(keys) => {
            layer.insert("remap".into(), keys);
        }
        None => {
            if let Some(s) = speed.as_f64().filter(|s| (s - 1.0).abs() > 1e-12) {
                layer.insert("speed".into(), json!(s));
            }
            if offset != 0.0 {
                layer.insert("offset".into(), json!(offset));
            }
        }
    }
    for k in ["fx", "layout", "loop", "blend", "masks", "opacity"] {
        if let Some(v) = d.get(k) {
            layer.insert(k.into(), v.clone());
        }
    }
    if let Some(mv) = d.get("move") {
        apply_move(&mut layer, mv, shot.dur, w, h).context("move")?;
    }
    Ok(layer)
}
