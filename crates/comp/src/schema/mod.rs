//! The composition file as data: strict, typed, and free of meaning. Nothing here loads a file, resolves a name or
//! touches a GPU; that is `plan`'s job. A mistake is reported with the field's name and the valid alternatives.

pub mod bake;
pub mod blocks;
pub mod extrude;
mod kinds;
pub mod letterbox;
mod path;
mod shape;
mod text;
pub mod time;
pub mod timeline;
pub mod tokens;
mod wrap;

use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use serde_json::{Map, Value};
use turbomotion_core::effects::{self, Blend, Effect};

pub use kinds::*;
pub use path::PathSpec;
pub use shape::{Fill, Gradient, Metal, Repeat, ShapeItem, Stroke, Trim};
pub use text::TextRun;
pub use text::{Align, Animator, Charset, Cipher, Counter, Marquee, PathSource, Perspective, SelectorShape, SurfaceKind, TextPath, TextSurface, Direction, Mark, Role, Selector, Stagger, StaggerFrom, TextSource, TextStroke, TextSwap, Unit, UnitProps, Verdict};
pub use tokens::TypeScale;
pub use wrap::{Col, Orient, Pos, Rot, Scale, P};

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
    /// Font roles: `{"display": "fonts/Anton.ttf"}`, then `"font": "display"` on text layers. Type roles may also set a font.
    pub fonts: BTreeMap<String, String>,
    /// Named colours any kind can use (`"color": "accent"` or `"$accent"`). Not a look; looks are per kind.
    #[serde(default)]
    pub palette: Map<String, Value>,
    /// Text styles per role. A number is still just a size; an object may add font, weight, colour, tracking.
    /// A text layer's `"role": "display"|"title"|"body"` (or `"size": "display"`) picks one.
    #[serde(rename = "type", default)]
    pub type_scale: Option<TypeScale>,
    /// Default treatment for image layers that do not set `fx`.
    #[serde(default)]
    pub image: Option<Value>,
    /// Named plate recipes for shape layers (`"surface": "card"`).
    #[serde(default)]
    pub surface: Map<String, Value>,
    /// Pad, gap, radius. Published as `$pad` / `$gap` / `$radius`; a shape may say `"radius": "radius"`.
    #[serde(default)]
    pub space: Map<String, Value>,
    /// Enter / exit / snap: `{ "enter": { "dur": 0.4, "ease": "decel" } }`. Fills a layer enter/exit that omitted `dur`.
    #[serde(default)]
    pub motion: Option<Value>,
    /// Motion blur settings (samples per frame, shutter angle). Layers with `"mblur": true` blur along their motion; the defaults are 8 samples at 180 degrees.
    pub mblur: Option<MotionBlur>,
    /// Finishing on the finished picture (grain, vignette). Not type, not images; those have their own looks.
    pub look: Vec<Value>,
    /// Check this composition's foreground layers for clashes (`check --layout`): text, cards, shapes and packs that overlap on screen.
    pub layout: bool,
    /// Layers, bottom first.
    pub layers: Vec<Value>,
    /// Root only. The folder every relative path (`src`, fonts, `fx_path`, blocks) starts from, relative to this
    /// file. A film built by the SDK into another folder sets it to the script's folder.
    pub root: Option<String>,
    /// Root only. Extra folders of shader plugins (`*.sksl`), relative to the root folder; see `comp fx list`.
    pub fx_path: Vec<String>,
    /// Root only. Extra folders of blocks (`<job>/<name>.json`), relative to the root folder; see `comp blocks`.
    pub block_path: Vec<String>,
    /// Root only. Black bars (or a safe-area matte) drawn over the shots and their transitions: `[w, h]` of the
    /// picture that stays visible, centred. Overlays and film-level layers sit above the bars.
    pub letterbox: Option<[f64; 2]>,
}

