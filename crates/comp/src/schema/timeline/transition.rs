//! Shot-to-shot transitions: parsing, and turning one into the outgoing shot's `exit`, the incoming shot's `enter`
//! and, for dips and floods, a helper layer.

use anyhow::{anyhow, bail, ensure, Result};
use serde_json::{json, Map, Value};

use super::{
    catalogue::{resolve_cue, TRANSITIONS},
    round, time,
};

#[derive(Clone)]
pub struct Trans {
    pub kind: String,
    pub dur: f64,
    pub ease: Option<String>,
    pub color: Value,
    pub sfx: Option<String>,
    /// `window`: the small portal as `[centre x, centre y, width]` (fractions of the frame).
    pub portal: Option<Value>,
    /// `window`: the portal's corner radius in pixels.
    pub radius: Option<f64>,
}

pub fn parse_trans(v: &Value, default: Option<&Trans>) -> Result<Trans> {
    let (kind, o) = match v {
        Value::String(s) => (s.clone(), Map::new()),
        Value::Object(o) => (o.get("type").and_then(Value::as_str).ok_or_else(|| anyhow!("a transition object needs \"type\""))?.to_string(), o.clone()),
        other => bail!("a transition is a name or {{\"type\": ..., \"dur\": ...}}, not {other}"),
    };
    for k in o.keys() {
        ensure!(["type", "dur", "ease", "color", "sfx", "portal", "radius"].contains(&k.as_str()), "transition: unknown field {k:?}; fields are type, dur, ease, color, sfx, portal, radius");
    }
    let entry = TRANSITIONS.iter().find(|t| t.0 == kind).ok_or_else(|| anyhow!("unknown transition {kind:?}; transitions are {}", TRANSITIONS.iter().map(|t| t.0).collect::<Vec<_>>().join(", ")))?;
    let default_dur = default.map_or(0.6, |d| d.dur);
    let dur = if kind == "cut" { 0.0 } else { o.get("dur").map_or(Ok(default_dur), |d| time(d, "transition dur"))? };
    ensure!(dur >= 0.0, "transition dur must not be negative");
    let sfx = match o.get("sfx") {
        Some(Value::Bool(false)) | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.trim_start_matches("kit:").to_string()),
        Some(Value::Bool(true)) | None => entry.2.map(String::from),
        Some(other) => bail!("transition sfx is a cue name or false, not {other}"),
    };
    Ok(Trans {
        kind,
        dur,
        ease: o.get("ease").and_then(Value::as_str).map(String::from).or_else(|| default.and_then(|d| d.ease.clone())),
        color: o.get("color").cloned().unwrap_or_else(|| json!("#000000")),
        sfx,
        portal: o.get("portal").cloned(),
        radius: o.get("radius").and_then(Value::as_f64),
    })
}

/// One preset entry (`enter`/`exit`) with the transition's length, ease and optional travel distance.
fn transit(preset: &str, t: &Trans, dist: Option<f64>) -> Value {
    let mut v = json!({"preset": preset, "dur": round(t.dur)});
    if let Some(e) = &t.ease {
        v["ease"] = json!(e);
    }
    if let Some(d) = dist {
        v["dist"] = round(d);
    }
    if t.kind == "window" {
        if let Some(p) = &t.portal {
            v["portal"] = p.clone();
        }
        if let Some(r) = t.radius {
            v["radius"] = json!(r);
        }
    }
    v
}

fn span(kind: &str, w: f64, h: f64) -> f64 {
    if kind.ends_with("_up") || kind.ends_with("_down") { h } else { w }
}

/// What the incoming shot does.
pub fn enter_of(t: &Trans, w: f64, h: f64) -> Option<Value> {
    let k = t.kind.as_str();
    Some(match k {
        "cut" | "dip_black" | "dip_white" | "dip" => return None,
        "dissolve" => transit("fade", t, None),
        "flood" => {
            let mut v = transit("iris", t, None);
            v["dur"] = round(t.dur * 0.55);
            v["delay"] = round(t.dur * 0.45);
            v["ease"] = json!(t.ease.clone().unwrap_or_else(|| "expo.out".into()));
            v
        }
        "iris" | "wipe_left" | "wipe_right" | "wipe_up" | "wipe_down" => {
            let mut v = transit(k, t, None);
            if t.ease.is_none() {
                v["ease"] = json!("power3.inOut");
            }
            v
        }
        _ if k.starts_with("slide_over_") => transit(&k.replace("slide_over_", "push_"), t, Some(span(k, w, h))),
        "cross_zoom" | "window" => transit(k, t, None),
        _ => transit(k, t, Some(span(k, w, h))),
    })
}

/// What the outgoing shot does while the next one enters.
pub fn exit_of(t: &Trans, w: f64, h: f64) -> Option<Value> {
    let k = t.kind.as_str();
    Some(match k {
        _ if k.starts_with("push_") || k.starts_with("whip_pan_") || k.starts_with("pan_out_") => transit(k, t, Some(span(k, w, h))),
        _ if k.starts_with("slide_over_") => transit("recede", t, None),
        "zoom_through" | "zoom_back" | "cross_zoom" | "window" | "blur_dissolve" | "glitch" => transit(k, t, None),
        _ => return None,
    })
}

impl Trans {
    /// Layers the transition needs around the incoming shot: (below it, above it). A flood disc sits below the
    /// shot that irises open on it; a dip's colour sits above both shots.
    pub fn helpers(&self, s: f64, w: f64, h: f64) -> (Vec<Value>, Vec<Value>) {
        let d = self.dur;
        match self.kind.as_str() {
            "dip_black" | "dip_white" | "dip" => {
                let color = match self.kind.as_str() {
                    "dip_black" => json!("#000000"),
                    "dip_white" => json!("#ffffff"),
                    _ => self.color.clone(),
                };
                let solid = json!({"type": "solid", "color": color, "start": round(s), "dur": round(d), "layout": "ignore",
                    "opacity": [[0, 0], [round(d / 2.0), 1, "sine.inOut"], [round(d), 0, "sine.inOut"]]});
                (vec![], vec![solid])
            }
            "flood" => {
                // radius reaching the corners from the centre; the ellipse is [cx, cy, rx, ry]
                let r = (w * w + h * h).sqrt() / 2.0 + 4.0;
                let disc = json!({"type": "shape", "start": round(s), "dur": round(d), "pos": [w / 2.0, h / 2.0], "layout": "ignore",
                    "items": [{"ellipse": [0, 0, r, r], "fill": self.color}],
                    "scale": [[0, 0], [round(d * 0.5), 1, "expo.in"]]});
                (vec![disc], vec![])
            }
            _ => (vec![], vec![]),
        }
    }

    /// The transition's sound, placed so the cue's loudest moment lands on the transition's midpoint.
    ///
    /// `used` counts how often each cue has been used so far in the film, so a cue that names a family or a pool steps
    /// through its variants and consecutive transitions sound different.
    pub fn sound(&self, s: f64, volume: f64, used: &mut std::collections::HashMap<String, usize>) -> Option<Value> {
        let cue = self.sfx.as_ref()?;
        let n = used.entry(cue.clone()).or_insert(0);
        let (exact, peak) = resolve_cue(cue, *n);
        *n += 1;
        let at = (s + self.dur / 2.0 - peak).max(0.0);
        Some(json!({"type": "audio", "src": format!("kit:{exact}"), "start": round(at), "volume": volume}))
    }
}
