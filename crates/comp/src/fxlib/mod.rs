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
//! half4 main(float2 xy) { ... }
//! ```
//!
//! Always available: `image` (effects), `resolution` (px), `time` (seconds on the layer's clock), and the library in
//! `prelude.sksl` (hash, noise, fbm, worley, rotations, colour and shape helpers).

use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde_json::{Map, Value};
use turbomotion_core::{effects, shader::Program};

use crate::anim::{
    prop::{parse_color_static, ColorProp, Prop, VecProp},
    suggest,
};

pub mod sweep;

const PRELUDE: &str = include_str!("prelude.sksl");

const BUILTIN: &[(&str, &str)] = &[
    ("duotone", include_str!("builtin/duotone.sksl")),
    ("posterize", include_str!("builtin/posterize.sksl")),
    ("halftone", include_str!("builtin/halftone.sksl")),
    ("kaleidoscope", include_str!("builtin/kaleidoscope.sksl")),
    ("glitch_slices", include_str!("builtin/glitch_slices.sksl")),
    ("vhs", include_str!("builtin/vhs.sksl")),
    ("light_sweep", include_str!("builtin/light_sweep.sksl")),
    ("ripple", include_str!("builtin/ripple.sksl")),
    ("mirror", include_str!("builtin/mirror.sksl")),
    ("noise", include_str!("builtin/noise.sksl")),
    ("clouds", include_str!("builtin/clouds.sksl")),
    ("cells", include_str!("builtin/cells.sksl")),
    ("stripes", include_str!("builtin/stripes.sksl")),
    ("grid", include_str!("builtin/grid.sksl")),
    ("gradient", include_str!("builtin/gradient.sksl")),
    ("light_leak", include_str!("builtin/light_leak.sksl")),
];

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
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Param {
    pub name: String,
    pub ty: Ty,
    /// Float: `[v]`; vec2: `[x, y]`; colour: `[r, g, b, a]` (channels 0..255, alpha 0..1).
    pub default: Vec<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub doc: String,
}

pub struct Def {
    pub name: String,
    pub kind: Kind,
    pub summary: String,
    pub params: Vec<Param>,
    /// `builtin`, or the file it came from.
    pub origin: String,
    pub program: Program,
}

#[derive(Default)]
pub struct Library {
    defs: BTreeMap<String, Def>,
}

/// One entry of an effect chain after planning: a platform effect, or a plugin with its parameters parsed.
#[derive(Debug, Clone)]
pub enum PlanFx {
    Builtin(effects::Effect),
    Plugin(PluginPlan),
}

/// A plugin use after planning: the definition's name and one animatable value per parameter.
#[derive(Debug, Clone)]
pub struct PluginPlan {
    pub name: String,
    pub params: Vec<(String, ParamProp)>,
}

#[derive(Debug, Clone)]
pub enum ParamProp {
    Num(Prop),
    Vec(VecProp),
    Col(ColorProp),
}

