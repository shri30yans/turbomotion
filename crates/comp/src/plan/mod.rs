//! The plan: the composition after every name has been resolved and every impossible thing rejected, once, with a
//! path to the mistake. Evaluation and rendering never look at a file or a string id again.

pub mod geometry;

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde_json::Value;

use crate::{
    anim::prop::ColorProp,
    media::probe::{self, ImageInfo, VideoInfo},
    schema::{self, Fx, Kind, Layer, Marker, MatteMode, MotionBlur},
};
use crate::fxlib::{Kind as FxKind, Library, PlanFx, PluginPlan};
use geometry::PathGeo;

pub struct Plan {
    /// Where relative paths (`src`, fonts, plugins) are resolved from.
    pub dir: PathBuf,
    pub fps: u32,
    pub comps: BTreeMap<String, CompPlan>,
    /// Font roles declared by the file.
    pub fonts: BTreeMap<String, String>,
    /// Built-in and project shader plugins.
    pub fx: Library,
}

/// The root composition is stored under this name.
pub const ROOT: &str = "";

pub struct CompPlan {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub dur: f64,
    pub bg: Option<ColorProp>,
    pub bpm: Option<f64>,
    pub beat_offset: f64,
    pub markers: Vec<Marker>,
    pub mblur: Option<MotionBlur>,
    /// Effects over the finished picture.
    pub look: Vec<PlanFx>,
    pub layers: Vec<LayerPlan>,
}

pub struct LayerPlan {
    pub index: usize,
    pub label: String,
    pub def: Layer,
    /// Absolute composition seconds.
    pub start: f64,
    pub end: f64,
    pub parent: Option<usize>,
    pub matte: Option<(usize, MatteMode)>,
    /// Used as someone's matte: not drawn by itself.
    pub is_matte_source: bool,
    pub shapes: Vec<PathGeo>,
    pub masks: Vec<PathGeo>,
    pub along: Option<PathGeo>,
    pub media: Media,
    /// The layer's effect chain, in order.
    pub fx: Vec<PlanFx>,
    /// A texture layer's generator and parameters.
    pub texture: Option<PluginPlan>,
}

/// A layer's media file, probed at plan time.
pub enum Media {
    None,
    Image { path: PathBuf, info: ImageInfo, depth: Option<(PathBuf, ImageInfo)> },
    Video { path: PathBuf, info: VideoInfo },
    Audio { path: PathBuf, dur: f64 },
}

impl Plan {
    pub fn load(file: &Path) -> Result<Plan> {
        let text = fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
        let value: Value = serde_json::from_str(&text).with_context(|| format!("{} is not valid JSON", file.display()))?;
        Plan::from_value(value, file.parent().unwrap_or(Path::new(".")))
    }

    pub fn from_value(mut value: Value, dir: &Path) -> Result<Plan> {
        schema::time::preprocess(&mut value).context("resolving times and variables")?;
        let root: schema::Composition = serde_json::from_value(value).map_err(|e| anyhow!("{e}")).context("the composition")?;
        let fx = Library::load(dir)?;
        let mut comps = BTreeMap::new();
        for (name, c) in &root.comps {
            ensure!(!name.is_empty(), "a nested composition needs a name");
            comps.insert(name.clone(), CompPlan::build(name, c, &root, dir, &fx).with_context(|| format!("comp {name:?}"))?);
        }
        comps.insert(ROOT.into(), CompPlan::build(ROOT, &root, &root, dir, &fx).context("the root composition")?);
        let plan = Plan { dir: dir.to_path_buf(), fps: root.fps.max(1), comps, fonts: root.fonts.clone(), fx };
        plan.check_references()?;
        Ok(plan)
    }

    pub fn root(&self) -> &CompPlan {
        &self.comps[ROOT]
    }

    pub fn frames(&self) -> u32 {
        (self.root().dur * f64::from(self.fps)).round().max(1.0) as u32
    }

    pub fn resolve(&self, rel: &str) -> PathBuf {
        let p = Path::new(rel);
        if p.is_absolute() { p.to_path_buf() } else { self.dir.join(p) }
    }

