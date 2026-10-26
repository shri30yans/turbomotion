//! The plan: the composition after every name has been resolved and every impossible thing rejected, once, with a
//! path to the mistake. Evaluation and rendering never look at a file or a string id again.

mod facing;
mod fonts;
mod fx_inputs;
pub mod geometry;
mod layer;
pub mod layer_space;
mod probe_media;

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde_json::Value;
use turbomotion_core::color::Pipeline;

use crate::{
    anim::prop::ColorProp,
    media::probe::{ImageInfo, VideoInfo},
    schema::{self, EffectSpace, Fx, Kind, Layer, Marker, MatteMode, MotionBlur, PathSource},
};
use crate::fxlib::{Library, PlanFx, PluginPlan};
use geometry::PathGeo;
use layer::{plan_layer, problems_of};

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
    /// Where this composition's layers run their effects and masks (the root's unless the composition says its own).
    pub effect_space: EffectSpace,
    /// What the numbers of its pictures mean, and what its frame leaves in (see `schema::color`).
    pub color: Pipeline,
    /// `check --layout` looks at this composition's layers.
    pub layout: bool,
    pub type_scale: crate::schema::tokens::ResolvedScale,
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
    /// Clip this layer to another layer's alpha without hiding that layer.
    pub clip_to: Option<usize>,
    pub shapes: Vec<PathGeo>,
    pub masks: Vec<PathGeo>,
    pub along: Option<PathGeo>,
    /// GSAP Flip source, if this layer has `"flip": {"from": "id"}`.
    pub flip_from: Option<usize>,
    /// The layer stands in the composition's shared 3D space: it is 3D itself, or one of its parents is.
    /// After Effects sorts the layers of that space by depth; a 2D layer between two of them is a wall
    /// that stops them sorting against each other.
    pub in_3d: bool,
    pub media: Media,
    /// The layer's effect chain, in order.
    pub fx: Vec<PlanFx>,
    /// Its effects and masks run on its own pixels before its transform (`effect_space: layer`, and it has any).
    pub in_layer_space: bool,
    /// A texture layer's generator and parameters.
    pub texture: Option<PluginPlan>,
    /// The path a text layer sets its text on, whether written out or borrowed from another layer's first shape item.
    pub text_path: Option<skia_safe::Path>,
}

/// A layer's media file, probed at plan time.
pub enum Media {
    None,
    Image { path: PathBuf, info: ImageInfo, depth: Option<(PathBuf, ImageInfo)> },
    Video { path: PathBuf, info: VideoInfo },
    /// `rate` and `gain` are 1 for ordinary files; a sound-kit cue that varies (or a note) sets them, and `kit` names the
    /// exact `family/variant` that was chosen.
    Audio { path: PathBuf, dur: f64, rate: f64, gain: f64, kit: Option<String> },
}

impl Plan {
    pub fn load(file: &Path) -> Result<Plan> {
        let text = fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
        let value: Value = serde_json::from_str(&text).with_context(|| format!("{} is not valid JSON", file.display()))?;
        Plan::from_value(value, file.parent().unwrap_or(Path::new(".")))
    }

