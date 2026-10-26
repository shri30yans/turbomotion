//! Reading a plugin's header comments (`// @effect`, `@param`, `@input`, `@pass`, ...) into a [`Header`], and checking
//! that what they declare is consistent before any shader is compiled.

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde_json::Value;
use turbomotion_core::effects;

use super::bounds::Bounds;
use super::{is_full_size, InputDecl, InputKind, Kind, Param, Ty, DEFAULT_DEPTH, FULL_SIZE, MAX_DEPTH, MAX_HISTORY};
use crate::anim::prop::parse_color_static;

/// A `// @pass` marker: the pass it starts, and the line (0-based) it is on.
pub struct PassMark {
    pub name: String,
    pub scale: f64,
    pub doc: String,
    pub line: usize,
}

pub struct Header {
    pub name: String,
    pub kind: Kind,
    pub summary: String,
    pub params: Vec<Param>,
    pub inputs: Vec<InputDecl>,
    pub example: Option<Value>,
    pub marks: Vec<PassMark>,
    /// What `// @bounds` says about the output's reach past its input (see [`Bounds`]).
    pub bounds: Bounds,
}

/// Names a shader cannot declare because the platform does.
const RESERVED: [&str; 6] = ["image", "resolution", "time", "layer_origin", "buffer_size", crate::schema::COMPOSITE_KEY];

fn is_ident(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !s.starts_with(|c: char| c.is_ascii_digit())
}

/// Split `spec -- doc`.
fn split_doc(s: &str) -> (Vec<&str>, String) {
    let (spec, doc) = s.split_once(" -- ").unwrap_or((s, ""));
    (spec.split_whitespace().collect(), doc.trim().to_string())
}

impl Header {
    pub fn parse(src: &str) -> Result<Header> {
        let mut name = None;
        let mut kind = Kind::Effect;
        let mut summary = String::new();
        let mut params = vec![];
        let mut inputs = vec![];
        let mut example = None;
        let mut marks = vec![];
        let mut bounds_tag: Option<(usize, String)> = None;
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
                "input" => inputs.push(parse_input(value).with_context(|| format!("{}: @input {value}", at()))?),
                "pass" => marks.push(parse_pass(value, n).with_context(|| format!("{}: @pass {value}", at()))?),
                "bounds" => {
                    ensure!(bounds_tag.is_none(), "{}: @bounds is given twice", at());
                    bounds_tag = Some((n, value.trim().to_string()));
                }
                "example" => example = Some(serde_json::from_str::<Value>(value).map_err(|e| anyhow!("{}: @example is not JSON: {e}", at()))?),
                other => bail!("{}: unknown header tag @{other}; the tags are @effect, @kind, @summary, @param, @input, @pass, @bounds, @example", at()),
            }
        }
        let name = name.ok_or_else(|| anyhow!("the file needs a header line `// @effect <name>`"))?;
        let bounds = match bounds_tag {
            None => Bounds::Undeclared,
            Some((n, text)) => {
                let layers: Vec<&str> = inputs.iter().filter(|i| matches!(i.kind, InputKind::Layer)).map(|i| i.name.as_str()).collect();
                Bounds::parse(&text, &params, &layers).with_context(|| format!("line {}: @bounds {text}", n + 1))?
            }
        };
        let header = Header { name, kind, summary, params, inputs, example, marks, bounds };
        header.check()?;
        Ok(header)
    }

    fn check(&self) -> Result<()> {
        ensure!(is_ident(&self.name), "@effect name {:?} must be letters, digits and underscores", self.name);
        ensure!(effects::entry(&self.name).is_none(), "{:?} is already a platform effect; pick another name", self.name);
        if self.kind == Kind::Generator {
            ensure!(self.inputs.is_empty(), "a generator paints the whole layer and has no inputs; @input is for effects");
            ensure!(self.marks.len() <= 1, "a generator is a single pass");
        }
        ensure!(self.inputs.iter().filter(|i| matches!(i.kind, InputKind::History { .. })).count() <= 1, "an effect has at most one history input");
        let mut taken: Vec<&str> = RESERVED.to_vec();
        let children: Vec<String> = self.inputs.iter().flat_map(InputDecl::children).collect();
        let named = self.params.iter().map(|p| ("parameter", p.name.as_str())).chain(children.iter().map(|c| ("input", c.as_str()))).chain(self.marks.iter().map(|m| ("pass", m.name.as_str())));
        for (what, n) in named {
            ensure!(!taken.contains(&n), "{what} {n:?} is a reserved or repeated name (image, resolution, time, layer_origin, buffer_size and every parameter, input and pass name are one namespace)");
            taken.push(n);
        }
        if let Some(last) = self.marks.last() {
            ensure!(is_full_size(last.scale), "the last pass {:?} is the effect's output and must be full size (no scale=)", last.name);
        }
        Ok(())
    }
}