impl Library {
    /// The built-in pack plus every `*.sksl` in `<dir>/effects/`. A project file of the same name replaces a built-in.
    pub fn load(dir: &Path) -> Result<Library> {
        let mut lib = Library::default();
        for (name, src) in BUILTIN {
            let def = Def::compile(src, "builtin").with_context(|| format!("built-in effect {name}"))?;
            ensure!(def.name == *name, "built-in file {name} declares @effect {}", def.name);
            lib.defs.insert(def.name.clone(), def);
        }
        let fx_dir = dir.join("effects");
        if fx_dir.is_dir() {
            let mut files: Vec<_> = fs::read_dir(&fx_dir)?.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "sksl")).collect();
            files.sort();
            for f in files {
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
            let hint = suggest(name, &names).map_or(String::new(), |s| format!(" Did you mean {s:?}?"));
            anyhow!("no effect or shader called {name:?}.{hint} Built-in: {}; plugins: add effects/{name}.sksl", effects::catalogue().iter().map(|e| e.effect).collect::<Vec<_>>().join(", "))
        })
    }

    pub fn defs(&self) -> impl Iterator<Item = &Def> {
        self.defs.values()
    }

    pub fn plan_fx(&self, fx: &crate::schema::Fx) -> Result<PlanFx> {
        Ok(match fx {
            crate::schema::Fx::Builtin(e) => PlanFx::Builtin((**e).clone()),
            crate::schema::Fx::Plugin { name, params } => {
                let def = self.get(name)?;
                ensure!(def.kind == Kind::Effect, "{name:?} is a generator: it paints a whole layer (use it as a texture layer), not an effect on one");
                PlanFx::Plugin(self.plan(name, params)?)
            }
        })
    }

    /// Check a use against its definition and parse each parameter (defaults fill the ones left out).
    pub fn plan(&self, name: &str, given: &Map<String, Value>) -> Result<PluginPlan> {
        let def = self.get(name)?;
        let names: Vec<&str> = def.params.iter().map(|p| p.name.as_str()).collect();
        for k in given.keys() {
            if !names.contains(&k.as_str()) {
                let hint = suggest(k, &names).map_or(String::new(), |s| format!(" Did you mean {s:?}?"));
                bail!("effect {name:?} has no parameter {k:?}.{hint} Its parameters are: {}", if names.is_empty() { "none".to_string() } else { names.join(", ") });
            }
        }
        let mut params = Vec::with_capacity(def.params.len());
        for p in &def.params {
            let prop = match given.get(&p.name) {
                None => match p.ty {
                    Ty::Float => ParamProp::Num(Prop::constant(p.default[0])),
                    Ty::Vec2 => ParamProp::Vec(VecProp::constant(&p.default)),
                    Ty::Color => ParamProp::Col(ColorProp::constant([p.default[0], p.default[1], p.default[2], p.default[3]])),
                },
                Some(v) => (|| -> Result<ParamProp> {
                    Ok(match p.ty {
                        Ty::Float => ParamProp::Num(Prop::parse(v)?),
                        Ty::Vec2 => ParamProp::Vec(VecProp::parse(v, 2..=2)?),
                        Ty::Color => ParamProp::Col(ColorProp::parse(v)?),
                    })
                })()
                .with_context(|| format!("effect {name:?} parameter {:?}", p.name))?,
            };
            params.push((p.name.clone(), prop));
        }
        Ok(PluginPlan { name: name.to_string(), params })
    }
}

