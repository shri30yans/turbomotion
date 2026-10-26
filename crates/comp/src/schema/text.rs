//! Text layers: styling, and the animators that move individual characters, words and lines.

use serde::Deserialize;
use serde_json::Value;

use super::wrap::{Col, P};

/// Which size on the composition's three-size scale this line is for.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Big numbers and one-line headlines.
    Display,
    /// Scene titles, card headings, definition terms.
    Title,
    /// Supporting copy, labels, chips, callouts.
    Body,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Display => "display",
            Role::Title => "title",
            Role::Body => "body",
        }
    }
}

/// A highlighter that sweeps across one word of the line, instead of recolouring the whole line.
#[derive(Debug, Clone)]
pub struct Mark {
    pub word: String,
    /// 0 = first match of `word`.
    pub nth: usize,
    pub at: f64,
    pub dur: f64,
    pub ease: Option<String>,
    pub color: Option<Col>,
    /// Extra pixels around the word.
    pub pad: Option<f64>,
    /// FACT / INTERPRETATION / verify / loss: picks the ink if `color` is left out.
    pub verdict: Option<Verdict>,
    /// Ragged highlighter edge. Default on.
    pub rough: bool,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    #[default]
    Fact,
    Interpret,
    Verify,
    Loss,
}

impl Verdict {
    pub fn color(self) -> [f32; 4] {
        match self {
            Verdict::Fact => [255.0, 229.0, 102.0, 0.84],
            Verdict::Interpret => [124.0, 199.0, 255.0, 0.72],
            Verdict::Verify => [255.0, 154.0, 58.0, 0.82],
            Verdict::Loss => [255.0, 74.0, 58.0, 0.80],
        }
    }
}

fn mark_rough() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MarkObj {
    word: String,
    #[serde(default)]
    nth: usize,
    #[serde(default)]
    at: f64,
    #[serde(default = "mark_dur")]
    dur: f64,
    #[serde(default)]
    ease: Option<String>,
    #[serde(default)]
    color: Option<Col>,
    #[serde(default)]
    pad: Option<f64>,
    #[serde(default)]
    verdict: Option<Verdict>,
    #[serde(default = "mark_rough")]
    rough: bool,
}

fn mark_dur() -> f64 {
    0.45
}

impl<'de> Deserialize<'de> for Mark {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        match v {
            Value::String(word) => Ok(Mark { word, nth: 0, at: 0.0, dur: mark_dur(), ease: None, color: None, pad: None, verdict: None, rough: true }),
            obj @ Value::Object(_) => {
                let o: MarkObj = serde_json::from_value(obj).map_err(serde::de::Error::custom)?;
                Ok(Mark { word: o.word, nth: o.nth, at: o.at, dur: o.dur, ease: o.ease, color: o.color, pad: o.pad, verdict: o.verdict, rough: o.rough })
            }
            other => Err(serde::de::Error::custom(format!("mark is a word or {{\"word\": \"future\", \"at\": 1.2}}, not {other}"))),
        }
    }
}

/// One restyled stretch of a text layer's string. Matching is case-sensitive and by exact substring, so
/// `"match": "prompt."` restyles the word and its full stop.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TextRun {
    #[serde(rename = "match")]
    pub text: String,
    /// 0 = the first occurrence.
    #[serde(default)]
    pub nth: usize,
    #[serde(default)]
    pub italic: Option<bool>,
    #[serde(default)]
    pub weight: Option<u16>,
    #[serde(default)]
    pub font: Option<String>,
    #[serde(default)]
    pub color: Option<Col>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    #[default]
    Chars,
    Words,
    Lines,
}

