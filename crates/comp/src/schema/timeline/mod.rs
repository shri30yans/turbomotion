//! The edit timeline: a film written as tracks, the way an editor thinks, instead of as a flat list of layers.
//!
//! ```json
//! "timeline": {
//!   "transition": {"type": "push_left", "dur": 0.6},         default between shots
//!   "sfx": {"volume": 0.45},                                  sound on every transition (false turns it off)
//!   "mblur": true,                                            true motion blur on every shot (or {"samples": 8, "shutter": 180})
//!   "shots": [
//!     {"id": "hook", "dur": 4, "layers": [...], "move": "push_in"},
//!     {"id": "map",  "dur": 6, "comp": "globe", "in": "pan_out_left", "speed": [[0, 1], [2, 0.3, "sine.inOut"], [4, 1]]},
//!     {"id": "deep", "dur": 5, "layers": [...], "camera": {"move": "orbit", "deg": 24}, "in": {"type": "zoom_through", "dur": 0.8}}
//!   ],
//!   "overlays": [{"shot": "map", "at": 0.5, "dur": 3, "type": "precomp", "block": "lower_third", "vars": {...}}],
//!   "hits": [{"at": "map.start+2.1", "cue": "boom"}]
//! }
//! ```
//!
//! Shots run one after another; a transition overlaps the two shots by its length (Remotion's TransitionSeries
//! model). A shot with an explicit `start` keeps it (narration-locked cuts) and the shot before it is extended to
//! cover the transition. Each shot becomes a `precomp` layer whose id is the shot id, so `"hook.end+0.2"` works in
//! overlays, and a marker of the same name. Transitions are the shot presets of `eval::transit` placed on the
//! outgoing shot's `exit` and the incoming shot's `enter`, plus a helper layer for dips and floods. Speed ramps
//! integrate to a `remap` curve. Nothing here draws: the output is ordinary composition JSON, so `inspect`,
//! `check` and the layout gate see every transition exactly as it renders.

mod camera;
mod catalogue;
mod moves;
mod shot;
mod speed;
mod timing;
mod tracks;
mod transition;
#[cfg(test)]
mod tests;

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde_json::{json, Map, Value};

pub use catalogue::{CAMERA_MOVES, MOVES, TRANSITIONS};
use shot::shot_layer;
use transition::{enter_of, exit_of, parse_trans, Trans};

pub(crate) struct Shot {
    pub id: String,
    pub start: f64,
    pub dur: f64,
    pub def: Map<String, Value>,
    pub into: Option<Trans>,
}

impl Shot {
    pub fn end(&self) -> f64 {
        self.start + self.dur
    }
}

/// The frame and the transition sound settings every part of the expansion needs.
pub(crate) struct Frame {
    pub w: f64,
    pub h: f64,
    pub fps: f64,
    /// `None` when the timeline says `"sfx": false`.
    pub sfx_volume: Option<f64>,
    /// The film's tempo, so a shot's own comp keeps the film's beat grid.
    pub bpm: Option<f64>,
    pub beat_offset: f64,
    /// `"mblur"` on the timeline: every shot is sampled across the shutter, so a transition smears like a real camera.
    pub mblur: bool,
}

pub(crate) fn time(v: &Value, what: &str) -> Result<f64> {
    match v {
        Value::Number(n) => n.as_f64().ok_or_else(|| anyhow!("{what} is not a finite number")),
        Value::String(s) => {
            let s = s.trim();
            if let Some(x) = s.strip_suffix('s') {
                x.trim().parse().map_err(|_| anyhow!("{what}: {s:?} is not seconds"))
            } else {
                bail!("{what} must be seconds (a number or \"1.5s\"); shot times are worked out before other times, so they cannot name layers")
            }
        }
        other => bail!("{what} must be seconds, not {other}"),
    }
}

pub(crate) fn round(x: f64) -> Value {
    json!((x * 1e5).round() / 1e5)
}

const FIELDS: &[&str] = &["transition", "sfx", "shots", "overlays", "hits", "stage", "mblur"];

