//! The compiled film (`film.json`, written by the studio plan compiler): scenes with every narration
//! anchor already resolved to seconds, and typed elements. Unknown element kinds or parameters are hard errors, so a
//! plan can never silently lose an effect in the native renderer.

use std::{collections::HashMap, fs, path::{Path, PathBuf}};

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{Map, Value};

pub const W: i32 = 1920;
pub const H: i32 = 1080;

pub type Rgb = [u8; 3];

pub const INK: Rgb = [9, 12, 18];
pub const INK2: Rgb = [16, 22, 32];
pub const STEEL: Rgb = [104, 126, 150];
pub const PAPER: Rgb = [236, 231, 219];
pub const WHITE: Rgb = [246, 244, 238];
pub const RED: Rgb = [232, 64, 47];
pub const GREEN: Rgb = [62, 182, 150];
pub const GOLD: Rgb = [222, 176, 82];

#[derive(Debug, Deserialize)]
pub struct Film {
    pub fps: u32,
    pub width: i32,
    pub height: i32,
    /// Film length in seconds.
    pub end: f64,
    /// Font role -> (file under `font_dir`, collection index).
    pub fonts: HashMap<String, (String, u32)>,
    pub font_dir: String,
    pub scenes: Vec<Scene>,
    /// The directory the film was loaded from; its geography, photographs and characters live beside `film.json`.
    #[serde(skip)]
    pub dir: PathBuf,
}

impl Film {
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join("film.json");
        let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let mut film: Self = serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        film.dir = dir.to_path_buf();
        if film.width != W || film.height != H {
            bail!("the studio renderer draws {W}x{H} films, this one is {}x{}", film.width, film.height);
        }
        film.check_timeline()?;
        Ok(film)
    }

    /// Scenes must follow each other without gaps or overlaps, from the start of the film to its end.
    fn check_timeline(&self) -> Result<()> {
        const TOLERANCE: f64 = 1e-6;
        let ends = self.scenes.iter().skip(1).map(|s| s.t0).chain(std::iter::once(self.end));
        for (scene, next) in self.scenes.iter().zip(ends) {
            if (scene.t1 - next).abs() > TOLERANCE || scene.t0 >= scene.t1 {
                bail!("scene {:?} runs {:.3}s to {:.3}s but the next scene (or the end of the film) is at {next:.3}s", scene.id, scene.t0, scene.t1);
            }
        }
        Ok(())
    }

    pub fn frames(&self) -> u32 {
        (self.end * f64::from(self.fps)) as u32
    }

    /// Index of the scene on screen at `t` (the last scene that has started).
    pub fn scene_at(&self, t: f64) -> usize {
        self.scenes.iter().rposition(|s| s.t0 <= t + 1e-6).unwrap_or(0)
    }
}

#[derive(Debug, Deserialize)]
pub struct Scene {
    pub id: String,
    pub t0: f64,
    pub t1: f64,
    pub trans: String,
    pub trd: f64,
    pub grade: Grade,
    pub vig: f64,
    pub bloom: f64,
    pub ca: f64,
    pub bg: Background,
    /// (time, zoom, centre x, centre y, rotation degrees)
    pub cam: Vec<[f64; 5]>,
    /// (time, amplitude px)
    pub shakes: Vec<[f64; 2]>,
    pub els: Vec<Element>,
}

/// `fx.grade` keyword arguments; an empty grade is skipped entirely.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Grade {
    pub lift: Option<f64>,
    pub gamma: Option<f64>,
    pub gain: Option<f64>,
    pub sat: Option<f64>,
    pub tint: Option<Rgb>,
    pub tint_k: Option<f64>,
    pub contrast: Option<f64>,
}

