//! Reading a scalar property from JSON: a number, an expression, keys or a tween.

use anyhow::{anyhow, bail, ensure, Result};
use serde_json::Value;

use super::{
    parse_keys::Reader,
    track::{Options, Track},
    value::{Kind, Value as Val},
    Key, Loop, Prop,
};
use crate::anim::{ease::Ease, expr::Expr};

/// The keys of a scalar property as a track.
fn track(keys: Vec<Key>, looping: Loop) -> Result<Track> {
    Track::new(Kind::Scalar, keys, Options { looping, spatial: false })
}

fn scalar_keys(v: &Value) -> Result<Vec<Key>> {
    Reader { kind: Kind::Scalar, path: None }.keys(v)
}

/// The `loop` field of a property object.
pub(super) fn loop_of(o: &serde_json::Map<String, Value>) -> Result<Loop> {
    match o.get("loop").map(|l| l.as_str().unwrap_or("")) {
        None => Ok(Loop::None),
        Some("cycle") => Ok(Loop::Cycle),
        Some("pingpong") => Ok(Loop::PingPong),
        Some("offset") => Ok(Loop::Offset),
        Some(other) => bail!("\"loop\" must be cycle, pingpong or offset, not {other:?}"),
    }
}

impl Prop {
    pub fn parse(v: &Value) -> Result<Prop> {
        match v {
            Value::Number(n) => Ok(Prop::constant(n.as_f64().ok_or_else(|| anyhow!("{n} is not a finite number"))?)),
            Value::String(s) => Ok(Prop { expr: Some(Expr::parse(s)?), ..Prop::constant(0.0) }),
            Value::Array(_) if is_number_list(v) => bail!("this property is a scalar, found the list of numbers {v}: that is how a vector (pos, anchor, scale) is written. A scalar is {SCALAR_FORMS}"),
            Value::Array(_) => Ok(Prop { track: track(scalar_keys(v)?, Loop::None)?, expr: None }),
            Value::Object(o) => {
                const VALID: &[&str] = &["keys", "value", "from", "to", "at", "dur", "delay", "ease", "loop", "expr"];
                if let Some(bad) = o.keys().find(|k| !VALID.contains(&k.as_str())) {
                    bail!("unknown property field {bad:?}; valid fields are {}", VALID.join(", "));
                }
                let keys = if let Some(k) = o.get("keys") {
                    scalar_keys(k)?
                } else if let Some(v) = o.get("value") {
                    let inner = Prop::parse(v)?;
                    ensure!(inner.expr.is_none(), "\"value\" is the base the expression builds on; it cannot have its own expression");
                    inner.track.keys().to_vec()
                } else if o.contains_key("from") || o.contains_key("to") {
                    let (from, to) = (o.get("from"), o.get("to"));
                    let (from, to) = (num("\"from\"", from.ok_or_else(|| anyhow!("a tween needs \"from\" and \"to\""))?)?, num("\"to\"", to.ok_or_else(|| anyhow!("a tween needs \"from\" and \"to\""))?)?);
                    let at = o.get("at").map_or(Ok(0.0), |v| num("\"at\"", v))? + o.get("delay").map_or(Ok(0.0), |v| num("\"delay\"", v))?;
                    let dur = o.get("dur").map_or(Ok(0.5), |v| num("\"dur\"", v))?;
                    ensure!(dur >= 0.0, "\"dur\" must not be negative");
                    let ease = o.get("ease").map_or(Ok(Ease::default()), |e| Ease::parse(e.as_str().ok_or_else(|| anyhow!("\"ease\" must be text like \"expo.out\""))?))?;
                    vec![Key::new(at, Val::F(from)), Key::new(at + dur.max(1e-6), Val::F(to)).eased(ease)]
                } else if o.contains_key("expr") {
                    vec![Key::new(0.0, Val::F(0.0))]
                } else {
                    bail!("a property object needs \"keys\", \"value\", a \"from\"/\"to\" tween, or an \"expr\"");
                };
                let looping = loop_of(o)?;
                let expr = o.get("expr").map(|e| Expr::parse(e.as_str().ok_or_else(|| anyhow!("\"expr\" must be text"))?)).transpose()?;
                Ok(Prop { track: track(keys, looping)?, expr })
            }
            other => bail!("a property is a number, text (an expression), keys or a tween, not {other}"),
        }
    }
}

/// The ways to write a scalar property, for errors.
pub(super) const SCALAR_FORMS: &str = "a number, an expression (text like \"time * 90\"), keys [[seconds, value, ease?], ..] or a tween {\"from\": a, \"to\": b, \"at\", \"dur\", \"ease\"}";

/// `v` as the one number that `what` ("\"from\"", "a key's value") is. A list is named as a vector, because that is the mix-up.
pub(super) fn num(what: &str, v: &Value) -> Result<f64> {
    v.as_f64().ok_or_else(|| match v {
        Value::Array(_) => anyhow!("{what} must be one number, found the list {v}: a list is a vector (pos, anchor, scale), and this property is a scalar. A scalar is {SCALAR_FORMS}"),
        _ => anyhow!("{what} must be a number, found {v}"),
    })
}

/// A list of plain numbers: how a vector is written, never a scalar.
pub(super) fn is_number_list(v: &Value) -> bool {
    v.as_array().is_some_and(|a| !a.is_empty() && a.iter().all(Value::is_number))
}

/// The ways to write a vector, for errors.
pub(super) const VECTOR_FORMS: &str = "[x, y] (each part any property: [{\"from\": 1, \"to\": 2, \"dur\": 1}, 0.5]), {\"from\": [0, 0], \"to\": [100, 50], \"at\": 1, \"dur\": 0.5} or {\"keys\": [[0, [0, 0]], [1, [100, 50], \"expo.out\"]]}";
