//! Map definitions (`assets/maps/views.json`): which view, pins, routes, rings, areas, labels and
//! schematic elements make up each named map, and when each appears.

use std::{collections::HashMap, fs, path::Path};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::film::Shot;

/// `[longitude, latitude]` in degrees.
pub type LonLat = [f64; 2];

/// A time within a shot: seconds, or `"cN"` / `"cN+0.4"` for cue `N` plus an offset.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum When {
    Seconds(f32),
    Cue(String),
}

impl When {
    /// Seconds into `shot`.
    pub fn resolve(&self, shot: &Shot) -> Result<f32> {
        match self {
            Self::Seconds(s) => Ok(*s),
            Self::Cue(text) => {
                let rest = text.strip_prefix('c').with_context(|| format!("{}: time {text:?} is neither seconds nor a cue", shot.id))?;
                let digits = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
                let index: usize = rest[..digits].parse().with_context(|| format!("{}: bad cue in time {text:?}", shot.id))?;
                let offset = match &rest[digits..] {
                    "" => 0.0,
                    sign if sign.starts_with(['+', '-']) => sign.parse().with_context(|| format!("{}: bad offset in time {text:?}", shot.id))?,
                    _ => bail!("{}: bad time {text:?}", shot.id),
                };
                Ok(shot.cue_at(index, None) + offset)
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct View {
    pub center: LonLat,
    /// Longitudes spanned by the frame width.
    pub span: f64,
    pub at: Option<When>,
    pub dur: Option<f32>,
    pub step: Option<u32>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PinStyle {
    Home,
}

#[derive(Debug, Deserialize)]
pub struct Pin {
    pub place: String,
    pub label: Option<String>,
    pub sub: Option<String>,
    pub at: Option<When>,
    pub step: Option<u32>,
    pub style: Option<PinStyle>,
    pub dx: Option<f32>,
    pub dy: Option<f32>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RouteColor {
    Teal,
    Terracotta,
}

#[derive(Debug, Deserialize)]
pub struct Route {
    pub via: Vec<String>,
    pub at: Option<When>,
    pub dur: Option<f32>,
    pub step: Option<u32>,
    pub label: Option<String>,
    #[serde(default)]
    pub dashed: bool,
    pub color: Option<RouteColor>,
    pub bend: Option<f32>,
    #[serde(rename = "labelDx")]
    pub label_dx: Option<f32>,
    #[serde(rename = "labelDy")]
    pub label_dy: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct Ring {
    pub center: LonLat,
    pub km: f32,
    pub label: Option<String>,
    pub at: Option<When>,
    pub step: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct Area {
    pub state: String,
    pub at: Option<When>,
    pub step: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct Label {
    pub text: String,
    #[serde(rename = "at_")]
    pub position: LonLat,
    pub at: Option<When>,
    pub step: Option<u32>,
    pub size: Option<f32>,
    #[serde(default)]
    pub faint: bool,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum SchematicKind {
    Hills,
    Lake,
    Grid,
}

#[derive(Debug, Deserialize)]
pub struct Schematic {
    pub kind: SchematicKind,
    /// `[lon, lat]`, plus a radius in km for hills.
    pub pts: Vec<Vec<f64>>,
    pub at: Option<When>,
    pub step: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct MapSpec {
    pub title: Option<String>,
    pub legend: Option<String>,
    pub note: Option<String>,
    #[serde(default)]
    pub home: Vec<String>,
    pub views: Vec<View>,
    #[serde(default)]
    pub pins: Vec<Pin>,
    #[serde(default)]
    pub routes: Vec<Route>,
    #[serde(default)]
    pub rings: Vec<Ring>,
    #[serde(default)]
    pub areas: Vec<Area>,
    #[serde(default)]
    pub labels: Vec<Label>,
    #[serde(default)]
    pub schematic: Vec<Schematic>,
}

#[derive(Debug, Deserialize)]
pub struct Maps {
    pub places: HashMap<String, LonLat>,
    pub maps: HashMap<String, MapSpec>,
}

impl Maps {
    pub fn load(assets: &Path) -> Result<Self> {
        let path = assets.join("maps/views.json");
        let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn place(&self, id: &str) -> Result<LonLat> {
        self.places.get(id).copied().with_context(|| format!("unknown map place {id:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot() -> Shot {
        Shot {
            id: "s001".into(),
            kind: "map".into(),
            args: vec![],
            params: HashMap::new(),
            duration: 10.0,
            start_frame: 0,
            duration_in_frames: 300,
            cues: 2,
            cue_times: vec![1.0, 3.0],
            cue_names: HashMap::new(),
        }
    }

    #[test]
    fn resolves_seconds_and_cue_relative_times() {
        let s = shot();
        assert_eq!(When::Seconds(0.4).resolve(&s).unwrap(), 0.4);
        assert_eq!(When::Cue("c1".into()).resolve(&s).unwrap(), 3.0);
        assert!((When::Cue("c0+0.6".into()).resolve(&s).unwrap() - 1.6).abs() < 1e-6);
        assert!((When::Cue("c1-0.4".into()).resolve(&s).unwrap() - 2.6).abs() < 1e-6);
        assert!(When::Cue("later".into()).resolve(&s).is_err());
    }

    #[test]
    fn the_bundled_maps_parse() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        let maps = Maps::load(&assets).unwrap();
        assert!(maps.maps.contains_key("region"));
        assert!(maps.place("amber").is_ok());
    }
}
