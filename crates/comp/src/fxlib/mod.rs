//! Shader plugins: new pixels without a rebuild. A plugin is one `.sksl` file whose leading comments declare what it
//! is and what knobs it has; the platform writes the uniform declarations and the standard library around it, so the
//! author writes only `half4 main(float2 xy)`. Mistakes are reported against the author's own line numbers.
//!
//! ```text
//! // @effect duotone                          (name used in comp.json: {"effect": "duotone", ...})
//! // @kind effect                             (effect: reads `image`; generator: paints the whole layer)
//! // @summary Maps brightness onto two colours.
//! // @param shadow color #101030 -- colour for the darks
//! // @param amount float 1 0 1 -- 0 = original, 1 = fully mapped     (default, then optional min and max)
//! // @input map layer -- another layer whose picture steers the effect (a displacement map, a matte)
//! // @input prev history=1 depth=8 -- the effect's own output one frame ago (feedback, trails, datamosh)
//! // @pass blur_h scale=0.5 -- a named pass; a later pass samples it as `uniform shader blur_h`
//! // @bounds expand=blur*3                   (layer space: pixels the output reaches past the input on each side; or `none`)
//! // @space linear                           (display, the default, or linear: the encoding the shader computes in; see below)
//! // @example {"effect": "duotone", "amount": 1}     (the doc example; one is generated from the defaults if absent)
//! half4 main(float2 xy) { ... }
//! ```
//!
//! Always available: `image` (effects), `resolution` (px), `time` (seconds on the layer's clock), `layer_origin` and
//! `buffer_size` (see below), and the library in `prelude.sksl` (hash, noise, fbm, worley, rotations, colour and shape helpers).
//!
//! One declaration describes an effect completely: see [`Descriptor`]. Rust built-ins (the `registry!` in
//! `turbomotion-core`) and plugins both produce one, and every generator of documentation reads only descriptors.
//!
//! * `@input name layer` adds a shader child `name`: the picture of another layer, picked in the effect's own
//!   parameters by id (`"map": "noise_layer"` or `{"layer": "noise_layer", "fit": "stretch"}`), rendered at the same
//!   moment. `fit` is `center` (the map keeps its size, centred on the effect's layer), `stretch` (the map's box is
//!   scaled to the effect layer's box), as in After Effects' Displacement Map, or `screen` (the map stays where it is).
//! * `@input name history=N [depth=D]` adds shader children `name`, `name2` .. `nameN`: the effect's own output at
//!   `t-1/fps` .. `t-N/fps`, each defined by recursion `D` levels deep (level 0 is the effect's input), so a frame is
//!   still a pure function of time and seeking gives the pixels playing from the start would give.
//! * In layer space (`effect_space: layer`) the picture an effect gets is the layer's pixels grown by the chain's
//!   `@bounds`, and `xy` stays in *layer pixels*: `resolution` is the layer's own content size (so `xy / resolution` is
//!   0..1 over the content whatever the other effects grow), `layer_origin` is where the picture's top-left corner is in
//!   layer pixels (negative when it grew up or left) and `buffer_size` is the picture's size. In screen space
//!   `layer_origin` is 0 and `buffer_size` is `resolution`.
//! * `@space linear` says the shader works on light: `image`, the inputs and colour parameters reach it as light, and what
//!   it returns is taken as light, however the composition's working profile is encoded (see `color` in `schema::color`).
//!   The default, `display`, is what the author sees on screen: 0.5 is mid-grey. In a film that is not in linear light
//!   both are the same numbers.
//! * `@pass name [scale=S]` starts a pass. Passes run in order; each sees `image`, the inputs and every earlier pass by
//!   name; the last one (scale 1) is the effect's output. Text before the first `@pass` is shared by all passes. A pass
//!   reads a bigger picture bilinearly with no mipmaps, so it skips pixels rather than averaging them: `scale=0.5` is
//!   fine, much smaller aliases fine detail (halve in steps, one pass per halving).

use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde_json::{Map, Value};
use turbomotion_core::{color::Encoding, effects, shader::Program, suggest::hint};

pub use bounds::{Bounds, Expr};
use crate::anim::prop::{ColorProp, Prop, VecProp};

