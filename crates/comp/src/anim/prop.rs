//! Animated properties. Every number a composition animates is a `Prop`; vectors and colours are small bundles of
//! them. A property is any of:
//!
//! ```text
//! 40                                                  a constant
//! "wiggle(2, 30)"                                     an expression (text)
//! [[0, 0], [0.6, 100, "expo.out"], [2, 100]]          keyframes [seconds, value, ease?]; the ease shapes the move INTO that key
//! {"from": 0, "to": 100, "at": 1, "dur": 0.6, "ease": "back.out(1.4)"}     a GSAP-style tween
//! {"keys": [[0, 0], [1, 360]], "loop": "cycle"}       keys that repeat: cycle | pingpong | offset
//! {"value": [[0, 0], [1, 100]], "expr": "value + wiggle(3, 5)"}             keys plus an expression on top
//! ```
//!
//! Before the first key a property holds its first value, after the last it holds its last (unless it loops).

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde_json::Value;

use super::{ease::Ease, expr::{Arg, Env, Expr}};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key {
    pub t: f64,
    pub v: f64,
    /// The ease of the segment that ends at this key.
    pub ease: Ease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loop {
    None,
    Cycle,
    PingPong,
    Offset,
}

#[derive(Debug, Clone)]
pub struct Prop {
    keys: Vec<Key>,
    looping: Loop,
    expr: Option<Expr>,
}

impl Prop {
    pub fn constant(v: f64) -> Prop {
        Prop { keys: vec![Key { t: 0.0, v, ease: Ease::Linear }], looping: Loop::None, expr: None }
    }

    pub fn is_constant(&self) -> bool {
        self.keys.len() == 1 && self.expr.is_none()
    }

    pub fn keys(&self) -> &[Key] {
        &self.keys
    }

    pub fn expr(&self) -> Option<&Expr> {
        self.expr.as_ref()
    }

    /// Plain keys: no expression and no loop, so the value is exactly what `base` says at any time.
    pub fn is_plain(&self) -> bool {
        self.expr.is_none() && self.looping == Loop::None
    }

    /// The keyframed value at `t`, before any expression.
    pub fn base(&self, t: f64) -> f64 {
        let (first, last) = (self.keys[0], self.keys[self.keys.len() - 1]);
        if self.keys.len() == 1 {
            return first.v;
        }
        let span = last.t - first.t;
        let (mut t, mut extra) = (t, 0.0);
        if t > last.t && span > 0.0 {
            let n = ((t - first.t) / span).floor();
            match self.looping {
                Loop::None => {}
                Loop::Cycle => t = first.t + (t - first.t).rem_euclid(span),
                Loop::Offset => {
                    t = first.t + (t - first.t).rem_euclid(span);
                    extra = n * (last.v - first.v);
                }
                Loop::PingPong => {
                    let m = (t - first.t).rem_euclid(2.0 * span);
                    t = first.t + if m > span { 2.0 * span - m } else { m };
                }
            }
        }
        if t <= first.t {
            return first.v + extra;
        }
        match self.keys.windows(2).find(|w| t < w[1].t) {
            None => last.v + extra,
            Some(w) => {
                let (a, b) = (w[0], w[1]);
                let u = (t - a.t) / (b.t - a.t).max(1e-9);
                a.v + (b.v - a.v) * b.ease.at(u) + extra
            }
        }
    }

    /// The value at `t` with the expression applied. `env` supplies `time`, `index`, and the host functions.
    pub fn at(&self, t: f64, env: &dyn Env) -> Result<f64> {
        let base = self.base(t);
        match &self.expr {
            None => Ok(base),
            Some(e) => e.eval(&WithValue { inner: env, value: base }),
        }
    }

    pub fn parse(v: &Value) -> Result<Prop> {
        match v {
            Value::Number(n) => Ok(Prop::constant(n.as_f64().ok_or_else(|| anyhow!("{n} is not a finite number"))?)),
            Value::String(s) => Ok(Prop { keys: vec![Key { t: 0.0, v: 0.0, ease: Ease::Linear }], looping: Loop::None, expr: Some(Expr::parse(s)?) }),
            Value::Array(_) => Ok(Prop { keys: parse_keys(v)?, looping: Loop::None, expr: None }),
            Value::Object(o) => {
                const VALID: &[&str] = &["keys", "value", "from", "to", "at", "dur", "delay", "ease", "loop", "expr"];
                if let Some(bad) = o.keys().find(|k| !VALID.contains(&k.as_str())) {
                    bail!("unknown property field {bad:?}; valid fields are {}", VALID.join(", "));
                }
                let keys = if let Some(k) = o.get("keys") {
                    parse_keys(k)?
                } else if let Some(v) = o.get("value") {
                    let inner = Prop::parse(v)?;
                    ensure!(inner.expr.is_none(), "\"value\" is the base the expression builds on; it cannot have its own expression");
                    inner.keys
                } else if o.contains_key("from") || o.contains_key("to") {
                    let (from, to) = (o.get("from"), o.get("to"));
                    let (from, to) = (num(from.ok_or_else(|| anyhow!("a tween needs \"from\" and \"to\""))?)?, num(to.ok_or_else(|| anyhow!("a tween needs \"from\" and \"to\""))?)?);
                    let at = o.get("at").map_or(Ok(0.0), num)? + o.get("delay").map_or(Ok(0.0), num)?;
                    let dur = o.get("dur").map_or(Ok(0.5), num)?;
                    ensure!(dur >= 0.0, "\"dur\" must not be negative");
                    let ease = o.get("ease").map_or(Ok(Ease::default()), |e| Ease::parse(e.as_str().ok_or_else(|| anyhow!("\"ease\" must be text like \"expo.out\""))?))?;
                    vec![Key { t: at, v: from, ease: Ease::Linear }, Key { t: at + dur.max(1e-6), v: to, ease }]
                } else if o.contains_key("expr") {
                    vec![Key { t: 0.0, v: 0.0, ease: Ease::Linear }]
                } else {
                    bail!("a property object needs \"keys\", \"value\", a \"from\"/\"to\" tween, or an \"expr\"");
                };
                let looping = match o.get("loop").map(|l| l.as_str().unwrap_or("")) {
                    None => Loop::None,
                    Some("cycle") => Loop::Cycle,
                    Some("pingpong") => Loop::PingPong,
                    Some("offset") => Loop::Offset,
                    Some(other) => bail!("\"loop\" must be cycle, pingpong or offset, not {other:?}"),
                };
                let expr = o.get("expr").map(|e| Expr::parse(e.as_str().ok_or_else(|| anyhow!("\"expr\" must be text"))?)).transpose()?;
                Ok(Prop { keys, looping, expr })
            }
            other => bail!("a property is a number, text (an expression), keys or a tween, not {other}"),
        }
    }
}

fn num(v: &Value) -> Result<f64> {
    v.as_f64().ok_or_else(|| anyhow!("expected a number, found {v}"))
}

fn parse_keys(v: &Value) -> Result<Vec<Key>> {
    let list = v.as_array().ok_or_else(|| anyhow!("keys must be a list like [[0, 0], [1, 100]]"))?;
    ensure!(!list.is_empty(), "keys must not be empty");
    let mut keys: Vec<Key> = Vec::with_capacity(list.len());
    for (i, item) in list.iter().enumerate() {
        let key = (|| -> Result<Key> {
            match item {
                Value::Array(a) => {
                    ensure!((2..=3).contains(&a.len()), "a key is [seconds, value] or [seconds, value, ease]");
                    ensure!(!a[1].is_array(), "a key's value here is one number, not {}. A vector animates as {VECTOR_FORMS}", a[1]);
                    let ease = a.get(2).map_or(Ok(Ease::default()), |e| Ease::parse(e.as_str().ok_or_else(|| anyhow!("the ease of a key is text like \"expo.out\""))?))?;
                    Ok(Key { t: num(&a[0])?, v: num(&a[1])?, ease })
                }
                Value::Object(o) => {
                    if !o.contains_key("t") && ["from", "to", "dur", "at"].iter().any(|k| o.contains_key(*k)) {
                        bail!("keys are [seconds, value, ease?] (or {{\"t\", \"v\"}}); this looks like a tween object, and a list of tweens is not keys. Use one tween per property ({{\"from\": 0, \"to\": 1, \"at\": 0.5, \"dur\": 0.4}}), or keys such as [[0, 0], [0.5, 1, \"expo.out\"]]");
                    }
                    let ease = o.get("ease").map_or(Ok(Ease::default()), |e| Ease::parse(e.as_str().unwrap_or("")))?;
                    Ok(Key { t: num(o.get("t").ok_or_else(|| anyhow!("key needs \"t\""))?)?, v: num(o.get("v").ok_or_else(|| anyhow!("key needs \"v\""))?)?, ease })
                }
                other => bail!("a key is [seconds, value, ease?], not {other}"),
            }
        })()
        .with_context(|| format!("key {i}"))?;
        if let Some(prev) = keys.last() {
            ensure!(key.t >= prev.t, "key {i} is at {}s but the key before it is at {}s: keys must be in time order", key.t, prev.t);
        }
        keys.push(key);
    }
    Ok(keys)
}

struct WithValue<'a> {
    inner: &'a dyn Env,
    value: f64,
}

