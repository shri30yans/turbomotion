//! What each kind of layer carries beyond the fields every layer has.

use serde::Deserialize;
use serde_json::{Map, Value};

use super::{
    shape::ShapeItem,
    text::{Align, Animator, TextSource, TextStroke},
    wrap::{Col, Pos, P},
};

#[derive(Debug, Clone)]
pub enum Kind {
    Solid(Solid),
    Shape(Shape),
    Text(Text),
    Image(Image),
    Video(Video),
    Audio(Audio),
    Precomp(Precomp),
    Null,
    Camera(Camera),
    Light(Light),
    Adjustment,
    Particles(Box<Particles>),
    Texture(Texture),
    Pack(Pack),
}

impl Kind {
    pub const NAMES: &'static [&'static str] = &["solid", "shape", "text", "image", "video", "audio", "precomp", "null", "camera", "light", "adjustment", "particles", "texture", "pack"];
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Solid {
    pub color: Col,
    /// Size in pixels; the composition's size when left out.
    #[serde(default)]
    pub size: Option<[f64; 2]>,
}

#[derive(Debug, Clone)]
pub struct Shape {
    pub items: Vec<ShapeItem>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Text {
    pub text: TextSource,
    /// A family name (`"Helvetica Neue"`) or a path to a `.ttf`/`.otf`/`.ttc` file (`"fonts/Anton.ttf"`, `"x.ttc#2"`).
    #[serde(default)]
    pub font: Option<String>,
    #[serde(default)]
    pub weight: Option<u16>,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub size: Option<P>,
    #[serde(default)]
    pub color: Option<Col>,
    /// Pixels added after every character.
    #[serde(default)]
    pub tracking: Option<P>,
    /// Line height as a multiple of the font size.
    #[serde(default)]
    pub leading: Option<f64>,
    #[serde(default)]
    pub align: Align,
    /// Wrap width in pixels; the text is one line per `\n` when left out.
    #[serde(default)]
    pub width: Option<f64>,
    #[serde(default)]
    pub stroke: Option<TextStroke>,
    #[serde(default)]
    pub animators: Vec<Animator>,
    /// A named animation from the catalogue, expanded into animators: `"rise"` or `{"name": "rise", "by": "words"}`.
    #[serde(default)]
    pub preset: Option<Value>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    /// Fill the composition, cropping.
    #[default]
    Cover,
    /// Fit inside the composition.
    Contain,
    /// Stretch.
    Fill,
    /// Natural size.
    None,
}

/// A Ken Burns move between two framings.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Framing {
    /// 1 = the fitted image, 1.3 = 30 % closer.
    #[serde(default)]
    pub zoom: Option<f64>,
    /// Centre of the view as a fraction of the image (0.5, 0.5 = middle).
    #[serde(default)]
    pub center: Option<[f64; 2]>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct KenBurns {
    pub from: Framing,
    pub to: Framing,
    #[serde(default)]
    pub at: f64,
    /// Seconds; the layer's whole length when left out.
    #[serde(default)]
    pub dur: Option<f64>,
    #[serde(default)]
    pub ease: Option<String>,
}

/// 2.5D: displace the image by a depth map (white = near) so the camera move reveals depth.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Parallax {
    pub depth: String,
    /// Maximum displacement as a fraction of the image width, e.g. 0.03.
    pub strength: P,
    /// Where the view is looking from, -1..1 on each axis.
    #[serde(default)]
    pub look: Option<Pos>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Image {
    pub src: String,
    /// The viewport in pixels: the composition's size by default (the natural size with `fit: none`).
    #[serde(default)]
    pub size: Option<[f64; 2]>,
    #[serde(default)]
    pub fit: Fit,
    /// The point to keep when cropping, as a fraction of the image.
    #[serde(default)]
    pub focus: Option<[f64; 2]>,
    #[serde(default)]
    pub camera: Option<KenBurns>,
    #[serde(default)]
    pub parallax: Option<Parallax>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Video {
    pub src: String,
    #[serde(default)]
    pub size: Option<[f64; 2]>,
    /// Seconds into the file where playback starts.
    #[serde(default)]
    pub src_in: f64,
    #[serde(default = "one")]
    pub speed: f64,
    #[serde(default, rename = "loop")]
    pub looping: bool,
    #[serde(default)]
    pub fit: Fit,
    #[serde(default)]
    pub focus: Option<[f64; 2]>,
    #[serde(default)]
    pub volume: Option<P>,
    /// Drop the file's sound.
    #[serde(default)]
    pub mute: bool,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Audio {
    pub src: String,
    #[serde(default)]
    pub src_in: f64,
    #[serde(default)]
    pub volume: Option<P>,
    #[serde(default)]
    pub fade_in: f64,
    #[serde(default)]
    pub fade_out: f64,
    #[serde(default = "one")]
    pub speed: f64,
}

fn one() -> f64 {
    1.0
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Precomp {
    /// The name of an entry in the composition's `comps`.
    pub comp: String,
    #[serde(default = "one")]
    pub speed: f64,
    /// Seconds into the nested composition that the layer starts at.
    #[serde(default)]
    pub offset: f64,
    /// Time remapping: the nested composition's time as a property of the layer's time.
    #[serde(default)]
    pub remap: Option<P>,
    #[serde(default, rename = "loop")]
    pub looping: bool,
    /// Variables for a block: `"$name"` strings inside the nested composition take these values.
    #[serde(default)]
    pub vars: Map<String, Value>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    /// Distance in pixels from the camera to the plane that draws 1:1. Default `1.5 * width`.
    #[serde(default)]
    pub zoom: Option<P>,
    /// Alternatively a horizontal field of view in degrees.
    #[serde(default)]
    pub fov: Option<P>,
    /// Point of interest the camera looks at. Default: the middle of the composition.
    #[serde(default)]
    pub poi: Option<Pos>,
    /// Roll in degrees.
    #[serde(default)]
    pub roll: Option<P>,
    /// Depth of field: blur grows with distance from `focus`.
    #[serde(default)]
    pub dof: Option<Dof>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Dof {
    /// Distance from the camera that is sharp.
    pub focus: P,
    /// Blur in pixels per 1000 px of distance from the focus plane.
    pub blur: P,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LightKind {
    Ambient,
    Point,
    Directional,
    Spot,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Light {
    pub kind: LightKind,
    #[serde(default)]
    pub color: Option<Col>,
    #[serde(default)]
    pub intensity: Option<P>,
    /// Directional and spot lights point at this world position.
    #[serde(default)]
    pub target: Option<Pos>,
    /// Distance at which a point light has half strength, px.
    #[serde(default)]
    pub radius: Option<f64>,
    /// Spot cone half-angle, degrees.
    #[serde(default)]
    pub cone: Option<f64>,
    /// Specular highlight strength 0..1.
    #[serde(default)]
    pub shine: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Texture {
    /// A built-in generator (`noise`, `fbm`, `worley`, `halftone`, `stripes`, `grid`, `gradient`, `light_leak`...) or a plugin from `effects/`.
    pub shader: String,
    /// The generator's parameters, each a number, a colour or an animated property.
    #[serde(default)]
    pub params: Map<String, Value>,
    #[serde(default)]
    pub size: Option<[f64; 2]>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Pack {
    /// The registered pack, like `studio.globe`.
    pub name: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Emitter {
    /// `point`, `line`, `circle`, `rect` or `ring`.
    pub shape: String,
    pub size: [f64; 2],
}

impl Default for Emitter {
    fn default() -> Self {
        Emitter { shape: "point".into(), size: [0.0, 0.0] }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Burst {
    pub at: f64,
    pub count: u32,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Turbulence {
    pub amount: f64,
    #[serde(default = "hundred")]
    pub scale: f64,
    #[serde(default = "one")]
    pub speed: f64,
}

fn hundred() -> f64 {
    100.0
}

/// A particle system whose every particle is a closed-form function of its age, so any frame renders on its own.
#[derive(Debug, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Particles {
    pub emitter: Emitter,
    /// Particles per second.
    pub rate: f64,
    pub burst: Vec<Burst>,
    /// Seconds of life: `[min, max]`.
    pub life: [f64; 2],
    /// Pixels per second: `[min, max]`.
    pub speed: [f64; 2],
    /// Direction in degrees (0 = right, -90 = up) and spread (full cone width).
    pub angle: f64,
    pub spread: f64,
    pub gravity: [f64; 2],
    /// Fraction of velocity lost per second.
    pub drag: f64,
    /// Start diameter in pixels: `[min, max]`.
    pub size: [f64; 2],
    /// Size multiplier over life: keys `[[0..1, multiplier]]`.
    pub size_over_life: Vec<[f64; 2]>,
    /// Colour over life: `[[0..1, colour]]`.
    pub colors: Vec<(f64, Col)>,
    /// Opacity multiplier over life.
    pub opacity_over_life: Vec<[f64; 2]>,
    /// `circle`, `soft`, `square`, `spark`, `star`, `ring`.
    pub shape: String,
    /// Degrees per second: `[min, max]`.
    pub spin: [f64; 2],
    pub turbulence: Option<Turbulence>,
    /// Ghost copies behind each particle (1 = none).
    pub trail: u32,
    pub trail_step: f64,
    pub seed: u32,
    /// Simulate this many seconds before the layer starts, so the system is already running.
    pub prewarm: f64,
    pub max: u32,
}

impl Default for Particles {
    fn default() -> Self {
        Particles {
            emitter: Emitter::default(),
            rate: 60.0,
            burst: vec![],
            life: [1.0, 2.0],
            speed: [100.0, 200.0],
            angle: -90.0,
            spread: 40.0,
            gravity: [0.0, 0.0],
            drag: 0.0,
            size: [6.0, 12.0],
            size_over_life: vec![],
            colors: vec![],
            opacity_over_life: vec![],
            shape: "soft".into(),
            spin: [0.0, 0.0],
            turbulence: None,
            trail: 1,
            trail_step: 0.02,
            seed: 1,
            prewarm: 0.0,
            max: 20_000,
        }
    }
}