mod bounds;
mod compile;
pub mod composite;
mod header;
pub mod search;
pub mod sweep;

include!(concat!(env!("OUT_DIR"), "/builtin_plugins.rs"));

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Effect,
    Generator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ty {
    Float,
    Vec2,
    Color,
    /// One of a fixed list of names; the shader receives the index as a float.
    Choice,
}

/// How a layer input is laid over the layer the effect is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    /// The map keeps its size, its centre on the effect layer's centre.
    Center,
    /// The map's box is scaled to the effect layer's box.
    Stretch,
    /// The map stays where it is on screen and the effect layer looks through it: what a matte wants.
    Screen,
}

/// What an extra input is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InputKind {
    /// Another layer's picture, named by id in the effect's parameters.
    Layer,
    /// The effect's own output `1..=frames` frames ago, computed by recursion `depth` levels deep.
    History { frames: u32, depth: u32 },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InputDecl {
    pub name: String,
    #[serde(flatten)]
    pub kind: InputKind,
    pub doc: String,
}

/// The `origin` of a platform effect's descriptor: it is Rust, not a plugin file.
pub const PLATFORM_ORIGIN: &str = "rust";
/// A colour's alpha when its default is written as `r, g, b`.
const OPAQUE: f64 = 1.0;

/// Most frames of history an effect may ask for, and deepest recursion: both multiply render work.
pub const MAX_HISTORY: u32 = 8;
pub const MAX_DEPTH: u32 = 32;
pub const DEFAULT_DEPTH: u32 = 8;

/// Whether a pass `scale` is the layer's own size.
pub fn is_full_size(scale: f64) -> bool {
    (scale - FULL_SIZE).abs() < f64::EPSILON
}

/// A pass at this scale is as big as the layer.
pub const FULL_SIZE: f64 = 1.0;

/// One render pass of a plugin: its own program, at `scale` of the layer's size.
pub struct Pass {
    pub name: String,
    pub scale: f64,
    pub doc: String,
    pub program: Program,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PassInfo {
    pub name: String,
    pub scale: f64,
    pub doc: String,
}

/// The complete, serialisable declaration of one effect: typed parameters, named inputs, passes, history, defaults and
/// a runnable example. Built-ins and plugins both produce one; documentation is written from descriptors alone.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Descriptor {
    pub name: String,
    pub kind: Kind,
    /// [`PLATFORM_ORIGIN`] for a platform effect, `builtin` for a shipped plugin, else the plugin file.
    pub origin: String,
    pub summary: String,
    pub params: Vec<Param>,
    pub inputs: Vec<InputDecl>,
    pub passes: Vec<PassInfo>,
    /// The most frames of own-output history any input asks for (0: the effect is stateless).
    pub history: u32,
    pub example: Value,
    /// How far the output reaches past its input, for layer space: `none`, `grows` (a platform effect), `expand=...`
    /// (a plugin's tag) or `undeclared` (a plugin without `@bounds`).
    pub bounds: String,
    /// The encoding it computes in, `display` or `linear` (see `core::color::Encoding`).
    pub space: String,
}