impl Env for WithValue<'_> {
    fn var(&self, name: &str) -> Option<f64> {
        if name == "value" { Some(self.value) } else { self.inner.var(name) }
    }
    fn call(&self, name: &str, args: &[Arg]) -> Option<Result<f64>> {
        self.inner.call(name, args)
    }
    fn salt(&self) -> u32 {
        self.inner.salt()
    }
}

/// The ways to write a vector, for errors.
const VECTOR_FORMS: &str = "[x, y] (each part any property: [{\"from\": 1, \"to\": 2, \"dur\": 1}, 0.5]), {\"from\": [0, 0], \"to\": [100, 50], \"at\": 1, \"dur\": 0.5} or {\"keys\": [[0, [0, 0]], [1, [100, 50], \"expo.out\"]]}";

/// `[[t, [x, y]], ..]` written without the `{"keys": ..}` wrapper.
fn looks_like_vector_keys(parts: &[Value]) -> bool {
    !parts.is_empty() && parts.iter().all(|p| p.as_array().is_some_and(|k| k.len() >= 2 && k[0].is_number() && k[1].is_array()))
}

/// Two or three properties moving together: positions, scales, anchors.
#[derive(Debug, Clone)]
pub struct VecProp(pub Vec<Prop>);

impl VecProp {
    pub fn constant(v: &[f64]) -> VecProp {
        VecProp(v.iter().map(|x| Prop::constant(*x)).collect())
    }

