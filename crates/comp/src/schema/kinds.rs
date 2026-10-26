//! What each kind of layer carries beyond the fields every layer has.

use serde::Deserialize;
use serde_json::{Map, Value};

use super::{
    shape::ShapeItem,
    text::{Align, Animator, TextSource, TextStroke},
    wrap::{Col, Pos, P},
};

// one `Kind` per layer in a plan; boxing the big variants would only add an indirection to every match
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum Kind {
    Solid(Solid),
    Shape(Shape),
    Text(Text),
    Mesh(MeshLayer),
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
    pub const NAMES: &'static [&'static str] = &["solid", "shape", "text", "mesh", "image", "video", "audio", "precomp", "null", "camera", "light", "adjustment", "particles", "texture", "pack"];

    /// The kind's name, as it is written in a composition file.
    pub fn name(&self) -> &'static str {
        match self {
            Kind::Solid(_) => "solid",
            Kind::Shape(_) => "shape",
            Kind::Text(_) => "text",
            Kind::Mesh(_) => "mesh",
            Kind::Image(_) => "image",
            Kind::Video(_) => "video",
            Kind::Audio(_) => "audio",
            Kind::Precomp(_) => "precomp",
            Kind::Null => "null",
            Kind::Camera(_) => "camera",
            Kind::Light(_) => "light",
            Kind::Adjustment => "adjustment",
            Kind::Particles(_) => "particles",
            Kind::Texture(_) => "texture",
            Kind::Pack(_) => "pack",
        }
    }
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
    /// A gradient or `"metal"` fill; when set it paints the glyphs instead of `color`.
    #[serde(default)]
    pub fill: Option<super::shape::Fill>,
    /// Sweep a highlighter across this word of the line.
    #[serde(default)]
    pub mark: Option<super::text::Mark>,
    /// Restyle part of the line: an italic accent word, a bold number, another face for a unit. Each run matches
    /// text (`"match": "prompt."`) and overrides only the fields it names.
    #[serde(default)]
    pub runs: Vec<super::text::TextRun>,
    /// Text that belongs to a mocked interface (a menu label, a status line): it is decoration at a deliberate size,
    /// so the phone-legibility size rule does not apply. Safe area and contrast are still checked.
    #[serde(default)]
    pub ui: bool,
    /// Which size on the composition's type scale this line is: `display`, `title` or `body`.
    #[serde(default)]
    pub role: Option<super::text::Role>,
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
    /// Outline-only type: the fill is not painted, only the stroke (the layer's colour at 2 px when no `stroke` is given).
    #[serde(default)]
    pub outline_only: bool,
    /// Draw only the part of the glyph outlines between fractions `[start, end]` (0..1) of their total length, in reading
    /// order: animate `end` from 0 to 1 and the letters write themselves. Needs a stroke (or `outline_only`).
    #[serde(default)]
    pub stroke_trim: Option<[P; 2]>,
    /// With `stroke_trim`, trim each glyph's own outline instead of the whole line's.
    #[serde(default)]
    pub stroke_trim_each: bool,
    /// Lay the text along a path (After Effects' Path Options).
    #[serde(default)]
    pub path: Option<super::text::TextPath>,
    /// Wrap the text on a cylinder or a sphere.
    #[serde(default)]
    pub surface: Option<super::text::TextSurface>,
    /// A seamlessly repeating, scrolling row of the text.
    #[serde(default)]
    pub marquee: Option<super::text::Marquee>,
    /// How per-character 3D (animator `z` / `rotation_*`, and `surface`) is seen on a flat layer: a viewer distance in
    /// pixels (smaller is stronger foreshortening) or `"orthographic"`. Default the default camera's zoom
    /// (`default_camera_zoom`). A `three_d` layer is seen through the composition camera instead and ignores this. The vanishing point
    /// is the layer's `pos`.
    #[serde(default)]
    pub perspective: Option<super::text::Perspective>,
    #[serde(default)]
    pub animators: Vec<Animator>,
    /// A named animation from the catalogue, expanded into animators: `"rise"` or `{"name": "rise", "by": "words"}`.
    #[serde(default)]
    pub preset: Option<Value>,
}