impl Param {
    /// The default as it is written in a use of the effect: a number, `[x, y]`, a `#rrggbb` colour or an option's name.
    pub fn default_json(&self) -> Value {
        match self.ty {
            Ty::Float => serde_json::json!(self.default[0]),
            Ty::Vec2 => serde_json::json!([self.default[0], self.default[1]]),
            Ty::Color => serde_json::json!(format!("#{:02x}{:02x}{:02x}", self.default[0] as u8, self.default[1] as u8, self.default[2] as u8)),
            Ty::Choice => serde_json::json!(self.options[self.default[0] as usize]),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Param {
    pub name: String,
    pub ty: Ty,
    /// Float: `[v]`; vec2: `[x, y]`; colour: `[r, g, b, a]` (channels 0..255, alpha 0..1); empty for an optional parameter.
    pub default: Vec<f64>,
    /// Left out, the parameter is simply absent (only a platform effect's colour can be).
    pub optional: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// A choice's names, in index order.
    pub options: Vec<String>,
    pub doc: String,
}

pub struct Def {
    pub name: String,
    pub kind: Kind,
    pub summary: String,
    pub params: Vec<Param>,
    pub inputs: Vec<InputDecl>,
    /// `builtin`, or the file it came from.
    pub origin: String,
    pub passes: Vec<Pass>,
    pub example: Option<Value>,
    /// How far the output reaches past the input (`// @bounds`).
    pub bounds: Bounds,
    /// The encoding the shader computes in (`// @space`).
    pub space: Encoding,
}

#[derive(Default)]
pub struct Library {
    defs: BTreeMap<String, Def>,
    /// Kit plugins that did not compile, as `(file, error)`. They do not stop a film that does not use them.
    pub broken: Vec<(String, String)>,
}

/// One entry of an effect chain after planning: what it does, and how its result is laid over its input.
#[derive(Debug, Clone)]
pub struct PlanFx {
    pub op: FxOp,
    pub composite: Option<composite::CompositePlan>,
}

/// A reference from an effect to another layer: what it is for, the id as written, and where the plan keeps the index.
pub struct LayerRef<'a> {
    pub what: String,
    pub id: &'a str,
    pub index: &'a mut Option<usize>,
}

impl PlanFx {
    pub fn name(&self) -> &str {
        match &self.op {
            FxOp::Builtin(e) => e.name(),
            FxOp::Plugin(p) => &p.name,
        }
    }

    /// Every layer this effect reads: a plugin's layer inputs and a layer mask.
    pub fn layer_refs_mut(&mut self) -> Vec<LayerRef<'_>> {
        let mut refs: Vec<LayerRef> = match &mut self.op {
            FxOp::Builtin(_) => vec![],
            FxOp::Plugin(p) => p.inputs.iter_mut().map(|i| LayerRef { what: format!("input {:?}", i.name), id: &i.id, index: &mut i.index }).collect(),
        };
        if let Some(composite::CompositePlan { mask: composite::MaskPlan::Layer { id, index, .. }, .. }) = &mut self.composite {
            refs.push(LayerRef { what: "composite mask".to_string(), id, index });
        }
        refs
    }

    /// The layers the effect's own shader samples, once the plan has resolved them.
    pub fn inputs_read(&self) -> Vec<usize> {
        match &self.op {
            FxOp::Builtin(_) => vec![],
            FxOp::Plugin(p) => p.inputs.iter().filter_map(|i| i.index).collect(),
        }
    }

    /// Every layer this effect reads, once the plan has resolved them: its inputs and its composite mask.
    pub fn layers_read(&self) -> Vec<usize> {
        let inputs = self.inputs_read();
        let mask = match &self.composite {
            Some(composite::CompositePlan { mask: composite::MaskPlan::Layer { index, .. }, .. }) => *index,
            _ => None,
        };
        inputs.into_iter().chain(mask).collect()
    }
}

/// A platform effect, or a plugin with its parameters parsed.
#[derive(Debug, Clone)]
pub enum FxOp {
    Builtin(effects::Effect),
    Plugin(PluginPlan),
}

/// A plugin use after planning: the definition's name and one animatable value per parameter.
#[derive(Debug, Clone)]
pub struct PluginPlan {
    pub name: String,
    pub params: Vec<(String, ParamProp)>,
    /// One entry per layer input the definition declares (history inputs need no plan).
    pub inputs: Vec<InputPlan>,
}

/// A layer input of one effect use: which layer, and how it is fitted. `index` is filled in when the composition's layer
/// ids are known.
#[derive(Debug, Clone)]
pub struct InputPlan {
    pub name: String,
    pub id: String,
    pub fit: Fit,
    pub index: Option<usize>,
}

// a parameter list holds a handful of these, so the size gap between variants costs nothing
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum ParamProp {
    Num(Prop),
    Vec(VecProp),
    Col(ColorProp),
    /// The index into the parameter's options. A choice cannot be animated: there is nothing between two names.
    Choice(usize),
}

impl Library {
    /// Only the shipped plugins: what every composition has before any folder adds to it.
    pub fn builtin() -> Result<Library> {
        let mut lib = Library::default();
        for (name, src) in BUILTIN_PLUGINS {
            let def = Def::compile(src, "builtin").with_context(|| format!("built-in effect {name}"))?;
            ensure!(def.name == *name, "built-in file {name} declares @effect {}", def.name);
            lib.defs.insert(def.name.clone(), def);
        }
        Ok(lib)
    }