/// A text layer's words: fixed text, a number that counts, or one string becoming another (GSAP TextPlugin).
#[derive(Debug, Clone)]
pub enum TextSource {
    Fixed(String),
    Counter(Counter),
    Swap(TextSwap),
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Counter {
    pub from: f64,
    pub to: f64,
    #[serde(default)]
    pub at: f64,
    #[serde(default = "one")]
    pub dur: f64,
    #[serde(default)]
    pub ease: Option<String>,
    #[serde(default)]
    pub decimals: usize,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub suffix: String,
    /// Thousands separator, e.g. `","`.
    #[serde(default)]
    pub sep: String,
}

fn one() -> f64 {
    1.0
}

fn swap_dur() -> f64 {
    0.6
}

/// `"text": {"from": "BUYER", "to": "SELLER", "at": 1.2, "dur": 0.5}` — the string itself changes (GSAP TextPlugin).
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TextSwap {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub at: f64,
    #[serde(default = "swap_dur")]
    pub dur: f64,
    #[serde(default)]
    pub ease: Option<String>,
}

impl<'de> Deserialize<'de> for TextSource {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        match v {
            serde_json::Value::String(s) => Ok(TextSource::Fixed(s)),
            serde_json::Value::Object(ref o) => {
                if !o.contains_key("from") && !o.contains_key("to") {
                    let keys: Vec<&str> = o.keys().map(String::as_str).collect();
                    return Err(serde::de::Error::custom(format!(
                        "an object here must be a counter {{\"from\": 0, \"to\": 100, \"dur\": 1}} or a swap {{\"from\": \"BUYER\", \"to\": \"SELLER\"}}, but it has the keys {}; the words themselves go in as a plain string (\"text\": \"Hello\"), and other options sit beside \"text\", not inside it",
                        keys.join(", ")
                    )));
                }
                let is_swap = o.get("from").is_some_and(serde_json::Value::is_string) || o.get("to").is_some_and(serde_json::Value::is_string);
                if is_swap {
                    serde_json::from_value(v).map(TextSource::Swap).map_err(serde::de::Error::custom)
                } else {
                    serde_json::from_value(v).map(TextSource::Counter).map_err(serde::de::Error::custom)
                }
            }
            other => Err(serde::de::Error::custom(format!("text is a string, a counter {{\"from\", \"to\", \"dur\"}}, or a swap {{\"from\": \"BUYER\", \"to\": \"SELLER\"}}, not {other}"))),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TextStroke {
    pub color: Col,
    pub width: f64,
}

/// What an animator does to a unit at full weight. Offsets are added to the unit's rest state.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default, deny_unknown_fields)]
pub struct UnitProps {
    pub x: Option<f64>,
    pub y: Option<f64>,
    /// Added to the scale (so `-1` shrinks to nothing, `1` doubles).
    pub scale: Option<f64>,
    /// Degrees.
    pub rotation: Option<f64>,
    /// Added to the opacity (`-1` hides the unit).
    pub opacity: Option<f64>,
    /// Gaussian sigma in pixels.
    pub blur: Option<f64>,
    /// Extra space after the unit, px.
    pub tracking: Option<f64>,
    /// The unit's colour mixes toward this at full weight.
    pub color: Option<Col>,
    /// Shear, degrees.
    pub skew: Option<f64>,
    /// Depth offset in pixels, per character (After Effects' Position Z): positive moves the character away from the
    /// viewer, as a layer's `pos` z does. Needs the text layer's perspective: see `Text.perspective`.
    pub z: Option<f64>,
    /// X Rotation in degrees, per character. Same axis and sign as the layer's `rotation: [x, y, z]`: positive tips
    /// the top of the character away. The three turns compose as a layer's do: Z reaches the character first, then Y, then X, about the
    /// unit's own centre.
    pub rotation_x: Option<f64>,
    /// Y Rotation in degrees (the layer's `rotation[1]`): at 90 the character is edge-on and has no width.
    pub rotation_y: Option<f64>,
    /// Z Rotation in degrees, clockwise; the same as `rotation`, and the two add.
    pub rotation_z: Option<f64>,
    /// Added to the depth scale (`-1` flattens it): stretches how far the turned character reaches in z, which is
    /// what perspective foreshortens. A flat character has no thickness, so it does nothing without a turn.
    pub scale_z: Option<f64>,
    /// Degrees added to the fill's hue at full weight.
    pub fill_hue: Option<f64>,
    /// Added to the fill's saturation (`-1` is grey, `0.5` is half as vivid again). Plain colour fills only.
    pub fill_saturation: Option<f64>,
    /// Added to the fill's brightness (`-1` is black).
    pub fill_brightness: Option<f64>,
    /// Pixels added to the stroke width at full weight. A glyph with no stroke of its own gets one in its fill colour.
    pub stroke_width: Option<f64>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StaggerFrom {
    #[default]
    Start,
    End,
    Center,
    Edges,
    Random,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Stagger {
    /// Seconds between one unit and the next.
    pub each: f64,
    pub from: StaggerFrom,
    pub seed: u32,
    /// An ease on the distribution of delays (GSAP's stagger ease), e.g. `"power2.in"`.
    pub ease: Option<String>,
}

impl Default for Stagger {
    fn default() -> Self {
        Stagger { each: 0.05, from: StaggerFrom::Start, seed: 1, ease: None }
    }
}

/// How a selector's weight runs across its range (After Effects' Shape).
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SelectorShape {
    /// 1 inside the range, 0 outside.
    #[default]
    Square,
    RampUp,
    RampDown,
    Triangle,
    Round,
    Smooth,
}

/// The After Effects range selector: units inside `start..end` (0..1 of the text, moved by `offset`) get weight 1.
#[derive(Debug, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Selector {
    pub start: Option<P>,
    pub end: Option<P>,
    pub offset: Option<P>,
    pub shape: SelectorShape,
    /// Softness of the edges, 0..1.
    pub smooth: f64,
    pub random: bool,
    pub seed: u32,
    /// After Effects' Wiggly selector: scales each unit's weight by its own random factor between `min` and `max`.
    pub wiggle: Option<Wiggle>,
}

impl Default for Selector {
    fn default() -> Self {
        Selector { start: None, end: None, offset: None, shape: SelectorShape::Square, smooth: 0.0, random: false, seed: 1, wiggle: None }
    }
}

/// A random factor per unit, `min..=max` (either may be negative, so a prop can go both ways). With `rate` 0 each unit keeps
/// its factor for good; above 0 the factor drifts smoothly, `rate` new random targets per second. The same seed gives the same factors.
#[derive(Debug, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Wiggle {
    pub min: f64,
    pub max: f64,
    pub rate: f64,
    pub seed: u32,
}

impl Default for Wiggle {
    fn default() -> Self {
        Wiggle { min: 0.0, max: 1.0, rate: 0.0, seed: 1 }
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Units start offset and settle to rest.
    #[default]
    In,
    /// Units start at rest and move away.
    Out,
}

/// One text animator. In GSAP mode (`stagger`, `at`, `dur`, `ease`) the weight of each unit runs 1 → 0 (in) or
/// 0 → 1 (out) on a staggered clock; in After Effects mode (`selector`) the weight is the selector's coverage.
#[derive(Debug, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Animator {
    pub by: Unit,
    pub props: UnitProps,
    pub at: f64,
    pub dur: f64,
    pub ease: Option<String>,
    pub direction: Direction,
    pub stagger: Option<Stagger>,
    pub selector: Option<Selector>,
    /// Clip every unit to its own box, so a unit moving down from `y: -1.2 * size` appears from behind a mask.
    pub mask: bool,
    /// While the weight is above 0 the unit shows random characters from this set (GSAP ScrambleText).
    pub scramble: Option<String>,
    /// With `by: chars`, spaces take a slot in the stagger too (a typewriter that pauses on spaces).
    pub count_spaces: bool,
    /// Cipher decode: while the weight is above `settle` the unit shows deterministic random characters, then locks on
    /// to the real one. Drive the per-character settle time with the stagger or the range selector.
    pub cipher: Option<Cipher>,
}

/// Scramble / decode animator (`cipher`): `{"charset": "upper", "rate": 24, "seed": 3, "settle": 0}`.
#[derive(Debug, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Cipher {
    pub charset: Charset,
    /// How many times a second the characters change.
    pub rate: f64,
    /// The same seed always gives the same noise.
    pub seed: u32,
    /// A unit locks on to its real character when its weight falls to this or below (0..1; 0 = when the animation ends).
    pub settle: f64,
}

/// Noise changes 24 times a second: fast enough to read as noise, slow enough to still read as characters at 30 fps.
const CIPHER_RATE: f64 = 24.0;

impl Default for Cipher {
    fn default() -> Self {
        Cipher { charset: Charset::Auto, rate: CIPHER_RATE, seed: 1, settle: 0.0 }
    }
}

/// Where a cipher's random characters come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Charset {
    /// The same kind as the real character: capitals for capitals, digits for digits, symbols for the rest.
    Auto,
    Upper,
    Lower,
    Digits,
    Alnum,
    Hex,
    Binary,
    Symbols,
    /// Half-width katakana and digits.
    Matrix,
    /// Any other string is the set itself.
    Literal(Vec<char>),
}

impl<'de> Deserialize<'de> for Charset {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let name = String::deserialize(d)?;
        Ok(match name.as_str() {
            "" => return Err(serde::de::Error::custom("a charset needs at least one character, or a name: auto, upper, lower, digits, alnum, hex, binary, symbols, matrix")),
            "auto" => Charset::Auto,
            "upper" => Charset::Upper,
            "lower" => Charset::Lower,
            "digits" => Charset::Digits,
            "alnum" => Charset::Alnum,
            "hex" => Charset::Hex,
            "binary" => Charset::Binary,
            "symbols" => Charset::Symbols,
            "matrix" => Charset::Matrix,
            literal => Charset::Literal(literal.chars().collect()),
        })
    }
}