impl Def {
    /// Parse the header, wrap the body in the generated prelude, compile, and check the header against the code.
    pub fn compile(src: &str, origin: &str) -> Result<Def> {
        let mut name = None;
        let mut kind = Kind::Effect;
        let mut summary = String::new();
        let mut params: Vec<Param> = vec![];
        for (n, line) in src.lines().enumerate() {
            let Some(rest) = line.trim().strip_prefix("//") else { continue };
            let Some(tag) = rest.trim().strip_prefix('@') else { continue };
            let (key, value) = tag.split_once(char::is_whitespace).unwrap_or((tag, ""));
            let at = || format!("line {}", n + 1);
            match key {
                "effect" => name = Some(value.trim().to_string()),
                "kind" => {
                    kind = match value.trim() {
                        "effect" => Kind::Effect,
                        "generator" => Kind::Generator,
                        other => bail!("{}: @kind is effect or generator, not {other:?}", at()),
                    }
                }
                "summary" => summary = value.trim().to_string(),
                "param" => params.push(parse_param(value).with_context(|| format!("{}: @param {value}", at()))?),
                other => bail!("{}: unknown header tag @{other}; the tags are @effect, @kind, @summary, @param", at()),
            }
        }
        let name = name.ok_or_else(|| anyhow!("the file needs a header line `// @effect <name>`"))?;
        ensure!(!name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'), "@effect name {name:?} must be letters, digits and underscores");
        ensure!(!effects::catalogue().iter().any(|e| e.effect == name), "{name:?} is already a platform effect; pick another name");
        let mut seen = vec![];
        for p in &params {
            ensure!(!["image", "resolution", "time"].contains(&p.name.as_str()), "parameter {:?} is a reserved name", p.name);
            ensure!(!seen.contains(&&p.name), "parameter {:?} is declared twice", p.name);
            seen.push(&p.name);
        }

        let mut head = String::new();
        if kind == Kind::Effect {
            head.push_str("uniform shader image;\n");
        }
        head.push_str("uniform float2 resolution;\nuniform float time;\n");
        for p in &params {
            let ty = match p.ty {
                Ty::Float => "float",
                Ty::Vec2 => "float2",
                Ty::Color => "float4",
            };
            head.push_str(&format!("uniform {ty} {};\n", p.name));
        }
        // Generators have no input layer, so the standard library's image helpers are not offered to them.
        let lib = if kind == Kind::Effect { PRELUDE.to_string() } else { PRELUDE.split("// The input layer at a point").next().unwrap_or(PRELUDE).to_string() };
        let prelude = format!("{head}{lib}\n");
        let offset = prelude.lines().count();
        let program = Program::shader(&name, &format!("{prelude}{src}")).map_err(|e| anyhow!("{}", explain(&format!("{e:#}"), src, offset)))?;
        let declared = program.uniform_names();
        for u in ["resolution", "time"].iter().map(|s| s.to_string()).chain(params.iter().map(|p| p.name.clone())) {
            ensure!(declared.contains(&u), "uniform {u:?} is unused by the shader, so the compiler dropped it; use it or remove it from the header");
        }
        Ok(Def { name, kind, summary, params, origin: origin.to_string(), program })
    }
}

fn parse_param(s: &str) -> Result<Param> {
    let (spec, doc) = s.split_once(" -- ").unwrap_or((s, ""));
    let t: Vec<&str> = spec.split_whitespace().collect();
    ensure!(t.len() >= 3, "expected `name type default [min max]`");
    let num = |x: &str| x.parse::<f64>().map_err(|_| anyhow!("{x:?} is not a number"));
    let (ty, default, rest) = match t[1] {
        "float" => (Ty::Float, vec![num(t[2])?], &t[3..]),
        "vec2" => {
            let (a, b) = t[2].split_once(',').ok_or_else(|| anyhow!("a vec2 default is written x,y without spaces"))?;
            (Ty::Vec2, vec![num(a)?, num(b)?], &t[3..])
        }
        "color" => (Ty::Color, parse_color_static(&Value::String(t[2].to_string()))?.to_vec(), &t[3..]),
        other => bail!("type {other:?}: use float, vec2 or color"),
    };
    ensure!(rest.is_empty() || (ty == Ty::Float && rest.len() == 2), "min and max follow a float default, nothing else");
    let (min, max) = if rest.len() == 2 { (Some(num(rest[0])?), Some(num(rest[1])?)) } else { (None, None) };
    Ok(Param { name: t[0].to_string(), ty, default, min, max, doc: doc.trim().to_string() })
}

/// Skia reports `error: <line>: ...` against the generated source; point at the author's line and show it.
fn explain(msg: &str, body: &str, offset: usize) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut out = String::new();
    for l in msg.lines() {
        let t = l.trim_start();
        let t = t.find("error: ").map_or(t, |i| &t[i..]);
        if let Some(rest) = t.strip_prefix("error: ") {
            if let Some((n, text)) = rest.split_once(": ") {
                if let Ok(n) = n.trim().parse::<usize>() {
                    // lines inside the generated prelude have no author line; show them as they are
                    let n = if n > offset { n - offset } else { 0 };
                    if n == 0 {
                        out.push_str(&format!("(generated prelude) {text}\n"));
                        continue;
                    }
                    out.push_str(&format!("line {n}: {text}\n"));
                    if let Some(src) = lines.get(n.wrapping_sub(1)) {
                        out.push_str(&format!("    | {}\n", src.trim_end()));
                    }
                    continue;
                }
            }
        }
        out.push_str(l);
        out.push('\n');
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_whole_built_in_pack_compiles() {
        let lib = Library::load(Path::new("/nonexistent")).unwrap();
        assert!(lib.defs().count() >= 16);
        for d in lib.defs() {
            assert!(!d.summary.is_empty(), "{} needs a @summary", d.name);
        }
    }

    #[test]
    fn errors_point_at_the_authors_line() {
        let src = "// @effect broken\n// @param amount float 1\nhalf4 main(float2 xy) {\n    return half4(amount + nothing_here);\n}\n";
        let e = Def::compile(src, "t").err().unwrap().to_string();
        assert!(e.contains("line 4") && e.contains("nothing_here"), "{e}");
    }

    #[test]
    fn headers_are_checked() {
        assert!(Def::compile("half4 main(float2 xy) { return half4(1); }", "t").is_err());
        let e = Def::compile("// @effect x\n// @param a floot 1\nhalf4 main(float2 xy) { return half4(1); }", "t").err().unwrap();
        assert!(format!("{e:#}").contains("float, vec2 or color"), "{e:#}");
        let e = Def::compile("// @effect blur\nhalf4 main(float2 xy) { return half4(time + resolution.x); }", "t").err().unwrap();
        assert!(e.to_string().contains("already a platform effect"), "{e}");
    }

    #[test]
    fn uses_are_checked_against_the_definition() {
        let lib = Library::load(Path::new("/nonexistent")).unwrap();
        let ok = lib.plan("halftone", serde_json::json!({"size": 10}).as_object().unwrap()).unwrap();
        assert_eq!(ok.params.len(), lib.get("halftone").unwrap().params.len());
        let e = lib.plan("halftone", serde_json::json!({"sise": 10}).as_object().unwrap()).err().unwrap().to_string();
        assert!(e.contains("Did you mean \"size\""), "{e}");
        let e = lib.get("halftoen").err().unwrap().to_string();
        assert!(e.contains("Did you mean \"halftone\""), "{e}");
    }

    #[test]
    fn a_composition_uses_builtin_and_project_plugins_in_one_chain() {
        use crate::{eval::{fx::FxState, Content, Evaluator}, plan::{Plan, ROOT}};
        let dir = std::env::temp_dir().join(format!("tm-fx-{}", std::process::id()));
        fs::create_dir_all(dir.join("effects")).unwrap();
        fs::write(dir.join("effects/tint2.sksl"), "// @effect tint2\n// @param amount float 0.5 0 1\nhalf4 main(float2 xy) { return image.eval(xy) * amount; }\n").unwrap();
        let v = serde_json::json!({"dur": 2, "layers": [
            {"type": "solid", "color": "#ffffff", "fx": [{"effect": "blur", "radius": 2}, {"effect": "tint2", "amount": {"from": 0, "to": 2, "dur": 2, "ease": "none"}}]},
            {"type": "texture", "shader": "noise", "params": {"scale": 50}}
        ]});
        let plan = Plan::from_value(v, &dir).unwrap();
        let f = Evaluator::new(&plan).frame(ROOT, 1.0).unwrap();
        assert!(matches!(f.layers[0].fx[0], FxState::Builtin));
        let FxState::Plugin(p) = &f.layers[0].fx[1] else { panic!("second effect is the plugin") };
        // 0 -> 2 over two seconds is 1.0 at t=1, then clamped to the declared 0..1 range
        assert_eq!(p.values[0], ("amount".to_string(), vec![1.0]));
        assert!(matches!(f.layers[1].content, Content::Texture { .. }));
        let bad = serde_json::json!({"layers": [{"type": "solid", "color": "#fff", "fx": [{"effect": "tint2", "amout": 1}]}]});
        let e = format!("{:#}", Plan::from_value(bad, &dir).err().unwrap());
        assert!(e.contains("Did you mean \"amount\""), "{e}");
        let wrong = serde_json::json!({"layers": [{"type": "texture", "shader": "duotone"}]});
        assert!(format!("{:#}", Plan::from_value(wrong, &dir).err().unwrap()).contains("needs an input layer"));
        let _ = fs::remove_dir_all(&dir);
    }
}