    /// The built-in pack, the kit and any include folders (see [`search`]), then every `*.sksl` in `<dir>/effects/`.
    /// A project file of the same name replaces an earlier one.
    pub fn load(dir: &Path) -> Result<Library> {
        Library::load_with(dir, &[])
    }

    /// As [`Library::load`] with the composition's own `fx_path` folders.
    pub fn load_with(dir: &Path, declared: &[String]) -> Result<Library> {
        let mut lib = Library::builtin()?;
        for folder in search::include_dirs(dir, declared)? {
            let mut from_folder: BTreeMap<String, String> = BTreeMap::new();
            for f in search::plugin_files(&folder)? {
                let origin = f.display().to_string();
                let compiled = fs::read_to_string(&f).with_context(|| format!("reading {origin}")).and_then(|src| Def::compile(&src, &origin).with_context(|| origin.clone()));
                match compiled {
                    Ok(def) => {
                        if let Some(first) = from_folder.insert(def.name.clone(), origin.clone()) {
                            lib.broken.push((origin.clone(), format!("@effect {:?} is already defined by {first}; rename one", def.name)));
                            continue;
                        }
                        lib.defs.insert(def.name.clone(), def);
                    }
                    Err(e) => lib.broken.push((origin, format!("{e:#}"))),
                }
            }
        }
        let fx_dir = dir.join("effects");
        if fx_dir.is_dir() {
            for f in search::plugin_files(&fx_dir)? {
                let src = fs::read_to_string(&f).with_context(|| format!("reading {}", f.display()))?;
                let origin = f.strip_prefix(dir).unwrap_or(&f).display().to_string();
                let def = Def::compile(&src, &origin).with_context(|| origin.clone())?;
                lib.defs.insert(def.name.clone(), def);
            }
        }
        Ok(lib)
    }

    pub fn get(&self, name: &str) -> Result<&Def> {
        self.defs.get(name).ok_or_else(|| {
            let names: Vec<&str> = self.defs.keys().map(String::as_str).collect();
            let hint = hint(name, &names);
            let broken = if self.broken.is_empty() { String::new() } else { format!(" Plugins that failed to compile: {}.", self.broken.iter().map(|(f, e)| format!("{f}: {}", e.lines().next().unwrap_or(""))).collect::<Vec<_>>().join("; ")) };
            anyhow!("no effect or shader called {name:?}.{hint} Built-in: {}; plugins: add effects/{name}.sksl or put it in the kit (kit/effects/**.sksl; `comp fx list` shows every plugin and where it came from).{broken}", effects::CATALOGUE.iter().map(|e| e.effect).collect::<Vec<_>>().join(", "))
        })
    }

    /// The names of the parameters of effect or generator `name` that are colours; none when nothing has that name.
    pub fn colour_params(&self, name: &str) -> Option<Vec<String>> {
        match effects::entry(name) {
            Some(e) => Some(e.params.iter().filter(|p| matches!(p.kind, effects::ParamKind::Color { .. })).map(|p| p.name.to_string()).collect()),
            None => self.defs.get(name).map(|d| d.params.iter().filter(|p| p.ty == Ty::Color).map(|p| p.name.clone()).collect()),
        }
    }

    pub fn defs(&self) -> impl Iterator<Item = &Def> {
        self.defs.values()
    }

    pub fn plan_fx(&self, fx: &crate::schema::Fx) -> Result<PlanFx> {
        let op = match &fx.op {
            crate::schema::FxOp::Builtin(e) => FxOp::Builtin((**e).clone()),
            crate::schema::FxOp::Plugin { name, params } => {
                let def = self.get(name)?;
                ensure!(def.kind == Kind::Effect, "{name:?} is a generator: it paints a whole layer (use it as a texture layer), not an effect on one");
                FxOp::Plugin(self.plan(name, params)?)
            }
        };
        let composite = fx.composite.as_ref().map(composite::parse).transpose().context("composite")?;
        Ok(PlanFx { op, composite })
    }