    /// Names that need the whole plan to check: nested compositions exist and do not contain themselves.
    fn check_references(&self) -> Result<()> {
        for (name, comp) in &self.comps {
            for l in &comp.layers {
                if let Kind::Precomp(p) = &l.def.kind {
                    ensure!(
                        self.comps.contains_key(&p.comp) && !p.comp.is_empty(),
                        "{}: {}: there is no comp {:?}; the comps are {}",
                        if name.is_empty() { "root" } else { name },
                        l.label,
                        p.comp,
                        if self.comps.len() <= 1 { "none (add them under \"comps\")".to_string() } else { self.comps.keys().filter(|k| !k.is_empty()).cloned().collect::<Vec<_>>().join(", ") }
                    );
                }
            }
        }
        // cycle: a comp reaching itself through precomps
        fn reaches(plan: &Plan, from: &str, seen: &mut Vec<String>) -> Result<()> {
            if let Some(i) = seen.iter().position(|s| s == from) {
                bail!("compositions contain each other in a loop: {}", seen[i..].iter().chain([&from.to_string()]).map(|s| if s.is_empty() { "root".to_string() } else { s.clone() }).collect::<Vec<_>>().join(" -> "));
            }
            seen.push(from.to_string());
            for l in &plan.comps[from].layers {
                if let Kind::Precomp(p) = &l.def.kind {
                    reaches(plan, &p.comp, seen)?;
                }
            }
            seen.pop();
            Ok(())
        }
        reaches(self, ROOT, &mut vec![])
    }
}