    /// `[x, y]` where each part is any property, or a tween / keys of whole vectors:
    /// `{"from": [0, 0], "to": [100, 50], "at": 1, "dur": 0.5}` and `{"keys": [[0, [0, 0]], [1, [100, 50], "expo.out"]]}`.
    pub fn parse(v: &Value, len: std::ops::RangeInclusive<usize>) -> Result<VecProp> {
        let what = || format!("{}-{} numbers", len.start(), len.end());
        match v {
            Value::Array(parts) => {
                ensure!(!looks_like_vector_keys(parts), "keys of vectors need the {{\"keys\"}} wrapper. A vector is {VECTOR_FORMS}; found {v}");
                ensure!(len.contains(&parts.len()), "expected {} as {VECTOR_FORMS}; found {} items", what(), parts.len());
                Ok(VecProp(parts.iter().enumerate().map(|(i, p)| Prop::parse(p).with_context(|| format!("component {i}"))).collect::<Result<_>>()?))
            }
            Value::Object(o) => {
                if let Some(keys) = o.get("keys") {
                    let list = keys.as_array().ok_or_else(|| anyhow!("keys must be a list"))?;
                    let n = list.first().and_then(|k| k.as_array()).and_then(|k| k.get(1)).and_then(Value::as_array).map(Vec::len).ok_or_else(|| anyhow!("vector keys look like [seconds, [x, y], ease?] inside {{\"keys\": ..}}; a vector is {VECTOR_FORMS}"))?;
                    ensure!(len.contains(&n), "expected {}, found {n}", what());
                    let mut comps = vec![];
                    for c in 0..n {
                        let part: Vec<Value> = list
                            .iter()
                            .map(|k| {
                                let k = k.as_array().ok_or_else(|| anyhow!("vector keys look like [seconds, [x, y], ease?]"))?;
                                let comp = k.get(1).and_then(Value::as_array).and_then(|a| a.get(c)).ok_or_else(|| anyhow!("every key needs {n} numbers"))?;
                                let mut out = vec![k[0].clone(), comp.clone()];
                                if let Some(e) = k.get(2) {
                                    out.push(e.clone());
                                }
                                Ok(Value::Array(out))
                            })
                            .collect::<Result<_>>()?;
                        let mut obj = serde_json::Map::new();
                        obj.insert("keys".into(), Value::Array(part));
                        if let Some(l) = o.get("loop") {
                            obj.insert("loop".into(), l.clone());
                        }
                        comps.push(Prop::parse(&Value::Object(obj))?);
                    }
                    Ok(VecProp(comps))
                } else if let (Some(from), Some(to)) = (o.get("from").and_then(Value::as_array), o.get("to").and_then(Value::as_array)) {
                    ensure!(from.len() == to.len() && len.contains(&from.len()), "\"from\" and \"to\" must both have {}", what());
                    let mut comps = vec![];
                    for (f, t) in from.iter().zip(to) {
                        let mut obj = o.clone();
                        obj.insert("from".into(), f.clone());
                        obj.insert("to".into(), t.clone());
                        comps.push(Prop::parse(&Value::Object(obj))?);
                    }
                    Ok(VecProp(comps))
                } else {
                    bail!("a vector is {VECTOR_FORMS}; found {v}")
                }
            }
            other => bail!("expected a vector ({}), found {other}. A vector is {VECTOR_FORMS}", what()),
        }
    }