    /// Check a use against its definition and parse each parameter (defaults fill the ones left out).
    pub fn plan(&self, name: &str, given: &Map<String, Value>) -> Result<PluginPlan> {
        let def = self.get(name)?;
        let layer_inputs: Vec<&InputDecl> = def.inputs.iter().filter(|i| i.kind == InputKind::Layer).collect();
        let names: Vec<&str> = def.params.iter().map(|p| p.name.as_str()).chain(layer_inputs.iter().map(|i| i.name.as_str())).collect();
        effects::check_names(name, given.keys(), &names)?;
        let mut params = Vec::with_capacity(def.params.len());
        for p in &def.params {
            let prop = match given.get(&p.name) {
                None => match p.ty {
                    Ty::Float => ParamProp::Num(Prop::constant(p.default[0])),
                    Ty::Vec2 => ParamProp::Vec(VecProp::constant(&p.default)),
                    Ty::Color => ParamProp::Col(ColorProp::constant([p.default[0], p.default[1], p.default[2], p.default[3]])),
                    Ty::Choice => ParamProp::Choice(p.default[0] as usize),
                },
                Some(v) => (|| -> Result<ParamProp> {
                    Ok(match p.ty {
                        Ty::Float => ParamProp::Num(Prop::parse(v)?),
                        Ty::Vec2 => ParamProp::Vec(VecProp::parse(v, 2..=2)?),
                        Ty::Color => ParamProp::Col(ColorProp::parse(v)?),
                        Ty::Choice => ParamProp::Choice(parse_choice(&p.options, v)?),
                    })
                })()
                .with_context(|| format!("effect {name:?} parameter {:?}", p.name))?,
            };
            params.push((p.name.clone(), prop));
        }
        let mut inputs = Vec::with_capacity(layer_inputs.len());
        for i in layer_inputs {
            let v = given.get(&i.name).ok_or_else(|| anyhow!("effect {name:?} needs {:?}: the id of the layer whose picture it reads (\"{}\": \"some_layer\"){}", i.name, i.name, if i.doc.is_empty() { String::new() } else { format!(" -- {}", i.doc) }))?;
            inputs.push(parse_input_use(&i.name, v).with_context(|| format!("effect {name:?} input {:?}", i.name))?);
        }
        Ok(PluginPlan { name: name.to_string(), params, inputs })
    }