impl Text {
    /// The rules a text layer's fields must keep together, checked once when the composition is planned.
    pub fn validate(&self) -> anyhow::Result<()> {
        use anyhow::ensure;
        ensure!(self.path.is_none() || self.surface.is_none(), "text takes a `path` or a `surface`, not both");
        ensure!(self.stroke_trim.is_none() || self.stroke.is_some() || self.outline_only, "`stroke_trim` trims the stroke: give the text a `stroke` or `outline_only`");
        if let Some(m) = &self.marquee {
            ensure!(m.speed.is_finite() && m.offset.is_finite(), "marquee speed and offset are finite numbers");
            ensure!(m.gap >= 0.0, "marquee gap is 0 or more pixels, not {}", m.gap);
        }
        for c in self.animators.iter().filter_map(|a| a.cipher.as_ref()) {
            ensure!(c.rate > 0.0, "cipher rate is above 0 changes per second, not {}", c.rate);
            ensure!((0.0..1.0).contains(&c.settle), "cipher settle is 0 up to 1, not {}", c.settle);
        }
        Ok(())
    }
}

/// A solid: a triangle mesh made by one of the `geometry` generators, lit by the composition's lights and seen through
/// its camera. Rotate it with the common `rotation`; it turns about its own middle. `text3d` is the
/// same layer with a text geometry written flat.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct MeshLayer {
    pub geometry: Geometry,
    /// The front face (and a curved surface's whole skin).
    #[serde(default)]
    pub color: Option<Col>,
    /// The sides and the bevel; the front colour darkened when left out.
    #[serde(default)]
    pub side: Option<Col>,
    /// The back face; `side` when left out.
    #[serde(default)]
    pub back: Option<Col>,
    /// Smoothness of curved geometry: segments around a full turn (48).
    #[serde(default)]
    pub segments: Option<u32>,
}

/// What a mesh is made of. Sizes are pixels.
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Geometry {
    /// One line of text, each glyph outline extruded; several glyphs share one mesh (one layer per letter to stagger a word).
    Text {
        text: String,
        #[serde(default)]
        font: Option<String>,
        #[serde(default)]
        weight: Option<u16>,
        #[serde(default)]
        italic: bool,
        /// Font size.
        size: P,
        /// Extrusion; 0.2 x size when left out.
        #[serde(default)]
        depth: Option<P>,
        /// Chamfer on the front and back edges; 0 (the default) is a hard edge.
        #[serde(default)]
        bevel: Option<P>,
    },
    /// A box `size` wide; `height` and `depth` default to `size`.
    Box {
        size: P,
        #[serde(default)]
        height: Option<P>,
        #[serde(default)]
        depth: Option<P>,
    },
    /// A sphere of diameter `size`.
    Sphere { size: P },
    /// A ring facing the viewer: `size` is its outer diameter, `tube` the thickness of its band (a quarter of `size`).
    Torus {
        size: P,
        #[serde(default)]
        tube: Option<P>,
    },
    /// A cylinder of diameter `size` and `height`; `top` is the top diameter (0 makes a cone).
    Cylinder {
        size: P,
        height: P,
        #[serde(default)]
        top: Option<P>,
    },
    /// An SVG path `d` (filled) scaled to `size` wide and pushed `depth` deep (0.3 x size when left out).
    Extrude {
        d: String,
        size: P,
        #[serde(default)]
        depth: Option<P>,
        #[serde(default)]
        bevel: Option<P>,
    },
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

/// Explicit raster filtering. Omission retains each layer kind's default.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RasterSampling {
    Nearest,
    Linear,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Image {
    pub src: String,
    /// Nearest preserves source pixels; linear interpolates without mipmaps. None keeps automatic filtering.
    #[serde(default)]
    pub sampling: Option<RasterSampling>,
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
    /// Filtering when the rendered composition is transformed. None keeps linear filtering.
    #[serde(default)]
    pub sampling: Option<RasterSampling>,
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

/// The default camera sees this fraction of the composition's width per unit of zoom: 36 mm of film over a 50 mm lens.
///
/// Source: py-aep's `default_camera_zoom`, measured in After Effects 2026 (the 50 mm preset): the default
/// camera's zoom in pixels is `width / 0.72`. After Effects scales that by the pixel aspect; this engine has no
/// pixel aspect (every pixel is square), so it is 1.
pub const DEFAULT_CAMERA_FRAME_WIDTH: f64 = 0.72;

/// The zoom (camera distance to the 1:1 plane, pixels) of a camera that sets none, for a composition `width` wide.
pub fn default_camera_zoom(width: f64) -> f64 {
    width / DEFAULT_CAMERA_FRAME_WIDTH
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    /// Distance in pixels from the camera to the plane that draws 1:1. Default `width / 0.72`, After Effects' 50 mm camera.
    #[serde(default)]
    pub zoom: Option<P>,
    /// Alternatively a horizontal field of view in degrees.
    #[serde(default)]
    pub fov: Option<P>,
    /// Point of interest the camera looks at. Default: the middle of the composition.
    #[serde(default)]
    pub poi: Option<Pos>,
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
    /// Fraction of velocity lost per second; negative accelerates (speed grows as e^(-drag t)), for tunnels and warp streaks.
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