impl Default for Animator {
    fn default() -> Self {
        Animator { by: Unit::Chars, props: UnitProps::default(), at: 0.0, dur: 0.6, ease: None, direction: Direction::In, stagger: None, selector: None, mask: false, scramble: None, count_spaces: false, cipher: None }
    }
}

/// Text along a path (After Effects' Path Options). The path is any shape geometry (`d`, `points`, `ellipse`, `rect`,
/// `polygon`, `star`) written in pixels from the layer's `pos`, so an `ellipse: [0, 0, 300, 300]` is a circle centred on
/// the layer. Or `layer: "ring"` borrows the first shape item of another layer.
///
/// ```json
/// "path": {"ellipse": [0, 0, 300, 300], "travel": {"from": 0, "to": 1, "dur": 6, "ease": "none"}}
/// ```
///
/// Characters advance along the arc length, so spacing and kerning survive curves, and each is turned to the tangent.
/// The alignment (`align`) decides which end of the text sits at the margin: left at the start, right at the end,
/// centre in the middle. A closed path wraps: a margin that keeps growing carries the text round and round. On an open
/// path, characters pushed past either end are hidden.
#[derive(Debug, Clone)]
pub struct TextPath {
    pub source: PathSource,
    /// Walk the path backwards (text on the other side of a circle, reading the other way).
    pub reverse: bool,
    /// Turn each character to the tangent (default). Off, characters stay upright and only ride the path.
    pub perpendicular: bool,
    /// Spread the characters so the text fills the whole stretch between the margins.
    pub force_align: bool,
    /// Pixels along the path from its start where the text begins (animatable).
    pub first_margin: Option<P>,
    /// Pixels in from the path's end where the text must stop (animatable); the right edge of right-aligned text.
    pub last_margin: Option<P>,
    /// Fractions of the path's length added to `first_margin`: animate it from 0 to 1 for exactly one lap of a closed
    /// path, so the loop has no seam whatever the path's measured length.
    pub travel: Option<P>,
}