    pub fn at(&self, t: f64, env: &dyn Env) -> Result<Vec<f64>> {
        self.0.iter().map(|p| p.at(t, env)).collect()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// An RGBA colour that can animate: channels are 0..255, alpha 0..1.
#[derive(Debug, Clone)]
pub struct ColorProp {
    pub r: Prop,
    pub g: Prop,
    pub b: Prop,
    pub a: Prop,
}

pub type Rgba = [f32; 4];

pub fn parse_color_static(v: &Value) -> Result<[f64; 4]> {
    match v {
        Value::String(s) => hex(s),
        Value::Array(a) if a.len() == 3 || a.len() == 4 => {
            let n: Vec<f64> = a.iter().map(num).collect::<Result<_>>()?;
            Ok([n[0], n[1], n[2], n.get(3).copied().unwrap_or(1.0)])
        }
        other => bail!("a colour is \"#rrggbb\", \"#rrggbbaa\", a name like \"white\", or [r, g, b, a?] (0-255, alpha 0-1); found {other}"),
    }
}

/// The key list of a colour animation, in either spelling: `[[t, c], ..]` or `{"keys": [[t, c], ..]}`.
fn colour_keys(v: &Value) -> Option<&Vec<Value>> {
    let list = match v {
        Value::Array(l) => l,
        Value::Object(o) => o.get("keys")?.as_array()?,
        _ => return None,
    };
    list.first().is_some_and(Value::is_array).then_some(list)
}

fn hex(s: &str) -> Result<[f64; 4]> {
    let named = match s {
        "white" => Some([255.0, 255.0, 255.0, 1.0]),
        "black" => Some([0.0, 0.0, 0.0, 1.0]),
        "transparent" | "none" => Some([0.0, 0.0, 0.0, 0.0]),
        "red" => Some([232.0, 64.0, 47.0, 1.0]),
        "green" => Some([62.0, 182.0, 150.0, 1.0]),
        "blue" => Some([60.0, 120.0, 255.0, 1.0]),
        "yellow" => Some([255.0, 214.0, 10.0, 1.0]),
        "gold" => Some([222.0, 176.0, 82.0, 1.0]),
        _ => None,
    };
    if let Some(c) = named {
        return Ok(c);
    }
    let h = s.strip_prefix('#').ok_or_else(|| anyhow!("colour {s:?} should start with # (like \"#ff8800\") or be a name like \"white\""))?;
    let d = |i: usize| u8::from_str_radix(&h[i..i + 1].repeat(2), 16);
    let b = |i: usize| u8::from_str_radix(&h[i..i + 2], 16);
    let bad = |_| anyhow!("colour {s:?} has non-hex digits");
    let c = match h.len() {
        3 => [d(0).map_err(bad)?, d(1).map_err(bad)?, d(2).map_err(bad)?, 255],
        4 => [d(0).map_err(bad)?, d(1).map_err(bad)?, d(2).map_err(bad)?, d(3).map_err(bad)?],
        6 => [b(0).map_err(bad)?, b(2).map_err(bad)?, b(4).map_err(bad)?, 255],
        8 => [b(0).map_err(bad)?, b(2).map_err(bad)?, b(4).map_err(bad)?, b(6).map_err(bad)?],
        _ => bail!("colour {s:?} should have 3, 4, 6 or 8 hex digits"),
    };
    Ok([f64::from(c[0]), f64::from(c[1]), f64::from(c[2]), f64::from(c[3]) / 255.0])
}

impl ColorProp {
    pub fn constant(c: [f64; 4]) -> ColorProp {
        ColorProp { r: Prop::constant(c[0]), g: Prop::constant(c[1]), b: Prop::constant(c[2]), a: Prop::constant(c[3]) }
    }

    /// A colour, a tween `{"from": "#fff", "to": "#f00", "at", "dur", "ease"}`, keys `[[t, "#fff", ease?], ..]`, or
    /// `{"keys": [[t, "#fff", ease?], ..], "loop": "cycle"}`.
    pub fn parse(v: &Value) -> Result<ColorProp> {
        let looping = v.get("loop").cloned();
        let split = |keys: Vec<(f64, [f64; 4], Value)>| -> Result<ColorProp> {
            let mk = |c: usize| -> Result<Prop> {
                let list: Vec<Value> = keys
                    .iter()
                    .map(|(t, col, ease)| {
                        let mut k = vec![Value::from(*t), Value::from(col[c])];
                        if !ease.is_null() {
                            k.push(ease.clone());
                        }
                        Value::Array(k)
                    })
                    .collect();
                match &looping {
                    None => Prop::parse(&Value::Array(list)),
                    Some(l) => Prop::parse(&serde_json::json!({"keys": list, "loop": l})),
                }
            };
            Ok(ColorProp { r: mk(0)?, g: mk(1)?, b: mk(2)?, a: mk(3)? })
        };
        match v {
            Value::Object(o) if o.contains_key("channels") => {
                ensure!(o.len() == 1, "colour channels cannot be combined with other fields");
                let channels = o["channels"].as_array().ok_or_else(|| anyhow!("colour channels must be [r,g,b,a] properties"))?;
                ensure!(channels.len() == 4, "colour channels must have four properties: [r,g,b,a]");
                Ok(ColorProp { r: Prop::parse(&channels[0])?, g: Prop::parse(&channels[1])?, b: Prop::parse(&channels[2])?, a: Prop::parse(&channels[3])? })
            }
            Value::Object(o) if o.contains_key("from") => {
                let from = parse_color_static(&o["from"])?;
                let to = parse_color_static(o.get("to").ok_or_else(|| anyhow!("a colour tween needs \"to\""))?)?;
                let at = o.get("at").map_or(Ok(0.0), num)? + o.get("delay").map_or(Ok(0.0), num)?;
                let dur = o.get("dur").map_or(Ok(0.5), num)?;
                let ease = o.get("ease").cloned().unwrap_or(Value::Null);
                split(vec![(at, from, Value::Null), (at + dur.max(1e-6), to, ease)])
            }
            Value::Array(_) | Value::Object(_) if colour_keys(v).is_some() => {
                let list = colour_keys(v).expect("checked in the guard");
                let keys = list
                    .iter()
                    .map(|k| {
                        let k = k.as_array().ok_or_else(|| anyhow!("colour keys are [seconds, colour, ease?]"))?;
                        ensure!(k.len() >= 2, "colour keys are [seconds, colour, ease?]");
                        Ok((num(&k[0])?, parse_color_static(&k[1])?, k.get(2).cloned().unwrap_or(Value::Null)))
                    })
                    .collect::<Result<Vec<_>>>()?;
                split(keys)
            }
            Value::Object(_) => bail!("a colour animation is {{\"from\": c, \"to\": c, \"at\", \"dur\", \"ease\"}} or {{\"keys\": [[seconds, colour, ease?], ..]}}; found {v}"),
            other => Ok(ColorProp::constant(parse_color_static(other)?)),
        }
    }

    pub fn at(&self, t: f64, env: &dyn Env) -> Result<Rgba> {
        Ok([
            self.r.at(t, env)?.clamp(0.0, 255.0) as f32,
            self.g.at(t, env)?.clamp(0.0, 255.0) as f32,
            self.b.at(t, env)?.clamp(0.0, 255.0) as f32,
            self.a.at(t, env)?.clamp(0.0, 1.0) as f32,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct E;
    impl Env for E {
        fn var(&self, n: &str) -> Option<f64> {
            if n == "time" { Some(0.5) } else { None }
        }
        fn call(&self, _: &str, _: &[Arg]) -> Option<Result<f64>> {
            None
        }
        fn salt(&self) -> u32 {
            1
        }
    }

    fn p(v: Value) -> Prop {
        Prop::parse(&v).unwrap()
    }

    #[test]
    fn keys_hold_and_ease_into_the_key() {
        let k = p(json!([[1, 0], [2, 100, "power2.in"]]));
        assert_eq!(k.base(0.0), 0.0);
        assert_eq!(k.base(1.5), 12.5);
        assert_eq!(k.base(9.0), 100.0);
    }

    #[test]
    fn tween_and_expression_on_top() {
        let t = p(json!({"from": 0, "to": 10, "at": 1, "dur": 2, "ease": "none"}));
        assert_eq!(t.base(2.0), 5.0);
        let e = p(json!({"value": [[0, 10], [1, 20]], "expr": "value * 2"}));
        assert_eq!(e.at(1.0, &E).unwrap(), 40.0);
        assert_eq!(p(json!("time * 4")).at(0.0, &E).unwrap(), 2.0);
    }

    #[test]
    fn loops_cycle_pingpong_and_offset() {
        let c = p(json!({"keys": [[0, 0], [1, 10, "none"]], "loop": "cycle"}));
        assert_eq!(c.base(2.25), 2.5);
        let pp = p(json!({"keys": [[0, 0], [1, 10, "none"]], "loop": "pingpong"}));
        assert_eq!(pp.base(1.25), 7.5);
        let o = p(json!({"keys": [[0, 0], [1, 10, "none"]], "loop": "offset"}));
        assert_eq!(o.base(2.5), 25.0);
    }

    #[test]
    fn mistakes_are_specific() {
        let e = Prop::parse(&json!([[2, 0], [1, 5]])).unwrap_err().to_string();
        assert!(e.contains("time order"), "{e}");
        let e = Prop::parse(&json!({"frm": 1})).unwrap_err().to_string();
        assert!(e.contains("valid fields"), "{e}");
        assert!(Prop::parse(&json!([[0, 0, "wobbly.out"]])).is_err());
    }

    #[test]
    fn vectors_and_colours_animate() {
        let v = VecProp::parse(&json!({"from": [0, 0], "to": [100, 50], "at": 0, "dur": 1, "ease": "none"}), 2..=3).unwrap();
        assert_eq!(v.at(0.5, &E).unwrap(), vec![50.0, 25.0]);
        let v = VecProp::parse(&json!({"keys": [[0, [0, 0]], [1, [10, 20], "none"]]}), 2..=3).unwrap();
        assert_eq!(v.at(0.5, &E).unwrap(), vec![5.0, 10.0]);
        let c = ColorProp::parse(&json!({"from": "#000000", "to": "#ff0000", "dur": 1, "ease": "none"})).unwrap();
        assert_eq!(c.at(0.5, &E).unwrap()[0], 127.5);
        assert_eq!(parse_color_static(&json!("#f80")).unwrap(), [255.0, 136.0, 0.0, 1.0]);
    }

    #[test]
    fn misplaced_vector_and_tween_lists_explain_both_forms() {
        let e = format!("{:#}", VecProp::parse(&json!([[0, [0, 0]], [1, [10, 20]], [2, [5, 5]]]), 2..=3).unwrap_err());
        assert!(e.contains("\"keys\"") && e.contains("\"from\""), "{e}");
        let e = format!("{:#}", VecProp::parse(&json!([[0, [0, 0]], [1, [10, 20]]]), 2..=2).unwrap_err());
        assert!(e.contains("wrapper"), "{e}");
        let e = format!("{:#}", Prop::parse(&json!([{"from": 0, "to": 1, "dur": 1}, {"from": 1, "to": 0, "at": 1}])).unwrap_err());
        assert!(e.contains("tween object") && !e.contains("needs \"t\""), "{e}");
        let e = format!("{:#}", Prop::parse(&json!([[0, [1, 2]], [1, [3, 4]]])).unwrap_err());
        assert!(e.contains("one number") && e.contains("keys"), "{e}");
        // per-axis tweens stay valid
        let v = VecProp::parse(&json!([{"from": 0, "to": 100, "dur": 1, "ease": "none"}, 5]), 2..=2).unwrap();
        assert_eq!(v.at(0.5, &E).unwrap(), vec![50.0, 5.0]);
    }

    #[test]
    fn colour_keys_work_bare_or_inside_keys() {
        let bare = ColorProp::parse(&json!([[0, "#000000"], [1, "#ff0000", "none"]])).unwrap();
        let wrapped = ColorProp::parse(&json!({"keys": [[0, "#000000"], [1, "#ff0000", "none"]]})).unwrap();
        assert_eq!(bare.at(0.5, &E).unwrap(), wrapped.at(0.5, &E).unwrap());
        assert_eq!(wrapped.at(0.5, &E).unwrap()[0], 127.5);
        let e = ColorProp::parse(&json!({"colour": "#fff"})).unwrap_err().to_string();
        assert!(e.contains("keys"), "{e}");
    }
}