impl CompPlan {
    fn build(name: &str, c: &schema::Composition, root: &schema::Composition, dir: &Path, lib: &Library) -> Result<CompPlan> {
        ensure!(c.width > 0 && c.height > 0, "width and height must be positive");
        ensure!(c.dur > 0.0, "dur (seconds) must be positive");
        let mut layers = Vec::with_capacity(c.layers.len());
        for (i, v) in c.layers.iter().enumerate() {
            let def = schema::parse_layer(i, v)?;
            let ty = v.get("type").and_then(Value::as_str).unwrap_or("?");
            let label = format!("layer {i} ({ty}{})", def.common.id.as_deref().map_or(String::new(), |id| format!(" {id:?}")));
            let start = def.common.start;
            let media = probe_media(&def.kind, dir).with_context(|| label.clone())?;
            let natural = match (&media, &def.kind) {
                (Media::Video { info, .. }, Kind::Video(v)) if !v.looping => Some((info.dur - v.src_in).max(0.0) / v.speed.max(1e-6)),
                (Media::Audio { dur, .. }, Kind::Audio(a)) => Some((dur - a.src_in).max(0.0) / a.speed.max(1e-6)),
                _ => None,
            };
            let end = match (def.common.dur, &def.kind) {
                (Some(d), _) => {
                    ensure!(d >= 0.0, "{label}: dur must not be negative");
                    start + d
                }
                (None, _) if natural.is_some() => (start + natural.unwrap_or(0.0)).min(c.dur.max(start)),
                (None, Kind::Precomp(p)) => root.comps.get(&p.comp).map_or(c.dur, |n| (start + (n.dur - p.offset) / p.speed.max(1e-6)).min(c.dur.max(start))),
                (None, _) => c.dur,
            };
            let shapes = match &def.kind {
                Kind::Shape(s) => s.items.iter().enumerate().map(|(k, it)| PathGeo::build(&it.path).with_context(|| format!("item {k}"))).collect::<Result<_>>().with_context(|| label.clone())?,
                _ => vec![],
            };
            let masks = def.common.masks.iter().enumerate().map(|(k, m)| PathGeo::build(&m.path).with_context(|| format!("mask {k}"))).collect::<Result<_>>().with_context(|| label.clone())?;
            let along = def.common.along.as_ref().map(|a| PathGeo::build(&a.path)).transpose().context("along").with_context(|| label.clone())?;
            let fx = def.fx.iter().enumerate().map(|(k, f)| lib.plan_fx(f).with_context(|| format!("fx {k}"))).collect::<Result<Vec<_>>>().with_context(|| label.clone())?;
            let texture = match &def.kind {
                Kind::Texture(t) => {
                    let plan = lib.plan(&t.shader, &t.params).with_context(|| label.clone())?;
                    ensure!(lib.get(&t.shader)?.kind == FxKind::Generator, "{label}: {:?} is an effect that needs an input layer; a texture layer takes a generator", t.shader);
                    Some(plan)
                }
                _ => None,
            };
            layers.push(LayerPlan { index: i, label, def, start, end, parent: None, matte: None, is_matte_source: false, shapes, masks, along, media, fx, texture });
        }
        // names → indices
        let mut ids: HashMap<String, usize> = HashMap::new();
        for l in &layers {
            if let Some(id) = &l.def.common.id {
                ensure!(ids.insert(id.clone(), l.index).is_none(), "two layers are called {id:?}");
            }
        }
        let known = || {
            let mut v: Vec<_> = ids.keys().cloned().collect();
            v.sort();
            if v.is_empty() { "none (give layers an \"id\")".to_string() } else { v.join(", ") }
        };
        for i in 0..layers.len() {
            if let Some(p) = &layers[i].def.common.parent {
                let &pi = ids.get(p).ok_or_else(|| anyhow!("{}: parent {p:?} does not exist; layer ids are {}", layers[i].label, known()))?;
                ensure!(pi != i, "{}: a layer cannot be its own parent", layers[i].label);
                layers[i].parent = Some(pi);
            }
            if let Some(m) = &layers[i].def.common.matte {
                let &mi = ids.get(&m.source).ok_or_else(|| anyhow!("{}: matte source {:?} does not exist; layer ids are {}", layers[i].label, m.source, known()))?;
                ensure!(mi != i, "{}: a layer cannot be its own matte", layers[i].label);
                layers[i].matte = Some((mi, m.mode));
            }
        }
        for i in 0..layers.len() {
            if let Some((m, _)) = layers[i].matte {
                layers[m].is_matte_source = true;
            }
        }
        // parent cycles
        for i in 0..layers.len() {
            let mut seen = HashSet::new();
            let mut at = Some(i);
            while let Some(a) = at {
                ensure!(seen.insert(a), "{}: parents form a loop", layers[i].label);
                at = layers[a].parent;
            }
        }
        Ok(CompPlan {
            name: name.to_string(),
            width: c.width,
            height: c.height,
            dur: c.dur,
            bg: c.bg.as_ref().map(|b| b.0.clone()),
            bpm: c.bpm,
            beat_offset: c.beat_offset,
            markers: c.markers.clone(),
            mblur: c.mblur,
            look: c.look.iter().enumerate().map(|(i, f)| Fx::parse(f).and_then(|f| lib.plan_fx(&f)).with_context(|| format!("look {i}"))).collect::<Result<_>>()?,
            layers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn plan(v: Value) -> Result<Plan> {
        Plan::from_value(v, Path::new("."))
    }

    #[test]
    fn a_small_composition_resolves_names() {
        let p = plan(json!({"dur": 4, "comps": {"inner": {"dur": 2, "layers": []}}, "layers": [
            {"type": "solid", "id": "bg", "color": "#102030"},
            {"type": "null", "id": "rig"},
            {"type": "shape", "id": "dot", "parent": "rig", "items": [{"ellipse": [0, 0, 20, 20], "fill": "#fff"}]},
            {"type": "precomp", "comp": "inner", "start": 1, "matte": {"source": "dot"}}
        ]}))
        .unwrap();
        let c = p.root();
        assert_eq!(c.layers[2].parent, Some(1));
        assert_eq!(c.layers[3].matte.map(|m| m.0), Some(2));
        assert!(c.layers[2].is_matte_source);
        assert_eq!(c.layers[3].end, 3.0);
        assert_eq!(p.frames(), 120);
    }

    #[test]
    fn unresolvable_references_name_the_candidates() {
        let e = format!("{:#}", plan(json!({"layers": [{"type": "null", "id": "a"}, {"type": "null", "parent": "b"}]})).err().unwrap());
        assert!(e.contains("parent \"b\" does not exist") && e.contains("a"), "{e}");
        let e = format!("{:#}", plan(json!({"layers": [{"type": "precomp", "comp": "nope"}]})).err().unwrap());
        assert!(e.contains("no comp"), "{e}");
        let e = format!("{:#}", plan(json!({"layers": [{"type": "null", "id": "a", "parent": "b"}, {"type": "null", "id": "b", "parent": "a"}]})).err().unwrap());
        assert!(e.contains("loop"), "{e}");
        let e = format!("{:#}", plan(json!({"comps": {"a": {"layers": [{"type": "precomp", "comp": "a"}]}}, "layers": [{"type": "precomp", "comp": "a"}]})).err().unwrap());
        assert!(e.contains("each other"), "{e}");
    }

    #[test]
    fn schema_mistakes_show_the_valid_names() {
        let e = format!("{:#}", plan(json!({"layers": [{"type": "solid", "colour": "#fff"}]})).err().unwrap());
        assert!(e.contains("unknown field"), "{e}");
        let e = format!("{:#}", plan(json!({"layers": [{"type": "blob"}]})).err().unwrap());
        assert!(e.contains("solid, shape, text"), "{e}");
    }
}

fn probe_media(kind: &Kind, dir: &Path) -> Result<Media> {
    Ok(match kind {
        Kind::Image(i) => {
            let path = probe::absolute(dir, &i.src);
            let info = probe::image(&path, &i.src)?;
            let depth = i.parallax.as_ref().map(|p| -> Result<_> {
                let dp = probe::absolute(dir, &p.depth);
                let di = probe::image(&dp, &p.depth).context("parallax depth map")?;
                Ok((dp, di))
            });
            Media::Image { path, info, depth: depth.transpose()? }
        }
        Kind::Video(v) => {
            let path = probe::absolute(dir, &v.src);
            let info = probe::video(&path, &v.src)?;
            Media::Video { path, info }
        }
        Kind::Audio(a) => {
            let path = probe::absolute(dir, &a.src);
            let dur = probe::audio(&path, &a.src)?;
            Media::Audio { path, dur }
        }
        _ => Media::None,
    })
}
