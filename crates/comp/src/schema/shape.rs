//! Shape layers: geometry plus fill, stroke, trim and repeat, in the order they are listed.

use anyhow::{anyhow, bail, ensure, Result};
use serde::Deserialize;
use serde_json::{Map, Value};

use super::{de::from_value_at, item_alias, palette::{self, Cycling}, path::PathSpec, taper::Taper, wrap::{Col, Pos, Scale, P}};
use crate::shape::stroke::{Cap, Join};

#[derive(Debug, Clone)]
pub struct ShapeItem {
    pub path: PathSpec,
    pub fill: Option<Fill>,
    pub stroke: Option<Stroke>,
    pub trim: Option<Trim>,
    pub offset: Option<Pos>,
    /// The point of the item that `rotation` and `scale` turn about, in layer space (default: the layer's origin).
    pub anchor: Option<Pos>,
    pub rotation: Option<P>,
    pub scale: Option<Scale>,
    pub opacity: Option<P>,
    pub repeat: Option<Repeat>,
    /// The stroke thins toward the ends of the outline.
    pub taper: Option<Taper>,
}

const PATH_KEYS: &[&str] = &["d", "points", "closed", "smooth", "rect", "radius", "ellipse", "polygon", "star", "rotate", "morph_to", "morph"];
const ITEM_KEYS: &[&str] = &["fill", "stroke", "trim", "offset", "anchor", "rotation", "scale", "opacity", "repeat", "taper"];

/// Every key a shape item accepts and which half it belongs to: `outline` keys say where the line goes, `look` keys how it is painted.
pub fn item_keys() -> Vec<(&'static str, &'static str)> {
    PATH_KEYS.iter().map(|k| (*k, "outline")).chain(ITEM_KEYS.iter().map(|k| (*k, "look"))).collect()
}

/// A brushed-metal ramp: highlight, mid, shadow. Used as a fill on big numbers and on metal edges.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Metal {
    pub highlight: Option<Col>,
    pub mid: Option<Col>,
    pub shadow: Option<Col>,
}