impl Grade {
    pub fn is_empty(&self) -> bool {
        self.lift.is_none() && self.gamma.is_none() && self.gain.is_none() && self.sat.is_none() && self.tint.is_none() && self.tint_k.is_none() && self.contrast.is_none()
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Background {
    pub style: String,
    pub grid: bool,
    pub grid_a: f64,
    pub dust: bool,
    pub seed: u32,
    pub sweep: bool,
    pub period: f64,
}

impl Default for Background {
    fn default() -> Self {
        Self { style: "studio".into(), grid: true, grid_a: 0.14, dust: true, seed: 3, sweep: true, period: 8.0 }
    }
}

/// Keys every element understands; they steer the shared in/out animation. Kinds read `d` (animation length)
/// and `op` (opacity) from here too.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Common {
    pub t0: f64,
    /// `None` = stays until the scene ends.
    pub t1: Option<f64>,
    #[serde(rename = "in")]
    pub enter: Option<String>,
    pub out: Option<String>,
    pub d: Option<f64>,
    pub dout: Option<f64>,
    pub op: Option<f64>,
    pub overlap_ok: bool,
    pub sfx: Option<String>,
    pub sfx_v: Option<f64>,
}

const COMMON_KEYS: [&str; 10] = ["t0", "t1", "in", "out", "d", "dout", "op", "overlap_ok", "sfx", "sfx_v"];

#[derive(Debug, Clone)]
pub struct Element {
    pub common: Common,
    pub kind: Kind,
}

impl<'de> Deserialize<'de> for Element {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let mut map = Map::<String, Value>::deserialize(de)?;
        let common: Map<String, Value> = COMMON_KEYS.iter().filter_map(|k| map.remove(*k).map(|v| (k.to_string(), v))).collect();
        let common: Common = serde_json::from_value(Value::Object(common)).map_err(D::Error::custom)?;
        let name = map.get("k").and_then(Value::as_str).unwrap_or("?").to_string();
        let kind = Kind::deserialize(Value::Object(map)).map_err(|e| D::Error::custom(format!("element {name:?}: {e}")))?;
        Ok(Self { common, kind })
    }
}

fn w2() -> f64 {
    f64::from(W) / 2.0
}
fn h2() -> f64 {
    f64::from(H) / 2.0
}
fn one() -> f64 {
    1.0
}
fn yes() -> bool {
    true
}
fn white() -> Rgb {
    WHITE
}
fn steel() -> Rgb {
    STEEL
}
fn red() -> Rgb {
    RED
}
fn gold() -> Rgb {
    GOLD
}
fn green() -> Rgb {
    GREEN
}
fn ink() -> Rgb {
    INK
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "k", rename_all = "snake_case")]
pub enum Kind {
    Text(Text),
    Title(Title),
    Counter(Counter),
    Chip(Chip),
    Card(Card),
    Stamp(Stamp),
    Rect(RectEl),
    Ring(Ring),
    Arrow(Arrow),
    Packets(Packets),
    Linechart(LineChart),
    Bars(Bars),
    Jar(Jar),
    Tiles(Tiles),
    Doc(Doc),
    List(List),
    Ticker(Ticker),
    Map(MapEl),
    Globe(GlobeEl),
    Plate(Plate),
    Char(CharEl),
    Flash(Flash),
    Scrim(Scrim),
    VignetteRed(VignetteRed),
}

macro_rules! defaults {
    ($($name:ident: $ty:ty = $val:expr;)*) => { $(fn $name() -> $ty { $val })* };
}

