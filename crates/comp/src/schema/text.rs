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
    /// With `by: chars`, spaces take a slot in the stagger too (a typewriter that pauses on spaces).
    pub count_spaces: bool,
}

impl Default for Animator {
    fn default() -> Self {
        Animator { by: Unit::Chars, props: UnitProps::default(), at: 0.0, dur: 0.6, ease: None, direction: Direction::In, stagger: None, selector: None, mask: false, scramble: None, count_spaces: false }
    }
}
