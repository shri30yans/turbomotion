//! Shape layers: geometry plus fill, stroke, trim and repeat, in the order they are listed.

use anyhow::{anyhow, bail, ensure, Result};
use serde::Deserialize;
use serde_json::{Map, Value};

use super::{path::PathSpec, wrap::{Col, Pos, Scale, P}};

#[derive(Debug, Clone)]
pub struct ShapeItem {
    pub path: PathSpec,
    pub fill: Option<Fill>,
    pub stroke: Option<Stroke>,
    pub trim: Option<Trim>,
    pub offset: Option<Pos>,
    pub rotation: Option<P>,
    pub scale: Option<Scale>,
    pub opacity: Option<P>,
    pub repeat: Option<Repeat>,
}

const PATH_KEYS: &[&str] = &["d", "points", "closed", "smooth", "rect", "radius", "ellipse", "polygon", "star", "rotate", "morph_to", "morph"];
const ITEM_KEYS: &[&str] = &["fill", "stroke", "trim", "offset", "rotation", "scale", "opacity", "repeat"];

impl ShapeItem {
    /// A shape item is its geometry keys (`rect`, `ellipse`, `d`, ...) and its look keys (`fill`, `stroke`, `trim`...) in one object.
    pub fn parse(v: &Value) -> Result<ShapeItem> {
        let o = v.as_object().ok_or_else(|| anyhow!("a shape item is an object like {{\"rect\": [0, 0, 300, 200], \"fill\": \"#fff\"}}"))?;
        if let Some(bad) = o.keys().find(|k| !PATH_KEYS.contains(&k.as_str()) && !ITEM_KEYS.contains(&k.as_str())) {
            bail!("unknown shape field {bad:?}; geometry: {}; look: {}", PATH_KEYS.join(", "), ITEM_KEYS.join(", "));
        }
        let path_map: Map<String, Value> = o.iter().filter(|(k, _)| PATH_KEYS.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect();
        let path: PathSpec = serde_json::from_value(Value::Object(path_map)).map_err(|e| anyhow!("{e}"))?;
        ensure!(path.geometry_count() == 1, "a shape item needs exactly one of d, points, rect, ellipse, polygon, star (found {})", path.geometry_count());
        let get = |k: &str| o.get(k);
        let de = |k: &str| -> Result<_> { Ok(get(k).cloned()) };
        let _ = de;
        Ok(ShapeItem {
            path,
            fill: get("fill").map(Fill::parse).transpose().map_err(|e| anyhow!("fill: {e:#}"))?,
            stroke: get("stroke").map(|s| serde_json::from_value::<Stroke>(s.clone())).transpose().map_err(|e| anyhow!("stroke: {e}"))?,
            trim: get("trim").map(|s| serde_json::from_value::<Trim>(s.clone())).transpose().map_err(|e| anyhow!("trim: {e}"))?,
            offset: get("offset").map(|s| serde_json::from_value::<Pos>(s.clone())).transpose().map_err(|e| anyhow!("offset: {e}"))?,
            rotation: get("rotation").map(|s| serde_json::from_value::<P>(s.clone())).transpose().map_err(|e| anyhow!("rotation: {e}"))?,
            scale: get("scale").map(|s| serde_json::from_value::<Scale>(s.clone())).transpose().map_err(|e| anyhow!("scale: {e}"))?,
            opacity: get("opacity").map(|s| serde_json::from_value::<P>(s.clone())).transpose().map_err(|e| anyhow!("opacity: {e}"))?,
            repeat: get("repeat").map(|s| serde_json::from_value::<Repeat>(s.clone())).transpose().map_err(|e| anyhow!("repeat: {e}"))?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Fill {
    pub color: Option<Col>,
    pub gradient: Option<Gradient>,
    pub opacity: Option<P>,
    pub even_odd: bool,
}

impl Fill {
    /// A bare colour, or `{"color": .., "gradient": .., "opacity": .., "rule": "evenodd"}`.
    pub fn parse(v: &Value) -> Result<Fill> {
        let is_object_form = v.as_object().is_some_and(|o| o.contains_key("color") || o.contains_key("gradient") || o.contains_key("opacity") || o.contains_key("rule"));
        if !is_object_form {
            return Ok(Fill { color: Some(serde_json::from_value(v.clone()).map_err(|e| anyhow!("{e}"))?), gradient: None, opacity: None, even_odd: false });
        }
        let o = v.as_object().expect("checked");
        if let Some(bad) = o.keys().find(|k| !["color", "gradient", "opacity", "rule"].contains(&k.as_str())) {
            bail!("unknown fill field {bad:?}; valid: color, gradient, opacity, rule");
        }
        let even_odd = match o.get("rule").and_then(Value::as_str) {
            None | Some("nonzero") => false,
            Some("evenodd") => true,
            Some(other) => bail!("fill rule must be nonzero or evenodd, not {other:?}"),
        };
        Ok(Fill {
            color: o.get("color").map(|c| serde_json::from_value(c.clone())).transpose().map_err(|e| anyhow!("{e}"))?,
            gradient: o.get("gradient").map(|c| serde_json::from_value(c.clone())).transpose().map_err(|e| anyhow!("gradient: {e}"))?,
            opacity: o.get("opacity").map(|c| serde_json::from_value(c.clone())).transpose().map_err(|e| anyhow!("{e}"))?,
            even_odd,
        })
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Gradient {
    /// `linear` or `radial`.
    #[serde(rename = "type", default = "linear")]
    pub kind: String,
    /// `[[0, "#ff8800"], [1, "#2200ff"]]`.
    pub stops: Vec<(f64, Col)>,
    /// Linear: start point.
    #[serde(default)]
    pub from: Option<Pos>,
    /// Linear: end point.
    #[serde(default)]
    pub to: Option<Pos>,
    /// Radial: centre.
    #[serde(default)]
    pub center: Option<Pos>,
    /// Radial: radius.
    #[serde(default)]
    pub radius: Option<P>,
}

fn linear() -> String {
    "linear".into()
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Stroke {
    pub color: Col,
    pub width: P,
    #[serde(default)]
    pub cap: Option<String>,
    #[serde(default)]
    pub join: Option<String>,
    /// `[on, off, ...]` in pixels.
    #[serde(default)]
    pub dash: Option<Vec<f64>>,
    #[serde(default)]
    pub dash_offset: Option<P>,
    #[serde(default)]
    pub opacity: Option<P>,
}

/// Draw-on / draw-off: show only the part of the outline between `start` and `end` (0..1), shifted by `offset`.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Trim {
    #[serde(default)]
    pub start: Option<P>,
    #[serde(default)]
    pub end: Option<P>,
    #[serde(default)]
    pub offset: Option<P>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Repeat {
    pub copies: u32,
    /// Added to every copy's position, cumulatively.
    #[serde(default)]
    pub offset: Option<[f64; 2]>,
    /// Degrees added per copy.
    #[serde(default)]
    pub rotation: f64,
    /// Multiplied per copy.
    #[serde(default = "one")]
    pub scale: f64,
    /// Opacity multiplied per copy.
    #[serde(default = "one")]
    pub opacity: f64,
}

fn one() -> f64 {
    1.0
}