/// Where a text path's geometry comes from.
#[derive(Debug, Clone)]
pub enum PathSource {
    Geometry(super::path::PathSpec),
    /// The first shape item of the layer with this id.
    Layer(String),
}

impl<'de> Deserialize<'de> for TextPath {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let v = Value::deserialize(d)?;
        let Value::Object(mut o) = v else { return Err(D::Error::custom("path is an object: {\"ellipse\": [0, 0, 300, 300], \"first_margin\": 0}")) };
        let take_p = |o: &mut serde_json::Map<String, Value>, k: &str| -> Result<Option<P>, D::Error> { o.remove(k).map(|v| serde_json::from_value(v).map_err(D::Error::custom)).transpose() };
        let first_margin = take_p(&mut o, "first_margin")?;
        let last_margin = take_p(&mut o, "last_margin")?;
        let travel = take_p(&mut o, "travel")?;
        let flag = |o: &mut serde_json::Map<String, Value>, k: &str, d: bool| -> Result<bool, D::Error> {
            match o.remove(k) {
                None => Ok(d),
                Some(Value::Bool(b)) => Ok(b),
                Some(other) => Err(D::Error::custom(format!("{k} is true or false, not {other}"))),
            }
        };
        let reverse = flag(&mut o, "reverse", false)?;
        let perpendicular = flag(&mut o, "perpendicular", true)?;
        let force_align = flag(&mut o, "force_align", false)?;
        let layer = match o.remove("layer") {
            None => None,
            Some(Value::String(s)) => Some(s),
            Some(other) => return Err(D::Error::custom(format!("layer is a layer id, not {other}"))),
        };
        let source = match (layer, o.is_empty()) {
            (Some(id), true) => PathSource::Layer(id),
            (None, false) => PathSource::Geometry(serde_json::from_value(Value::Object(o)).map_err(D::Error::custom)?),
            _ => return Err(D::Error::custom("path needs exactly one of a geometry (d, points, ellipse, rect, polygon, star) or a layer id")),
        };
        Ok(TextPath { source, reverse, perpendicular, force_align, first_margin, last_margin, travel })
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceKind {
    /// Text wraps round a vertical cylinder: the line bends horizontally, characters keep their height.
    #[default]
    Cylinder,
    /// Text wraps round a sphere: lines also curve toward the poles (After Effects' CC Sphere look).
    Sphere,
}

/// Wrap the text on a cylinder or sphere: every character becomes a flat plane standing on the surface, turned to
/// follow it, then seen through the text's perspective (CC Cylinder / CC Sphere look, but with live, animatable type).
/// The alignment edge of the text faces the viewer when `turn` is 0; `turn` spins the surface, so animating it scrolls
/// the text round. The axis passes through the layer's `pos`.
#[derive(Debug, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct TextSurface {
    pub kind: SurfaceKind,
    /// Pixels; the text's width over pi when left out (the text covers half a turn).
    pub radius: Option<P>,
    /// Degrees the surface has turned: positive carries the text to the right.
    pub turn: Option<P>,
    /// Draw the characters that face away (mirrored, as seen through a glass drum). Off, they are hidden.
    pub back: bool,
}

