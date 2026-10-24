//! The composition file as data: strict, typed, and free of meaning. Nothing here loads a file, resolves a name or
//! touches a GPU; that is `plan`'s job. A mistake is reported with the field's name and the valid alternatives.

mod kinds;
mod path;
mod shape;
mod text;
pub mod time;
mod wrap;

use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use serde_json::{Map, Value};
use turbomotion_core::effects::{self, Blend, Effect};

pub use kinds::*;
pub use path::PathSpec;
pub use shape::{Fill, Gradient, Repeat, ShapeItem, Stroke, Trim};
pub use text::{Align, Animator, Counter, Direction, Selector, Stagger, StaggerFrom, TextSource, TextStroke, Unit, UnitProps};
pub use wrap::{Col, Pos, Scale, P};

/// A nested composition or the root. Only the root uses `fps`, `comps`, `fonts` and `audio`.
#[derive(Debug, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Composition {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    /// Length in seconds.
    pub dur: f64,
    /// Background colour; transparent when left out.
    pub bg: Option<Col>,
    /// Beats per minute: enables `beat:N` / `bar:N` times and the `beat()` expression.
    pub bpm: Option<f64>,
    /// Seconds from the start to the first beat.
    pub beat_offset: f64,
    /// Named moments: `[{"t": 4.2, "name": "drop"}]`, usable as `"marker:drop"`.
    pub markers: Vec<Marker>,
    /// Nested compositions that `precomp` layers play.
    pub comps: BTreeMap<String, Composition>,
    /// Variables replacing `"$name"` strings anywhere in the file.
    pub vars: Map<String, Value>,
    /// Font roles: `{"display": "fonts/Anton.ttf"}`, then `"font": "display"` on text layers.
    pub fonts: BTreeMap<String, String>,
    /// Frames of motion blur per frame: 0 = off. Layers with `"mblur": true` blur along their motion.
    pub mblur: Option<MotionBlur>,
    /// Layers, bottom first.
    pub layers: Vec<Value>,
}

impl Default for Composition {
    fn default() -> Self {
        Composition { width: 1920, height: 1080, fps: 30, dur: 5.0, bg: None, bpm: None, beat_offset: 0.0, markers: vec![], comps: BTreeMap::new(), vars: Map::new(), fonts: BTreeMap::new(), mblur: None, layers: vec![] }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    pub t: f64,
    pub name: String,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(default, deny_unknown_fields)]
pub struct MotionBlur {
    /// Sub-frames averaged per frame.
    pub samples: u32,
    /// Shutter angle in degrees (180 = half the frame interval).
    pub shutter: f64,
}

impl Default for MotionBlur {
    fn default() -> Self {
        MotionBlur { samples: 8, shutter: 180.0 }
    }
}

/// An effect on a layer: one from the platform library, or a shader plugin from `effects/`.
#[derive(Debug, Clone)]
pub enum Fx {
    Builtin(Box<Effect>),
    Plugin { name: String, params: Map<String, Value> },
}

impl Fx {
    pub fn parse(v: &Value) -> Result<Fx> {
        let o = v.as_object().ok_or_else(|| anyhow!("an effect is an object like {{\"effect\": \"blur\", \"radius\": 8}}"))?;
        let name = o.get("effect").and_then(Value::as_str).ok_or_else(|| anyhow!("an effect needs \"effect\": \"<name>\""))?;
        if effects::catalogue().iter().any(|e| e.effect == name) {
            Ok(Fx::Builtin(Box::new(serde_json::from_value(v.clone()).map_err(|e| anyhow!("effect {name:?}: {e}"))?)))
        } else {
            let mut params = o.clone();
            params.remove("effect");
            Ok(Fx::Plugin { name: name.to_string(), params })
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MaskMode {
    #[default]
    Add,
    Subtract,
    Intersect,
    Difference,
}

/// A vector mask on a layer: the layer shows only where the mask covers.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Mask {
    pub path: PathSpec,
    #[serde(default)]
    pub mode: MaskMode,
    /// Edge softness in pixels.
    #[serde(default)]
    pub feather: Option<P>,
    /// Grow (positive) or shrink (negative) the mask by this many pixels.
    #[serde(default)]
    pub expansion: Option<P>,
    #[serde(default)]
    pub opacity: Option<P>,
    #[serde(default)]
    pub invert: bool,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MatteMode {
    /// The source's alpha decides.
    #[default]
    Alpha,
    AlphaInverted,
    /// The source's brightness decides.
    Luma,
    LumaInverted,
}

/// A track matte: another layer's alpha or brightness decides where this layer shows.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Matte {
    pub source: String,
    #[serde(default)]
    pub mode: MatteMode,
}

/// A named entrance or exit for the layer as a whole: `{"preset": "slide_up", "dur": 0.5}`.
#[derive(Debug, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Transit {
    pub preset: String,
    pub dur: f64,
    pub delay: f64,
    pub ease: Option<String>,
    /// Travel distance in pixels for slides and whips.
    pub dist: Option<f64>,
    /// Blur in pixels at the hidden end.
    pub blur: Option<f64>,
}

impl Default for Transit {
    fn default() -> Self {
        Transit { preset: "fade".into(), dur: 0.5, delay: 0.0, ease: None, dist: None, blur: None }
    }
}

/// Motion along a path (GSAP MotionPath): the layer's position follows the outline as `t` goes 0 → 1.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Along {
    pub path: PathSpec,
    pub t: P,
    /// Rotate the layer to face the direction of travel.
    #[serde(default)]
    pub orient: bool,
}

/// Fields every layer has.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Common {
    pub id: Option<String>,
    /// Seconds into the composition when the layer begins. Every property's time is relative to this.
    pub start: f64,
    /// How long the layer lasts. Default: to the end of the composition (or of the media).
    pub dur: Option<f64>,
    pub parent: Option<String>,
    /// Do not draw this layer (it can still be a parent, a matte or a camera target).
    pub hidden: bool,
    pub three_d: bool,
    pub pos: Option<Pos>,
    pub anchor: Option<Pos>,
    pub scale: Option<Scale>,
    /// Degrees, clockwise.
    pub rotation: Option<P>,
    pub rot_x: Option<P>,
    pub rot_y: Option<P>,
    pub opacity: Option<P>,
    pub blend: Option<Blend>,
    pub fx: Vec<Value>,
    pub masks: Vec<Mask>,
    pub matte: Option<Matte>,
    pub mblur: bool,
    pub along: Option<Along>,
    pub enter: Option<Transit>,
    pub exit: Option<Transit>,
}

