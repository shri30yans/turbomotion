//! Re-aim a finished composition at another aspect ratio (a 16:9 film becomes a 9:16 short) by wrapping it:
//! the original becomes a nested composition and the new root plays it scaled and positioned. Nothing inside the
//! original is edited, so the two versions never drift apart.

use anyhow::{bail, ensure, Result};
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Fill the new frame and crop the sides, following `focus`.
    #[default]
    Cover,
    /// Fit the whole picture and fill the bars with a blurred, darkened copy.
    Contain,
}

impl std::str::FromStr for Mode {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "cover" => Ok(Mode::Cover),
            "contain" => Ok(Mode::Contain),
            other => bail!("unknown reframe mode {other:?} (use cover or contain)"),
        }
    }
}

pub struct Options {
    pub width: u32,
    pub height: u32,
    pub mode: Mode,
    /// Where the crop is centred when the source is wider than the target: 0 = left edge, 0.5 = middle, 1 = right edge.
    pub focus: f64,
}

/// Keys that describe the whole project and so move up to the new root (they stay on the nested copy too, which is harmless).
const PROJECT_KEYS: &[&str] = &["fps", "bg", "bpm", "beat_offset", "markers", "vars", "fonts", "palette", "type", "image", "surface", "space", "motion", "mblur"];

pub fn reframe(source: &Value, o: &Options) -> Result<Value> {
    ensure!(o.width > 0 && o.height > 0, "the target size must be positive");
    ensure!((0.0..=1.0).contains(&o.focus), "focus is a fraction between 0 and 1");
    let root = source.as_object().ok_or_else(|| anyhow::anyhow!("a composition file is a JSON object"))?;
    let num = |k: &str, d: f64| root.get(k).and_then(Value::as_f64).unwrap_or(d);
    let (sw, sh, dur) = (num("width", 1920.0), num("height", 1080.0), num("dur", 5.0));
    let (tw, th) = (f64::from(o.width), f64::from(o.height));

    let mut nested = root.clone();
    let comps = match nested.remove("comps") {
        Some(Value::Object(m)) => m,
        _ => Map::new(),
    };
    ensure!(!comps.contains_key("source"), "the composition already has a nested comp called \"source\"; rename it first");
    let name = "source";

    let cover = (tw / sw).max(th / sh);
    let fit = (tw / sw).min(th / sh);
    let place = |scale: f64, focus: f64| -> Value {
        let half = tw / (2.0 * scale);
        let cx = (focus * sw).clamp(half.min(sw / 2.0), (sw - half).max(sw / 2.0));
        json!({"type": "precomp", "comp": name, "anchor": [cx, sh / 2.0], "pos": [tw / 2.0, th / 2.0], "scale": scale})
    };
    let mut layers = vec![];
    match o.mode {
        Mode::Cover => layers.push(place(cover, o.focus)),
        Mode::Contain => {
            let mut back = place(cover, o.focus);
            back["id"] = json!("backdrop");
            back["layout"] = json!("backdrop");
            back["fx"] = json!([{"effect": "blur", "radius": 36}, {"effect": "brightness", "amount": -0.25}]);
            layers.push(back);
            let mut front = place(fit, 0.5);
            front["id"] = json!("film");
            layers.push(front);
        }
    }

    let mut out = Map::new();
    out.insert("width".into(), json!(o.width));
    out.insert("height".into(), json!(o.height));
    out.insert("dur".into(), json!(dur));
    for k in PROJECT_KEYS {
        if let Some(v) = root.get(*k) {
            out.insert((*k).into(), v.clone());
        }
    }
    let mut all = comps;
    all.insert(name.into(), Value::Object(nested));
    out.insert("comps".into(), Value::Object(all));
    out.insert("layers".into(), Value::Array(layers));
    Ok(Value::Object(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src() -> Value {
        json!({"width": 1920, "height": 1080, "fps": 30, "dur": 4, "layers": [{"type": "solid", "color": "#fff"}], "comps": {"inner": {"width": 10, "height": 10, "dur": 1, "layers": []}}})
    }

    #[test]
    fn cover_fills_a_vertical_frame_and_follows_focus() {
        let out = reframe(&src(), &Options { width: 1080, height: 1920, mode: Mode::Cover, focus: 0.5 }).unwrap();
        let l = &out["layers"][0];
        assert_eq!(out["width"], 1080);
        assert!((l["scale"].as_f64().unwrap() - 1920.0 / 1080.0).abs() < 1e-9, "scaled until the height is filled");
        assert_eq!(l["pos"], json!([540.0, 960.0]));
        assert!(out["comps"]["source"]["comps"].is_null(), "nested comps move up to the root");
        assert!(out["comps"]["inner"].is_object());
        let left = reframe(&src(), &Options { width: 1080, height: 1920, mode: Mode::Cover, focus: 0.0 }).unwrap();
        let cx = left["layers"][0]["anchor"][0].as_f64().unwrap();
        assert!((cx - 1080.0 / (2.0 * 1920.0 / 1080.0)).abs() < 1e-6, "the crop stops at the left edge, got {cx}");
    }

    #[test]
    fn contain_adds_a_blurred_backdrop() {
        let out = reframe(&src(), &Options { width: 1080, height: 1920, mode: Mode::Contain, focus: 0.5 }).unwrap();
        assert_eq!(out["layers"].as_array().unwrap().len(), 2);
        assert!((out["layers"][1]["scale"].as_f64().unwrap() - 1080.0 / 1920.0).abs() < 1e-9);
        assert_eq!(out["layers"][0]["layout"], "backdrop");
    }

    #[test]
    fn rejects_bad_input() {
        assert!(reframe(&src(), &Options { width: 0, height: 10, mode: Mode::Cover, focus: 0.5 }).is_err());
        assert!(reframe(&src(), &Options { width: 10, height: 10, mode: Mode::Cover, focus: 2.0 }).is_err());
        assert!("zoom".parse::<Mode>().is_err());
    }
}
