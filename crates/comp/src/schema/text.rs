//! Text layers: styling, and the animators that move individual characters, words and lines.

use serde::Deserialize;

use super::wrap::{Col, P};

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

/// A text layer's words: fixed text, or a number that counts.
#[derive(Debug, Clone)]
pub enum TextSource {
    Fixed(String),
    Counter(Counter),
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

impl<'de> Deserialize<'de> for TextSource {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        match v {
            serde_json::Value::String(s) => Ok(TextSource::Fixed(s)),
            serde_json::Value::Object(_) => serde_json::from_value(v).map(TextSource::Counter).map_err(serde::de::Error::custom),
            other => Err(serde::de::Error::custom(format!("text is a string or a counter {{\"from\", \"to\", \"dur\"}}, not {other}"))),
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

/// The After Effects range selector: units inside `start..end` (0..1 of the text, moved by `offset`) get weight 1.
#[derive(Debug, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Selector {
    pub start: Option<P>,
    pub end: Option<P>,
    pub offset: Option<P>,
    /// `square`, `ramp_up`, `ramp_down`, `triangle`, `round`, `smooth`.
    pub shape: String,
    /// Softness of the edges, 0..1.
    pub smooth: f64,
    pub random: bool,
    pub seed: u32,
}

impl Default for Selector {
    fn default() -> Self {
        Selector { start: None, end: None, offset: None, shape: "square".into(), smooth: 0.0, random: false, seed: 1 }
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
}

impl Default for Animator {
    fn default() -> Self {
        Animator { by: Unit::Chars, props: UnitProps::default(), at: 0.0, dur: 0.6, ease: None, direction: Direction::In, stagger: None, selector: None, mask: false, scramble: None }
    }
}