impl Default for Composition {
    fn default() -> Self {
        Composition { width: 1920, height: 1080, fps: 30, dur: 5.0, bg: None, bpm: None, beat_offset: 0.0, markers: vec![], comps: BTreeMap::new(), vars: Map::new(), fonts: BTreeMap::new(), palette: Map::new(), type_scale: None, image: None, surface: Map::new(), space: Map::new(), motion: None, mblur: None, look: vec![], layout: false, layers: vec![], root: None, fx_path: vec![], block_path: vec![], letterbox: None }
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

/// An effect on a layer: what it is, and how its result is laid over its input (see `fxlib::composite`).
#[derive(Debug, Clone)]
pub struct Fx {
    pub op: FxOp,
    /// The effect's `"composite"` options as written; parsed when the plan is made, where animated values and layer ids resolve.
    pub composite: Option<Value>,
}

/// One from the platform library, or a shader plugin from `effects/`.
#[derive(Debug, Clone)]
pub enum FxOp {
    Builtin(Box<Effect>),
    Plugin { name: String, params: Map<String, Value> },
}

/// The key that carries the composite options on any effect.
pub const COMPOSITE_KEY: &str = "composite";

impl Fx {
    pub fn parse(v: &Value) -> Result<Fx> {
        let o = v.as_object().ok_or_else(|| anyhow!("an effect is an object like {{\"effect\": \"blur\", \"radius\": 8}}"))?;
        let name = o.get("effect").and_then(Value::as_str).ok_or_else(|| anyhow!("an effect needs \"effect\": \"<name>\""))?;
        let mut own = o.clone();
        let composite = own.remove(COMPOSITE_KEY);
        own.remove("effect");
        let op = if effects::entry(name).is_some() {
            bake::bake(&mut own).map_err(|e| anyhow!("effect {name:?}: {e:#}"))?;
            FxOp::Builtin(Box::new(Effect::build(name, &own)?))
        } else {
            FxOp::Plugin { name: name.to_string(), params: own }
        };
        Ok(Fx { op, composite })
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
    /// `window` only: where the portal sits when it is small, as `[centre x, centre y, width]`, all fractions of the
    /// frame (the portal keeps the frame's aspect ratio). Default `[0.5, 0.5, 0.34]`.
    pub portal: Option<[f64; 3]>,
    /// `window` only: corner radius in pixels of the portal when it is small; it squares off as the portal opens.
    pub radius: Option<f64>,
}

impl Default for Transit {
    fn default() -> Self {
        Transit { preset: "fade".into(), dur: 0.5, delay: 0.0, ease: None, dist: None, blur: None, portal: None, radius: None }
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

/// How a lit 3D layer answers to light: strengths of the ambient and diffuse response, the specular highlight and its tightness.
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(default, deny_unknown_fields)]
pub struct Material {
    pub ambient: f64,
    pub diffuse: f64,
    pub specular: f64,
    pub shininess: f64,
}

/// Which side of a 3D layer is hidden. After Effects draws both sides of a layer; a plane seen from behind shows
/// its content mirrored.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Cull {
    /// Draw both sides.
    #[default]
    None,
    /// Hide the side the layer is read from, so it shows only from behind.
    Front,
    /// Hide the far side, so the layer shows only from the front.
    Back,
}

/// A faded, softened mirror of the layer, drawn on the floor beneath it.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Reflect {
    /// Opacity of the mirror at the floor line. Default 0.4.
    #[serde(default)]
    pub strength: Option<P>,
    /// How much of the layer's height is mirrored, 0..1. Default 0.55.
    #[serde(default)]
    pub fade: Option<f64>,
    /// Gap between the layer and the mirror, px. Default 8.
    #[serde(default)]
    pub gap: Option<f64>,
    /// Softness of the mirror, px. Default 3.
    #[serde(default)]
    pub blur: Option<f64>,
}

impl Default for Material {
    fn default() -> Self {
        Material { ambient: 1.0, diffuse: 1.0, specular: 0.0, shininess: 32.0 }
    }
}

/// Wrap a coordinate into `[min, max)` after it is evaluated.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Wrap {
    pub x: Option<[f64; 2]>,
    pub y: Option<[f64; 2]>,
}

/// GSAP stagger applied to a parent's children: they start `each` seconds apart.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct LayerStagger {
    #[serde(default = "stagger_each")]
    pub each: f64,
    #[serde(default)]
    pub from: StaggerFrom,
    #[serde(default = "one_u32")]
    pub seed: u32,
}