impl ShapeItem {
    /// A shape item is its geometry keys (`rect`, `ellipse`, `d`, ...) and its look keys (`fill`, `stroke`, `trim`...) in one object.
    pub fn parse(v: &Value) -> Result<ShapeItem> {
        let o = v.as_object().ok_or_else(|| anyhow!("a shape item is an object like {{\"rect\": [0, 0, 300, 200], \"fill\": \"#fff\"}}"))?;
        let flat: Vec<&str> = item_alias::ALIASES.iter().map(|(flat, ..)| *flat).collect();
        let valid: Vec<&str> = PATH_KEYS.iter().chain(ITEM_KEYS).chain(&flat).copied().collect();
        if let Some(message) = super::fields::stray_keys("shape item", o.keys(), &valid) {
            bail!("{message}");
        }
        let o = &item_alias::nest(o)?;
        let path_map: Map<String, Value> = o.iter().filter(|(k, _)| PATH_KEYS.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect();
        let path: PathSpec = serde_json::from_value(Value::Object(path_map)).map_err(|e| anyhow!("{e}"))?;
        ensure!(path.geometry_count() == 1, "a shape item needs exactly one of d, points, rect, ellipse, polygon, star (found {})", path.geometry_count());
        let get = |k: &str| o.get(k);
        let item = ShapeItem {
            path,
            fill: get("fill").map(Fill::parse).transpose().map_err(|e| anyhow!("fill: {e:#}"))?,
            stroke: get("stroke").map(|s| serde_json::from_value::<Stroke>(s.clone())).transpose().map_err(|e| anyhow!("stroke: {e}"))?,
            trim: get("trim").map(|s| serde_json::from_value::<Trim>(s.clone())).transpose().map_err(|e| anyhow!("trim: {e}"))?,
            offset: get("offset").map(|s| serde_json::from_value::<Pos>(s.clone())).transpose().map_err(|e| anyhow!("offset: {e}"))?,
            anchor: get("anchor").map(|s| serde_json::from_value::<Pos>(s.clone())).transpose().map_err(|e| anyhow!("anchor: {e}"))?,
            rotation: get("rotation").map(|s| serde_json::from_value::<P>(s.clone())).transpose().map_err(|e| anyhow!("rotation: {e}"))?,
            scale: get("scale").map(|s| serde_json::from_value::<Scale>(s.clone())).transpose().map_err(|e| anyhow!("scale: {e}"))?,
            opacity: get("opacity").map(|s| serde_json::from_value::<P>(s.clone())).transpose().map_err(|e| anyhow!("opacity: {e}"))?,
            repeat: get("repeat").map(Repeat::parse).transpose().map_err(|e| anyhow!("repeat: {e:#}"))?,
            taper: get("taper").map(|s| from_value_at::<Taper>(s.clone())).transpose().map_err(|e| anyhow!("taper: {e:#}"))?,
        };
        item.check()?;
        Ok(item)
    }

    /// The rules between an item's keys, which no single key can check alone.
    fn check(&self) -> Result<()> {
        if let Some(t) = &self.taper {
            t.validate().map_err(|e| anyhow!("taper: {e:#}"))?;
            let stroke = self.stroke.as_ref().ok_or_else(|| anyhow!("taper thins a stroke, and this item has none: add a `stroke`, or leave the taper out (a fill has no width to taper)"))?;
            ensure!(stroke.dash.is_none(), "a tapered stroke cannot also be dashed: remove `dash` or `taper`");
            Cap::parse(stroke.cap.as_deref().unwrap_or("butt")).map_err(|e| anyhow!("stroke: {e}"))?;
            Join::parse(stroke.join.as_deref().unwrap_or("miter")).map_err(|e| anyhow!("stroke: {e}"))?;
        }
        if let Some(cycling) = self.repeat.as_ref().and_then(|r| r.palette.as_ref()) {
            cycling.check_item(self).map_err(|e| anyhow!("repeat: {e:#}"))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Fill {
    pub color: Option<Col>,
    pub gradient: Option<Gradient>,
    pub metal: Option<Metal>,
    pub opacity: Option<P>,
    pub even_odd: bool,
}

impl Fill {
    /// A bare colour, `"metal"`, or `{"color"|"gradient"|"metal", "opacity", "rule"}`.
    pub fn parse(v: &Value) -> Result<Fill> {
        if v.as_str() == Some("metal") {
            return Ok(Fill { color: None, gradient: None, metal: Some(Metal::default()), opacity: None, even_odd: false });
        }
        let is_object_form = v.as_object().is_some_and(|o| o.contains_key("color") || o.contains_key("gradient") || o.contains_key("metal") || o.contains_key("opacity") || o.contains_key("rule"));
        if !is_object_form {
            return Ok(Fill { color: Some(serde_json::from_value(v.clone()).map_err(|e| anyhow!("{e}"))?), gradient: None, metal: None, opacity: None, even_odd: false });
        }
        let o = v.as_object().expect("checked");
        if let Some(bad) = o.keys().find(|k| !["color", "gradient", "metal", "opacity", "rule"].contains(&k.as_str())) {
            bail!("unknown fill field {bad:?}; valid: color, gradient, metal, opacity, rule");
        }
        let even_odd = match o.get("rule").and_then(Value::as_str) {
            None | Some("nonzero") => false,
            Some("evenodd") => true,
            Some(other) => bail!("fill rule must be nonzero or evenodd, not {other:?}"),
        };
        let metal = match o.get("metal") {
            None => None,
            Some(Value::Bool(true)) => Some(Metal::default()),
            Some(v) => Some(serde_json::from_value(v.clone()).map_err(|e| anyhow!("metal: {e}"))?),
        };
        Ok(Fill {
            color: o.get("color").map(|c| serde_json::from_value(c.clone())).transpose().map_err(|e| anyhow!("{e}"))?,
            gradient: o.get("gradient").map(|c| serde_json::from_value(c.clone())).transpose().map_err(|e| anyhow!("gradient: {e}"))?,
            metal,
            opacity: o.get("opacity").map(|c| serde_json::from_value(c.clone())).transpose().map_err(|e| anyhow!("{e}"))?,
            even_odd,
        })
    }
}

impl<'de> Deserialize<'de> for Fill {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        Fill::parse(&v).map_err(serde::de::Error::custom)
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

/// Copies of an item, each moved on from the last; with a palette each copy also takes the next colour (see
/// `schema::palette`).
#[derive(Debug, Clone)]
pub struct Repeat {
    pub copies: u32,
    /// Added to every copy's position, cumulatively.
    pub offset: Option<[f64; 2]>,
    /// Degrees added per copy.
    pub rotation: f64,
    /// Multiplied per copy.
    pub scale: f64,
    /// Opacity multiplied per copy.
    pub opacity: f64,
    /// The colours the copies take, one each.
    pub palette: Option<Cycling>,
}

/// The keys of a repeat that are not the palette's.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RepeatKeys {
    copies: u32,
    #[serde(default)]
    offset: Option<[f64; 2]>,
    #[serde(default)]
    rotation: f64,
    #[serde(default = "one")]
    scale: f64,
    #[serde(default = "one")]
    opacity: f64,
}

impl Repeat {
    pub fn parse(v: &Value) -> Result<Repeat> {
        let mut rest = v.as_object().cloned().ok_or_else(|| anyhow!("a repeat is an object like {{\"copies\": 5, \"offset\": [40, 0]}}"))?;
        let valid: Vec<&str> = ["copies", "offset", "rotation", "scale", "opacity"].into_iter().chain(palette::KEYS).collect();
        if let Some(message) = super::fields::stray_keys("repeat", rest.keys(), &valid) {
            bail!("{message}");
        }
        let palette = Cycling::take(&mut rest)?;
        let RepeatKeys { copies, offset, rotation, scale, opacity } = from_value_at(Value::Object(rest))?;
        Ok(Repeat { copies, offset, rotation, scale, opacity, palette })
    }
}

fn one() -> f64 {
    1.0
}

#[cfg(test)]
#[path = "shape_tests.rs"]
mod tests;