    pub fn from_value(mut value: Value, dir: &Path) -> Result<Plan> {
        // Resolve once before block expansion, media probing, fonts and shader discovery.
        let root_dir = match value.get("root") {
            None | Some(Value::Null) => dir.to_path_buf(),
            Some(Value::String(root)) => dir.join(root),
            Some(_) => bail!("composition.root must be a directory path string"),
        };
        let dir = root_dir.as_path();
        schema::letterbox::apply_flat(&mut value).context("letterbox")?;
        schema::tokens::publish_palette(&mut value);
        schema::timeline::expand(&mut value).context("timeline")?;
        schema::blocks::expand(&mut value, dir).context("expanding blocks")?;
        schema::repeat::expand(&mut value).context("repeat")?;
        schema::extrude::expand(&mut value).context("extrude")?;
        schema::time::preprocess(&mut value).context("resolving times and variables")?;
        // the library comes first: which fields of an effect are colours is its own declaration, and the tokens need it
        let fx_path: Vec<String> = value.get("fx_path").map_or(Ok(vec![]), |v| serde_json::from_value(v.clone())).context("fx_path is a list of folders")?;
        let fx = Library::load_with(dir, &fx_path)?;
        schema::tokens::apply(&mut value, &|name| fx.colour_params(name)).context("tokens")?;
        if let (Some(keys), Some(given)) = (schema::fields::of::<schema::Composition>(), value.as_object()) {
            if let Some(message) = schema::fields::stray_keys("the composition", given.keys(), keys.keys) {
                bail!("{message}");
            }
        }
        let root: schema::Composition = schema::de::from_value_at(value).context("the composition")?;
        let mut comps = BTreeMap::new();
        for (name, c) in &root.comps {
            ensure!(!name.is_empty(), "a nested composition needs a name");
            comps.insert(name.clone(), CompPlan::build(name, c, &root, dir, &fx).with_context(|| format!("comp {name:?}"))?);
        }
        comps.insert(ROOT.into(), CompPlan::build(ROOT, &root, &root, dir, &fx).context("the root composition")?);
        let plan = Plan { dir: dir.to_path_buf(), fps: root.fps.max(1), comps, fonts: root.fonts.clone(), fx };
        plan.check_references()?;
        fonts::check(&plan)?;
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
        let (kit_order, spread_seed) = kit_spread(name, c);
        let mut problems: Vec<schema::Problem> = Vec::new();
        let effect_space = c.effect_space.or(root.effect_space).unwrap_or_default();
        let color = schema::color::pipeline(name, c.color.as_ref(), root.color.as_ref())?;
        for (i, v) in c.layers.iter().enumerate() {
            match plan_layer(i, v, c, root, dir, lib, kit_order.get(&i).map(|&o| (o, spread_seed)), effect_space) {
                Ok(layer) => layers.push(layer),
                Err(e) => problems.extend(problems_of(i, v, name, e)),
            }
        }
        schema::Problems::finish(problems)?;
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
        for (i, layer) in layers.iter_mut().enumerate() {
            if let Some(p) = &layer.def.common.parent {
                let &pi = ids.get(p).ok_or_else(|| anyhow!("{}: parent {p:?} does not exist; layer ids are {}", layer.label, known()))?;
                ensure!(pi != i, "{}: a layer cannot be its own parent", layer.label);
                layer.parent = Some(pi);
            }
            if let Some(m) = &layer.def.common.matte {
                let &mi = ids.get(&m.source).ok_or_else(|| anyhow!("{}: matte source {:?} does not exist; layer ids are {}", layer.label, m.source, known()))?;
                ensure!(mi != i, "{}: a layer cannot be its own matte", layer.label);
                layer.matte = Some((mi, m.mode));
            }
            if let Some(id) = &layer.def.common.clip_to {
                let &ci = ids.get(id).ok_or_else(|| anyhow!("{}: clip_to {:?} does not exist; layer ids are {}", layer.label, id, known()))?;
                ensure!(ci != i, "{}: a layer cannot clip_to itself", layer.label);
                layer.clip_to = Some(ci);
            }
        }
        for i in 0..layers.len() {
            let Kind::Text(tx) = &layers[i].def.kind else { continue };
            let label = layers[i].label.clone();
            tx.validate().with_context(|| label.clone())?;
            let path = match tx.path.as_ref().map(|p| &p.source) {
                None => None,
                Some(PathSource::Geometry(spec)) => Some(PathGeo::build(spec).context("text path").with_context(|| label.clone())?.path),
                Some(PathSource::Layer(id)) => {
                    let &si = ids.get(id).ok_or_else(|| anyhow!("{label}: text path layer {id:?} does not exist; layer ids are {}", known()))?;
                    let shape = layers[si].shapes.first().ok_or_else(|| anyhow!("{label}: text path layer {id:?} has no shape item to follow"))?;
                    Some(shape.path.clone())
                }
            };
            if let Some(path) = &path {
                crate::text::place::ensure_walkable(path).with_context(|| format!("{label}: text path"))?;
            }
            layers[i].text_path = path;
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
        // the shared 3D space: a layer stands in it when it is 3D itself or when one of its parents is
        for i in 0..layers.len() {
            let mut seen = HashSet::new();
            let mut at = Some(i);
            while let Some(a) = at {
                if !seen.insert(a) {
                    break;
                }
                if layers[a].def.common.three_d {
                    layers[i].in_3d = true;
                    break;
                }
                at = layers[a].parent;
            }
        }
        for i in 0..layers.len() {
            if let Some(f) = &layers[i].def.common.flip {
                let &si = ids.get(&f.from).ok_or_else(|| anyhow!("{}: flip from {:?} does not exist; layer ids are {}", layers[i].label, f.from, known()))?;
                ensure!(si != i, "{}: a layer cannot flip from itself", layers[i].label);
                ensure!(f.dur >= 0.0, "{}: flip dur must not be negative", layers[i].label);
                let mut at = Some(si);
                let mut seen = HashSet::new();
                while let Some(a) = at {
                    ensure!(seen.insert(a), "{}: flip from {:?} loops back", layers[i].label, f.from);
                    ensure!(a != i, "{}: flip from {:?} is this layer's descendant (would recurse)", layers[i].label, f.from);
                    at = layers[a].parent;
                }
                layers[i].flip_from = Some(si);
            }
        }
        {
            let mut seen = HashSet::new();
            for i in 0..layers.len() {
                let mut at = layers[i].flip_from;
                let mut chain = vec![i];
                while let Some(a) = at {
                    ensure!(!chain.contains(&a), "{}: flip from-chain loops", layers[i].label);
                    chain.push(a);
                    at = layers[a].flip_from;
                }
                seen.insert(i);
            }
            let _ = seen;
        }
        for i in 0..layers.len() {
            let Some(st) = layers[i].def.common.stagger.clone() else { continue };
            let kids: Vec<usize> = layers.iter().filter(|l| l.parent == Some(i)).map(|l| l.index).collect();
            let n = kids.len();
            for (k, &ci) in kids.iter().enumerate() {
                let order = crate::text::animate::stagger_order(st.from, k, n, st.seed);
                let delay = st.each * order;
                layers[ci].start += delay;
                layers[ci].end += delay;
            }
        }
        let mut look = c.look.iter().enumerate().map(|(i, f)| Fx::parse(f).and_then(|f| lib.plan_fx(&f)).with_context(|| format!("look {i}"))).collect::<Result<Vec<_>>>()?;
        fx_inputs::resolve(&mut layers, &mut look, &ids, lib)?;
        Ok(CompPlan {
            name: name.to_string(),
            width: c.width,
            height: c.height,
            dur: c.dur,
            bg: c.bg.as_ref().map(|b| b.0.clone()),
            // a nested comp without its own tempo keeps the film's, so beat() works inside shots and blocks
            bpm: c.bpm.or(if name.is_empty() { None } else { root.bpm }),
            beat_offset: if c.bpm.is_none() && !name.is_empty() && root.bpm.is_some() { root.beat_offset } else { c.beat_offset },
            markers: c.markers.clone(),
            mblur: c.mblur,
            effect_space,
            color,
            layout: c.layout,
            look,
            layers,
            type_scale: match &c.type_scale {
                Some(t) => t.resolved(f64::from(c.height)),
                None if !name.is_empty() => root.type_scale.as_ref().cloned().unwrap_or_default().resolved(f64::from(root.height)),
                None => schema::TypeScale::default().resolved(f64::from(c.height)),
            },
        })
    }
}

/// For each `kit:` audio layer in a composition, how many earlier layers (by start time) use the same cue, so a cue
/// that names a family walks through its variants in time order; and a seed that differs between compositions.
fn kit_spread(comp: &str, c: &schema::Composition) -> (HashMap<usize, usize>, u64) {
    let mut by_cue: HashMap<String, Vec<(f64, usize)>> = HashMap::new();
    for (i, v) in c.layers.iter().enumerate() {
        let Some(src) = v.get("src").and_then(Value::as_str).filter(|s| s.starts_with("kit:") && v.get("type").and_then(Value::as_str) == Some("audio")) else { continue };
        if let Ok(def) = schema::parse_layer(i, v) {
            by_cue.entry(src.to_string()).or_default().push((def.common.start, i));
        }
    }
    let mut order = HashMap::new();
    for list in by_cue.values_mut() {
        list.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        order.extend(list.iter().enumerate().map(|(k, (_, i))| (*i, k)));
    }
    (order, comp.bytes().fold(0x9e37_79b9u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn plan(v: Value) -> Result<Plan> {
        Plan::from_value(v, Path::new("."))
    }

    #[test]
    fn nested_comps_inherit_the_films_tempo_unless_they_set_their_own() {
        let p = plan(json!({"dur": 4, "bpm": 120, "beat_offset": 0.25, "comps": {
            "plain": {"dur": 2, "layers": []}, "own": {"dur": 2, "bpm": 90, "layers": []}},
            "layers": [{"type": "precomp", "comp": "plain"}, {"type": "precomp", "comp": "own"}]}))
        .unwrap();
        assert_eq!((p.comps["plain"].bpm, p.comps["plain"].beat_offset), (Some(120.0), 0.25));
        assert_eq!((p.comps["own"].bpm, p.comps["own"].beat_offset), (Some(90.0), 0.0));
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
        assert!(e.contains("unknown key \"colour\". Did you mean \"color\"?") && e.contains("A solid layer takes color, size"), "{e}");
        let e = format!("{:#}", plan(json!({"layers": [{"type": "blob"}]})).err().unwrap());
        assert!(e.contains("solid, shape, text"), "{e}");
    }
}