fn stagger_each() -> f64 {
    0.1
}

fn one_u32() -> u32 {
    1
}

/// GSAP Flip: this layer flies from `from`'s transform onto its own, starting at `at`.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Flip {
    pub from: String,
    #[serde(default)]
    pub at: f64,
    #[serde(default = "flip_dur")]
    pub dur: f64,
    #[serde(default)]
    pub ease: Option<String>,
}

fn flip_dur() -> f64 {
    0.8
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
    /// A 3D layer is lit by the composition's lights (default: yes when there are any). `false` keeps it flat.
    pub lit: Option<bool>,
    /// How a lit layer answers to light.
    pub material: Option<Material>,
    pub pos: Option<Pos>,
    pub anchor: Option<Pos>,
    pub scale: Option<Scale>,
    /// Degrees, clockwise: the layer's route through space (After Effects' X, Y and Z Rotation). One number
    /// spins the layer about the z axis; three numbers turn it about each axis in turn, inside its orientation.
    pub rotation: Option<Rot>,
    /// The layer's own orientation in degrees (After Effects' Orientation): `[x, y, z]`.
    /// `x` tips the layer's face down, `y` turns its face to the left, `z` spins it clockwise in its own plane.
    /// After Effects stores it as a quaternion: keys interpolate along the shortest turn (see `eval::pose`).
    pub orientation: Option<Orient>,
    /// The point the layer's face turns toward, in composition (world) space, as After Effects' auto-orient toward
    /// the point of interest does, even when the layer has a parent.
    /// The face is the side the layer is read from; the layer's own `orientation` is applied on top of it.
    pub aim: Option<Pos>,
    /// Which side of a 3D layer is hidden. Both sides by default.
    pub cull: Option<Cull>,
    /// Shear in degrees (GSAP `skewX`). Positive leans the top to the right.
    pub skew: Option<P>,
    pub opacity: Option<P>,
    /// After evaluating position, wrap `x` / `y` into a range (GSAP `modifiers.wrap`). A ticker: `"pos": ["time * -80", 900], "wrap": {"x": [-1920, 1920]}`.
    pub wrap: Option<Wrap>,
    /// Direct children start this many seconds apart (GSAP stagger on a group).
    pub stagger: Option<LayerStagger>,
    /// This layer interpolates from another layer's transform onto its own (GSAP Flip).
    pub flip: Option<Flip>,
    pub blend: Option<Blend>,
    pub fx: Vec<Value>,
    pub masks: Vec<Mask>,
    pub matte: Option<Matte>,
    /// Keep this layer inside another layer's alpha, and still draw that layer (unlike a track matte).
    pub clip_to: Option<String>,
    pub mblur: bool,
    /// How `check --layout` treats this layer: `foreground` (must not overlap other foreground layers), `backdrop` or `ignore`.
    /// Left out, text, shapes, nested comps and packs are foreground.
    pub layout: Option<String>,
    pub along: Option<Along>,
    pub enter: Option<Transit>,
    pub exit: Option<Transit>,
    /// Draw a faded mirror of this layer on the floor beneath it.
    pub reflect: Option<Reflect>,
}

#[derive(Debug, Clone)]
pub struct Layer {
    pub common: Common,
    pub fx: Vec<Fx>,
    pub kind: Kind,
}

const COMMON_KEYS: &[&str] = &[
    "id", "start", "dur", "parent", "hidden", "three_d", "lit", "material", "pos", "anchor", "scale", "rotation", "orientation", "aim", "cull", "skew", "opacity", "wrap", "stagger", "flip", "blend", "fx", "masks", "matte", "clip_to", "mblur", "layout", "along", "enter", "exit", "reflect",
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
        "mesh" => Kind::Mesh(serde_json::from_value(kv).map_err(kind_err)?),
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