impl Default for TextSurface {
    fn default() -> Self {
        TextSurface { kind: SurfaceKind::Cylinder, radius: None, turn: None, back: false }
    }
}

/// A row of text that scrolls and repeats seamlessly, as a ticker or a marquee: the text and `gap` form one period, and
/// as many copies as the composition is wide are laid end to end, shifted by `speed` pixels a second. The row starts at
/// the layer's `pos` (left aligned: put the layer at x 0 for a full-width row).
#[derive(Debug, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Marquee {
    /// Pixels per second; negative scrolls to the right.
    pub speed: f64,
    /// Pixels between the end of one copy and the start of the next.
    pub gap: f64,
    /// How many copies to lay down; enough to cover the composition's width when left out.
    pub repeat: Option<std::num::NonZeroU32>,
    /// Pixels the row has already travelled at time 0.
    pub offset: f64,
}

/// A ticker's usual pace and spacing, in pixels.
const MARQUEE_SPEED: f64 = -120.0;
const MARQUEE_GAP: f64 = 80.0;

impl Default for Marquee {
    fn default() -> Self {
        Marquee { speed: MARQUEE_SPEED, gap: MARQUEE_GAP, repeat: None, offset: 0.0 }
    }
}

/// How per-character depth is seen on a flat layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Perspective {
    /// A viewer this many pixels away: smaller is stronger foreshortening.
    Distance(f64),
    /// No foreshortening: depth only shows through the turns.
    Orthographic,
}

impl<'de> Deserialize<'de> for Perspective {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let v = Value::deserialize(d)?;
        match (&v, v.as_f64()) {
            (Value::String(s), _) if s == "orthographic" => Ok(Perspective::Orthographic),
            (Value::Number(_), Some(px)) if px > 0.0 => Ok(Perspective::Distance(px)),
            _ => Err(D::Error::custom(format!("perspective is a distance in pixels above 0, or \"orthographic\", not {v}"))),
        }
    }
}