pub fn expand(root: &mut Value) -> Result<()> {
    let Some(o) = root.as_object_mut() else { return Ok(()) };
    let Some(tl) = o.remove("timeline") else { return Ok(()) };
    let tl = tl.as_object().ok_or_else(|| anyhow!("\"timeline\" must be an object"))?.clone();
    if let Some(message) = super::fields::stray_keys("timeline", tl.keys(), FIELDS) {
        bail!("{message}");
    }
    let frame = Frame {
        w: o.get("width").and_then(Value::as_f64).unwrap_or(1920.0),
        h: o.get("height").and_then(Value::as_f64).unwrap_or(1080.0),
        fps: o.get("fps").and_then(Value::as_f64).unwrap_or(30.0).max(1.0),
        bpm: o.get("bpm").and_then(Value::as_f64),
        beat_offset: o.get("beat_offset").and_then(Value::as_f64).unwrap_or(0.0),
        mblur: matches!(tl.get("mblur"), Some(Value::Bool(true) | Value::Object(_))),
        sfx_volume: match tl.get("sfx") {
            Some(Value::Bool(false)) => None,
            s => Some(s.and_then(|s| s.get("volume")).and_then(Value::as_f64).unwrap_or(0.45)),
        },
    };
    if let Some(Value::Object(m)) = tl.get("mblur") {
        o.insert("mblur".into(), Value::Object(m.clone()));
    }
    let default = tl.get("transition").map(|t| parse_trans(t, None)).transpose().context("timeline transition")?;
    let list = tl.get("shots").and_then(Value::as_array).ok_or_else(|| anyhow!("timeline needs \"shots\": [...]"))?;
    let shots = timing::schedule(list, default.as_ref())?;
    let end = shots.iter().map(Shot::end).fold(0.0, f64::max);

    let mut comps: Vec<(String, Value)> = vec![];
    let mut layers: Vec<Value> = vec![];
    let mut audio: Vec<Value> = vec![];
    if let Some(stage) = tl.get("stage") {
        layers.push(json!({"type": "solid", "id": "timeline_stage", "color": stage, "dur": round(end), "layout": "backdrop"}));
    }
    let mut used = std::collections::HashMap::new();
    for (i, shot) in shots.iter().enumerate() {
        let next = shots.get(i + 1).and_then(|n| n.into.as_ref());
        layers.extend(shot_layers(shot, next, &frame, &mut comps, &mut audio, &mut used).with_context(|| format!("shot {:?}", shot.id))?);
    }
    let frame_facts = json!({"width": o.get("width"), "height": o.get("height"), "letterbox": o.get("letterbox")});
    layers.extend(super::letterbox::of_root(&frame_facts)?.unwrap_or_default());
    if let Some(list) = tl.get("overlays") {
        layers.extend(tracks::overlays(list, &shots)?);
    }
    if let Some(list) = tl.get("hits") {
        audio.extend(tracks::hits(list, frame.sfx_volume.unwrap_or(0.45))?);
    }
    layers.extend(audio);
    let markers = shots.iter().map(|s| json!({"t": round(s.start), "name": s.id})).collect();
    splice(o, layers, markers, comps, end)
}

/// A shot's precomp layer with its exit (toward `next`) and entrance, the helper layers of its transition, and the
/// transition's sound.
fn shot_layers(shot: &Shot, next: Option<&Trans>, f: &Frame, comps: &mut Vec<(String, Value)>, audio: &mut Vec<Value>, used: &mut std::collections::HashMap<String, usize>) -> Result<Vec<Value>> {
    let mut layer = shot_layer(shot, f.w, f.h, f.fps, comps)?;
    if f.mblur {
        layer.insert("mblur".into(), json!(true));
    }
    // a shot's own comp starts at 0, so its beat grid is the film's shifted back by the shot's start
    if let (Some(bpm), Some((_, comp))) = (f.bpm, comps.last_mut().filter(|(n, _)| *n == format!("shot:{}", shot.id))) {
        comp["bpm"] = json!(bpm);
        comp["beat_offset"] = round(f.beat_offset - shot.start);
    }
    if let Some(exit) = next.filter(|t| t.dur > 0.0).and_then(|t| exit_of(t, f.w, f.h)) {
        layer.insert("exit".into(), exit);
    }
    let Some(t) = shot.into.as_ref().filter(|t| t.dur > 0.0) else { return Ok(vec![Value::Object(layer)]) };
    if let Some(enter) = enter_of(t, f.w, f.h) {
        layer.insert("enter".into(), enter);
    }
    if let Some(sound) = f.sfx_volume.and_then(|v| t.sound(shot.start, v, used)) {
        audio.push(sound);
    }
    let (mut out, after) = t.helpers(shot.start, f.w, f.h);
    out.push(Value::Object(layer));
    out.extend(after);
    Ok(out)
}

/// Timeline layers go under whatever the file already had; markers and comps are added beside the existing ones.
fn splice(o: &mut Map<String, Value>, mut layers: Vec<Value>, markers: Vec<Value>, comps: Vec<(String, Value)>, end: f64) -> Result<()> {
    if o.get("dur").and_then(Value::as_f64).is_none() {
        o.insert("dur".into(), round(end));
    }
    layers.extend(o.remove("layers").and_then(|l| l.as_array().cloned()).unwrap_or_default());
    o.insert("layers".into(), Value::Array(layers));
    let m = o.entry("markers").or_insert_with(|| Value::Array(vec![]));
    m.as_array_mut().ok_or_else(|| anyhow!("\"markers\" must be a list"))?.extend(markers);
    let cs = o.entry("comps").or_insert_with(|| Value::Object(Map::new()));
    let cs = cs.as_object_mut().ok_or_else(|| anyhow!("\"comps\" must be an object"))?;
    for (name, c) in comps {
        ensure!(!cs.contains_key(&name), "timeline: a comp named {name:?} already exists");
        cs.insert(name, c);
    }
    Ok(())
}