defaults! {
    d_avn_b: String = "avn_b".into();
    d_avn_m: String = "avn_m".into();
    d_dinc: String = "dinc".into();
    d_center: String = "center".into();
    d_left: String = "left".into();
    d_out: String = "out".into();
    d_io: String = "io".into();
    d_acme: String = "ACME CORP".into();
    d_card_bg: Rgb = [18, 26, 38];
    d_card_border: Rgb = [84, 106, 134];
    d_land: Rgb = [46, 62, 84];
    d_border: Rgb = [100, 124, 152];
    f2: f64 = 2.0;
    f4: f64 = 4.0;
    f5: f64 = 5.0;
    f6: f64 = 6.0;
    f8: f64 = 8.0;
    f15: f64 = 15.0;
    f22: f64 = 22.0;
    f26: f64 = 26.0;
    f28: f64 = 28.0;
    f30: f64 = 30.0;
    f34: f64 = 34.0;
    f46: f64 = 46.0;
    f52: f64 = 52.0;
    f54: f64 = 54.0;
    f60: f64 = 60.0;
    f64_: f64 = 64.0;
    f70: f64 = 70.0;
    f74: f64 = 74.0;
    f120: f64 = 120.0;
    f200: f64 = 200.0;
    f220: f64 = 220.0;
    f260: f64 = 260.0;
    f300: f64 = 300.0;
    f380: f64 = 380.0;
    f520: f64 = 520.0;
    f640: f64 = 640.0;
    f760: f64 = 760.0;
    f780: f64 = 780.0;
    f980: f64 = 980.0;
    f120a: f64 = 120.0;
    fm6: f64 = -6.0;
    fm25: f64 = -2.5;
    fm34: f64 = -34.0;
    fm44: f64 = -44.0;
    fm48: f64 = -48.0;
    f01: f64 = 0.1;
    f03: f64 = 0.30;
    f035: f64 = 0.35;
    f05: f64 = 0.5;
    f08: f64 = 0.8;
    f085: f64 = 0.85;
    f09: f64 = 0.9;
    f092: f64 = 0.92;
    f14: f64 = 1.4;
    f16: f64 = 1.6;
    f028: f64 = 0.28;
    u2: u32 = 2;
    u5: u32 = 5;
    u6: u32 = 6;
    u15: u32 = 15;
    i_m1: i64 = -1;
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Text {
    pub text: String,
    #[serde(default = "d_avn_b")]
    pub font: String,
    #[serde(default = "f52")]
    pub size: f64,
    #[serde(default = "white")]
    pub fill: Rgb,
    #[serde(default)]
    pub tracking: f64,
    #[serde(default = "d_center")]
    pub align: String,
    pub maxw: Option<f64>,
    #[serde(default = "w2")]
    pub x: f64,
    #[serde(default = "h2")]
    pub y: f64,
    #[serde(default)]
    pub rot: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Title {
    pub text: String,
    #[serde(default = "d_dinc")]
    pub font: String,
    #[serde(default = "f200")]
    pub size: f64,
    #[serde(default = "white")]
    pub fill: Rgb,
    #[serde(default = "f30")]
    pub depth: f64,
    #[serde(default)]
    pub tracking: f64,
    pub refl: Option<f64>,
    #[serde(default = "w2")]
    pub x: f64,
    #[serde(default = "h2")]
    pub y: f64,
    pub glow: Option<Rgb>,
    #[serde(default = "one")]
    pub scale: f64,
    #[serde(default)]
    pub rot: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Counter {
    pub v0: f64,
    pub v1: f64,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub suffix: String,
    #[serde(default)]
    pub dec: usize,
    #[serde(default = "f220")]
    pub size: f64,
    #[serde(default = "white")]
    pub fill: Rgb,
    #[serde(default = "d_dinc")]
    pub font: String,
    #[serde(default = "f30")]
    pub depth: f64,
    #[serde(default = "f2")]
    pub count: f64,
    #[serde(default = "f01")]
    pub delay: f64,
    #[serde(default = "d_out")]
    pub ease: String,
    pub glow: Option<Rgb>,
    #[serde(default = "w2")]
    pub x: f64,
    #[serde(default = "h2")]
    pub y: f64,
    #[serde(default = "one")]
    pub scale: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Chip {
    pub text: String,
    #[serde(default = "f30")]
    pub size: f64,
    #[serde(default = "steel")]
    pub fill: Rgb,
    pub x: Option<f64>,
    #[serde(default = "f70")]
    pub y: f64,
}

/// One line of a card: text, font role, size and optional colour.
#[derive(Debug, Deserialize, Clone)]
pub struct CardLine(pub String, pub String, pub f64, #[serde(default)] pub Option<Rgb>);

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Card {
    #[serde(default = "f520")]
    pub w: f64,
    #[serde(default = "f260")]
    pub h: f64,
    #[serde(default = "d_left")]
    pub align: String,
    pub title: Option<String>,
    #[serde(default = "f28")]
    pub tsize: f64,
    #[serde(default = "steel")]
    pub tfill: Rgb,
    #[serde(default)]
    pub lines: Vec<CardLine>,
    #[serde(default = "f22")]
    pub radius: f64,
    #[serde(default = "d_card_bg")]
    pub bg: Rgb,
    #[serde(default = "f092")]
    pub bg_a: f64,
    #[serde(default = "d_card_border")]
    pub border: Rgb,
    pub accent: Option<Rgb>,
    #[serde(default = "w2")]
    pub x: f64,
    #[serde(default = "h2")]
    pub y: f64,
    #[serde(default)]
    pub rot: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Stamp {
    pub text: String,
    #[serde(default = "f120")]
    pub size: f64,
    #[serde(default = "d_dinc")]
    pub font: String,
    #[serde(default = "red")]
    pub fill: Rgb,
    #[serde(default = "f4")]
    pub tracking: f64,
    #[serde(default = "w2")]
    pub x: f64,
    #[serde(default = "h2")]
    pub y: f64,
    #[serde(default = "fm6")]
    pub rot: f64,
    #[serde(default = "one")]
    pub scale: f64,
    /// Seed of the worn-ink texture (`crc32(text) % 9999`, written by the exporter).
    pub seed: u32,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct RectEl {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(default = "steel")]
    pub fill: Rgb,
    #[serde(default)]
    pub radius: f64,
    #[serde(default = "one")]
    pub a: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Ring {
    pub x: f64,
    pub y: f64,
    #[serde(default = "f60")]
    pub r: f64,
    #[serde(default = "red")]
    pub fill: Rgb,
    #[serde(default = "u2")]
    pub n: u32,
    #[serde(default = "f16")]
    pub dur: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Arrow {
    pub pts: Vec<[f64; 2]>,
    #[serde(default = "white")]
    pub fill: Rgb,
    #[serde(default = "f6")]
    pub w: f64,
    #[serde(default)]
    pub dash: bool,
    #[serde(default = "yes")]
    pub head: bool,
    #[serde(default = "f26")]
    pub hs: f64,
    #[serde(default)]
    pub glow: bool,
    pub pulse: Option<u32>,
    #[serde(default = "f05")]
    pub speed: f64,
    #[serde(default = "gold")]
    pub pc: Rgb,
    pub label: Option<String>,
    #[serde(default = "f28")]
    pub lsize: f64,
    #[serde(default = "white")]
    pub lfill: Rgb,
    #[serde(default)]
    pub lx: f64,
    #[serde(default = "fm34")]
    pub ly: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Packets {
    pub pts: Vec<[f64; 2]>,
    #[serde(default = "u6")]
    pub n: u32,
    #[serde(default = "f035")]
    pub speed: f64,
    #[serde(default = "gold")]
    pub fill: Rgb,
    #[serde(default = "f15")]
    pub r: f64,
    pub sym: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct LineChart {
    pub rect: [f64; 4],
    pub data: Option<Vec<f64>>,
    /// (time, value) keys of the word-synced mode.
    pub keys: Option<Vec<(f64, f64)>>,
    pub span: Option<[f64; 2]>,
    pub ymin: Option<f64>,
    pub ymax: Option<f64>,
    #[serde(default = "d_io")]
    pub ease: String,
    #[serde(default = "f2")]
    pub draw: f64,
    pub baseline: Option<f64>,
    #[serde(default = "green")]
    pub fill: Rgb,
    #[serde(default = "red")]
    pub fill2: Rgb,
    pub value_label: Option<String>,
    #[serde(default = "f54")]
    pub vsize: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BarItem {
    pub label: String,
    pub v: f64,
    #[serde(default)]
    pub text: String,
    pub fill: Option<Rgb>,
    pub t0: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Bars {
    pub items: Vec<BarItem>,
    pub rect: [f64; 4],
    pub vmax: Option<f64>,
    #[serde(default = "f035")]
    pub gap: f64,
    #[serde(default = "f46")]
    pub depth: f64,
    #[serde(default = "f09")]
    pub grow: f64,
    #[serde(default = "f34")]
    pub lsize: f64,
    #[serde(default = "f64_")]
    pub vsize: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Jar {
    pub x: f64,
    pub y: f64,
    #[serde(default = "f300")]
    pub w: f64,
    #[serde(default = "f380")]
    pub h: f64,
    /// (time, fill level 0..1)
    pub levels: Vec<(f64, f64)>,
    #[serde(default = "gold")]
    pub fill: Rgb,
    pub label: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Tiles {
    #[serde(default = "u15")]
    pub cols: u32,
    #[serde(default = "u15")]
    pub rows: u32,
    pub rect: [f64; 4],
    pub merge_at: Option<f64>,
    pub merge_y: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DocRow {
    pub text: String,
    #[serde(default = "f34")]
    pub size: f64,
    #[serde(default = "d_avn_m")]
    pub font: String,
    pub fill: Option<Rgb>,
    pub hl: Option<f64>,
    pub hlc: Option<Rgb>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Doc {
    #[serde(default = "f640")]
    pub w: f64,
    #[serde(default = "f760")]
    pub h: f64,
    pub head: Option<String>,
    #[serde(default)]
    pub rows: Vec<DocRow>,
    #[serde(default = "w2")]
    pub x: f64,
    #[serde(default = "h2")]
    pub y: f64,
    #[serde(default = "fm25")]
    pub rot: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct List {
    /// Each row: left text and an optional right-aligned value.
    pub rows: Vec<Vec<String>>,
    #[serde(default = "f640")]
    pub w: f64,
    #[serde(default = "f74")]
    pub rh: f64,
    pub hi_at: Option<f64>,
    #[serde(default = "i_m1")]
    pub hi: i64,
    #[serde(default = "w2")]
    pub x: f64,
    #[serde(default = "h2")]
    pub y: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Ticker {
    #[serde(default = "f640")]
    pub w: f64,
    #[serde(default = "f300")]
    pub h: f64,
    pub price: Vec<(f64, f64)>,
    pub base: Option<f64>,
    #[serde(default = "d_acme")]
    pub name: String,
    pub shares: Option<String>,
    #[serde(default = "w2")]
    pub x: f64,
    #[serde(default = "h2")]
    pub y: f64,
}

/// A map view: centre longitude, centre latitude, horizontal span in degrees.
pub type ViewKey = [f64; 3];

#[derive(Debug, Deserialize, Clone)]
pub struct MapFill {
    pub at: f64,
    pub countries: Option<Vec<String>>,
    pub states: Option<Vec<String>>,
    #[serde(default = "red")]
    pub fill: Rgb,
    #[serde(default = "f085")]
    pub a: f64,
    #[serde(default = "white")]
    pub edge: Rgb,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MapArc {
    pub at: f64,
    pub a: [f64; 2],
    pub b: [f64; 2],
    #[serde(default = "f14")]
    pub dur: f64,
    #[serde(default = "f120a")]
    pub lift: f64,
    #[serde(default = "gold")]
    pub fill: Rgb,
    #[serde(default = "f5")]
    pub w: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MapPin {
    pub at: f64,
    pub lon: f64,
    pub lat: f64,
    #[serde(default = "f8")]
    pub r: f64,
    #[serde(default = "red")]
    pub fill: Rgb,
    #[serde(default)]
    pub pulse: f64,
    pub label: Option<String>,
    #[serde(default = "f30")]
    pub lsize: f64,
    #[serde(default)]
    pub dx: f64,
    #[serde(default = "fm44")]
    pub dy: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct MapEl {
    pub views: Vec<(f64, ViewKey)>,
    #[serde(default = "d_land")]
    pub land: Rgb,
    #[serde(default = "d_border")]
    pub border: Rgb,
    #[serde(default)]
    pub fills: Vec<MapFill>,
    #[serde(default)]
    pub arcs: Vec<MapArc>,
    #[serde(default)]
    pub pins: Vec<MapPin>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GlobeHighlight {
    pub at: f64,
    pub countries: Vec<String>,
    pub fill: Rgb,
    #[serde(default = "f09")]
    pub a: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GlobePackets {
    pub at: f64,
    #[serde(default = "u5")]
    pub n: u32,
    #[serde(default = "f03")]
    pub speed: f64,
    #[serde(default = "white")]
    pub fill: Rgb,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GlobeArc {
    pub at: f64,
    pub a: [f64; 2],
    pub b: [f64; 2],
    #[serde(default = "f16")]
    pub dur: f64,
    #[serde(default = "f028")]
    pub lift: f64,
    #[serde(default = "gold")]
    pub fill: Rgb,
    #[serde(default = "f6")]
    pub w: f64,
    pub packets: Option<GlobePackets>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GlobePin {
    pub at: f64,
    pub lon: f64,
    pub lat: f64,
    #[serde(default = "red")]
    pub fill: Rgb,
    pub label: Option<String>,
    #[serde(default = "f34")]
    pub lsize: f64,
    #[serde(default)]
    pub dx: f64,
    #[serde(default = "fm48")]
    pub dy: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct GlobeEl {
    #[serde(default = "w2")]
    pub x: f64,
    #[serde(default = "h2")]
    pub y: f64,
    #[serde(rename = "D", default = "f980")]
    pub diameter: f64,
    /// (time, [centre longitude, centre latitude])
    pub rot: Vec<(f64, [f64; 2])>,
    #[serde(default)]
    pub spin: f64,
    #[serde(default)]
    pub highlights: Vec<GlobeHighlight>,
    #[serde(default)]
    pub arcs: Vec<GlobeArc>,
    #[serde(default)]
    pub pins: Vec<GlobePin>,
    #[serde(default = "one")]
    pub scale: f64,
}

/// (zoom, centre x, centre y) at the start and end of the Ken Burns move.
pub type KenBurns = [[f64; 3]; 2];

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Plate {
    pub name: String,
    pub kb: Option<KenBurns>,
    #[serde(default = "f6")]
    pub dur: f64,
    pub focus: Option<[f64; 2]>,
    #[serde(default = "f6")]
    pub dof: f64,
    #[serde(default)]
    pub grade: Grade,
    #[serde(default)]
    pub scrim: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct CharEl {
    /// (time, pose name)
    pub poses: Vec<(f64, String)>,
    #[serde(default = "f780")]
    pub h: f64,
    pub x: Option<f64>,
    pub y: Option<f64>,
    #[serde(default)]
    pub flip: bool,
    #[serde(default)]
    pub sway: f64,
    #[serde(default)]
    pub shiver: bool,
    #[serde(default = "one")]
    pub scale: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Flash {
    #[serde(default = "f08")]
    pub a: f64,
    #[serde(default = "f035")]
    pub dur: f64,
    #[serde(default = "white")]
    pub fill: Rgb,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Scrim {
    #[serde(default = "f05")]
    pub a: f64,
    #[serde(default = "ink")]
    pub fill: Rgb,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct VignetteRed {
    #[serde(default = "f05")]
    pub a: f64,
}