/// `name layer [-- doc]` or `name history=N [depth=D] [-- doc]`.
fn parse_input(s: &str) -> Result<InputDecl> {
    let (t, doc) = split_doc(s);
    ensure!(t.len() >= 2, "expected `name layer` or `name history=N [depth=D]`");
    ensure!(is_ident(t[0]), "input name {:?} must be letters, digits and underscores", t[0]);
    let whole = |key: &str, x: &str| x.parse::<u32>().map_err(|_| anyhow!("{key}= takes a whole number, not {x:?}"));
    let kind = if t[1] == "layer" {
        ensure!(t.len() == 2, "`layer` takes no options");
        InputKind::Layer
    } else if let Some(n) = t[1].strip_prefix("history=") {
        let frames = whole("history", n)?;
        ensure!((1..=MAX_HISTORY).contains(&frames), "history={frames} is out of range: history= is 1 to {MAX_HISTORY} frames (each is a recursive render)");
        let mut depth = DEFAULT_DEPTH;
        for extra in &t[2..] {
            let d = extra.strip_prefix("depth=").ok_or_else(|| anyhow!("unknown option {extra:?}; history takes depth=D"))?;
            depth = whole("depth", d)?;
            ensure!((1..=MAX_DEPTH).contains(&depth), "depth={depth} is out of range: depth= is 1 to {MAX_DEPTH}");
        }
        InputKind::History { frames, depth }
    } else {
        bail!("input type {:?}: use `layer` or `history=N`", t[1]);
    };
    Ok(InputDecl { name: t[0].to_string(), kind, doc })
}

/// `name [scale=S] [-- doc]`, on header line `line`.
fn parse_pass(s: &str, line: usize) -> Result<PassMark> {
    let (t, doc) = split_doc(s);
    ensure!(!t.is_empty(), "expected `name [scale=S]`");
    ensure!(is_ident(t[0]), "pass name {:?} must be letters, digits and underscores", t[0]);
    let mut scale = FULL_SIZE;
    for extra in &t[1..] {
        let v = extra.strip_prefix("scale=").ok_or_else(|| anyhow!("unknown option {extra:?}; a pass takes scale=S"))?;
        scale = v.parse::<f64>().map_err(|_| anyhow!("scale= takes a number, not {v:?}"))?;
        ensure!(scale > 0.0 && scale <= FULL_SIZE, "scale={scale} is out of range: scale= is above 0 and at most 1 (the layer's own size)");
    }
    Ok(PassMark { name: t[0].to_string(), scale, doc, line })
}

/// `name type default [min max] [-- doc]`; a `choice` is `name choice default a,b,c`.
fn parse_param(s: &str) -> Result<Param> {
    let (t, doc) = split_doc(s);
    ensure!(t.len() >= 3, "expected `name type default [min max]`");
    let num = |x: &str| x.parse::<f64>().map_err(|_| anyhow!("{x:?} is not a number"));
    let mut options = vec![];
    let (ty, default, rest) = match t[1] {
        "float" => (Ty::Float, vec![num(t[2])?], &t[3..]),
        "vec2" => {
            let (a, b) = t[2].split_once(',').ok_or_else(|| anyhow!("a vec2 default is written x,y without spaces"))?;
            (Ty::Vec2, vec![num(a)?, num(b)?], &t[3..])
        }
        "color" => (Ty::Color, parse_color_static(&Value::String(t[2].to_string()))?.to_vec(), &t[3..]),
        "choice" => {
            ensure!(t.len() == 4, "a choice is written `name choice default option1,option2,...`");
            options = t[3].split(',').map(str::to_string).collect();
            ensure!(options.len() >= 2 && options.iter().all(|o| is_ident(o)), "a choice has two or more options, each letters, digits and underscores");
            let at = options.iter().position(|o| o == t[2]).ok_or_else(|| anyhow!("the default {:?} is not one of the options", t[2]))?;
            (Ty::Choice, vec![at as f64], &t[4..])
        }
        other => bail!("type {other:?}: use float, vec2, color or choice"),
    };
    ensure!(rest.is_empty() || (ty == Ty::Float && rest.len() == 2), "min and max follow a float default, nothing else");
    let (min, max) = if rest.len() == 2 { (Some(num(rest[0])?), Some(num(rest[1])?)) } else { (None, None) };
    Ok(Param { name: t[0].to_string(), ty, default, optional: false, min, max, options, doc })
}