#[derive(Debug, Clone)]
pub struct Layer {
    pub common: Common,
    pub fx: Vec<Fx>,
    pub kind: Kind,
}

const COMMON_KEYS: &[&str] = &[
    "id", "start", "dur", "parent", "hidden", "three_d", "pos", "anchor", "scale", "rotation", "rot_x", "rot_y", "opacity", "blend", "fx", "masks", "matte", "mblur", "along", "enter", "exit",
];

/// Parse layer number `index` of a composition.
pub fn parse_layer(index: usize, v: &Value) -> Result<Layer> {
    let o = v.as_object().ok_or_else(|| anyhow!("a layer is an object like {{\"type\": \"text\", ...}}"))?;
    let ty = o.get("type").and_then(Value::as_str).ok_or_else(|| anyhow!("every layer needs \"type\": one of {}", Kind::NAMES.join(", ")))?;
    let label = || format!("layer {index} ({ty}{})", o.get("id").and_then(Value::as_str).map_or(String::new(), |i| format!(" {i:?}")));
    let (mut common_map, mut kind_map) = (Map::new(), Map::new());
    for (k, val) in o {
        if k == "type" {
            continue;
        }
        if COMMON_KEYS.contains(&k.as_str()) { common_map.insert(k.clone(), val.clone()) } else { kind_map.insert(k.clone(), val.clone()) };
    }
    let common: Common = serde_json::from_value(Value::Object(common_map)).map_err(|e| anyhow!("{}: {e}", label()))?;
    let fx = common.fx.iter().enumerate().map(|(i, f)| Fx::parse(f).with_context(|| format!("fx {i}"))).collect::<Result<Vec<_>>>().with_context(label)?;
    let kv = Value::Object(kind_map);
    let kind_err = |e: serde_json::Error| anyhow!("{}: {e}", label());
    let kind = match ty {
        "solid" => Kind::Solid(serde_json::from_value(kv).map_err(kind_err)?),
        "shape" => {
            let items = kv.get("items").and_then(Value::as_array).ok_or_else(|| anyhow!("{}: a shape layer needs \"items\": [{{\"rect\": [..], \"fill\": ..}}]", label()))?;
            if let Some(extra) = kv.as_object().and_then(|m| m.keys().find(|k| *k != "items")) {
                bail!("{}: unknown field {extra:?}; a shape layer has only \"items\" besides the common fields", label());
            }
            Kind::Shape(Shape { items: items.iter().enumerate().map(|(i, it)| ShapeItem::parse(it).with_context(|| format!("item {i}"))).collect::<Result<_>>().with_context(label)? })
        }
        "text" => Kind::Text(serde_json::from_value(kv).map_err(kind_err)?),
        "image" => Kind::Image(serde_json::from_value(kv).map_err(kind_err)?),
        "video" => Kind::Video(serde_json::from_value(kv).map_err(kind_err)?),
        "audio" => Kind::Audio(serde_json::from_value(kv).map_err(kind_err)?),
        "precomp" => Kind::Precomp(serde_json::from_value(kv).map_err(kind_err)?),
        "null" => no_fields(&kv, ty, &label).map(|()| Kind::Null)?,
        "adjustment" => no_fields(&kv, ty, &label).map(|()| Kind::Adjustment)?,
        "camera" => Kind::Camera(serde_json::from_value(kv).map_err(kind_err)?),
        "light" => Kind::Light(serde_json::from_value(kv).map_err(kind_err)?),
        "particles" => Kind::Particles(Box::new(serde_json::from_value(kv).map_err(kind_err)?)),
        "texture" => Kind::Texture(serde_json::from_value(kv).map_err(kind_err)?),
        "pack" => Kind::Pack(serde_json::from_value(kv).map_err(kind_err)?),
        other => bail!("{}: unknown layer type {other:?}; the types are {}", label(), Kind::NAMES.join(", ")),
    };
    Ok(Layer { common, fx, kind })
}

fn no_fields(kv: &Value, ty: &str, label: &dyn Fn() -> String) -> Result<()> {
    match kv.as_object().and_then(|m| m.keys().next()) {
        Some(extra) => bail!("{}: unknown field {extra:?}; a {ty} layer has only the common fields", label()),
        None => Ok(()),
    }
}