    /// One descriptor per effect: the platform's own, then every plugin.
    pub fn descriptors(&self) -> Vec<Descriptor> {
        let mut out: Vec<Descriptor> = effects::CATALOGUE.iter().map(builtin_descriptor).collect();
        out.extend(self.defs.values().map(Def::descriptor));
        out
    }
}

/// A choice is written as one of its option names.
fn parse_choice(options: &[String], v: &Value) -> Result<usize> {
    let name = v.as_str().ok_or_else(|| anyhow!("a choice is one of its names: {}", options.join(", ")))?;
    options.iter().position(|o| o == name).ok_or_else(|| {
        let names: Vec<&str> = options.iter().map(String::as_str).collect();
        anyhow!("{name:?} is not an option.{} The options are: {}", hint(name, &names), options.join(", "))
    })
}

/// `"layer_id"` or `{"layer": "layer_id", "fit": "center" | "stretch"}`.
fn parse_input_use(name: &str, v: &Value) -> Result<InputPlan> {
    let (id, fit) = match v {
        Value::String(id) => (id.clone(), Fit::Center),
        Value::Object(o) => {
            for k in o.keys() {
                ensure!(k == "layer" || k == "fit", "unknown key {k:?}; an input is {{\"layer\": id, \"fit\": \"center\" | \"stretch\" | \"screen\"}}");
            }
            let id = o.get("layer").and_then(Value::as_str).ok_or_else(|| anyhow!("an input object needs \"layer\": <layer id>"))?.to_string();
            let fit = match o.get("fit") {
                None => Fit::Center,
                Some(f) => match f.as_str() {
                    Some("center") => Fit::Center,
                    Some("stretch") => Fit::Stretch,
                    Some("screen") => Fit::Screen,
                    _ => bail!("fit {f} is not an option; the fits are \"center\", \"stretch\" and \"screen\""),
                },
            };
            (id, fit)
        }
        _ => bail!("an input is a layer id (\"map\": \"noise_layer\") or {{\"layer\": id, \"fit\": \"center\" | \"stretch\" | \"screen\"}}"),
    };
    ensure!(!id.is_empty(), "the layer id is empty");
    Ok(InputPlan { name: name.to_string(), id, fit, index: None })
}

/// A platform effect's descriptor, written from its registry entry: parameters, defaults, ranges and docs are what the
/// effect itself is built from.
fn builtin_descriptor(e: &effects::Entry) -> Descriptor {
    let params = e
        .params
        .iter()
        .map(|p| {
            let (ty, default) = match p.kind {
                effects::ParamKind::Float { default } => (Ty::Float, vec![default]),
                effects::ParamKind::Color { default } => (Ty::Color, default.map_or(vec![], |[r, g, b]| vec![f64::from(r), f64::from(g), f64::from(b), OPAQUE])),
            };
            Param { name: p.name.to_string(), ty, optional: default.is_empty(), default, min: p.min, max: p.max, options: vec![], doc: p.doc.to_string() }
        })
        .collect();
    Descriptor { name: e.effect.to_string(), kind: Kind::Effect, origin: PLATFORM_ORIGIN.to_string(), summary: e.summary.to_string(), params, inputs: vec![], passes: vec![PassInfo { name: "main".into(), scale: 1.0, doc: String::new() }], history: 0, example: (e.example)(), bounds: (if effects::grows_bounds(e.effect) { "grows" } else { "none" }).to_string(), space: e.encoding.name().to_string() }
}

impl InputDecl {
    /// The shader children this input declares: `prev`, `prev2` .. `prevN` for history, else its own name.
    pub fn children(&self) -> Vec<String> {
        match self.kind {
            InputKind::Layer => vec![self.name.clone()],
            InputKind::History { frames, .. } => (1..=frames).map(|k| if k == 1 { self.name.clone() } else { format!("{}{k}", self.name) }).collect(),
        }
    }
}

impl Def {
    /// The whole declaration as data, with an example generated from the defaults when the header gave none.
    pub fn descriptor(&self) -> Descriptor {
        let example = self.example.clone().unwrap_or_else(|| self.default_example());
        let history = self.inputs.iter().filter_map(|i| if let InputKind::History { frames, .. } = i.kind { Some(frames) } else { None }).max().unwrap_or(0);
        Descriptor {
            name: self.name.clone(),
            kind: self.kind,
            origin: self.origin.clone(),
            summary: self.summary.clone(),
            params: self.params.clone(),
            inputs: self.inputs.clone(),
            passes: self.passes.iter().map(|p| PassInfo { name: p.name.clone(), scale: p.scale, doc: p.doc.clone() }).collect(),
            history,
            example,
            bounds: self.bounds.describe(),
            space: self.space.name().to_string(),
        }
    }

    /// A use of the effect with every parameter at its default and a placeholder for each layer input.
    fn default_example(&self) -> Value {
        let mut o = Map::new();
        if self.kind == Kind::Effect {
            o.insert("effect".to_string(), Value::String(self.name.clone()));
        } else {
            o.insert("type".to_string(), Value::String("texture".to_string()));
            o.insert("shader".to_string(), Value::String(self.name.clone()));
        }
        let mut params = Map::new();
        for p in &self.params {
            params.insert(p.name.clone(), p.default_json());
        }
        for i in self.inputs.iter().filter(|i| i.kind == InputKind::Layer) {
            params.insert(i.name.clone(), Value::String("layer_id".to_string()));
        }
        if self.kind == Kind::Effect {
            o.extend(params);
        } else {
            o.insert("params".to_string(), Value::Object(params));
        }
        Value::Object(o)
    }

    /// The history input, if the effect has one.
    pub fn history(&self) -> Option<(&InputDecl, u32, u32)> {
        self.inputs.iter().find_map(|i| if let InputKind::History { frames, depth } = i.kind { Some((i, frames, depth)) } else { None })
    }
}

#[cfg(test)]
mod tests;
