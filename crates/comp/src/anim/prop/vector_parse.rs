//! Reading a vector property from JSON.

use std::ops::RangeInclusive;

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde_json::{Map, Value};
use turbomotion_core::suggest::hint;

use super::{
    key::Tangents,
    parse::{loop_of, VECTOR_FORMS},
    parse_keys::Reader,
    track::{Options, Track},
    value::Kind,
    Prop, VecProp,
};

/// `[[t, [x, y]], ..]` written without the `{"keys": ..}` wrapper.
fn looks_like_vector_keys(parts: &[Value]) -> bool {
    !parts.is_empty() && parts.iter().all(|p| p.as_array().is_some_and(|k| k.len() >= 2 && k[0].is_number() && k[1].is_array()))
}

/// The fields of a property object that keeps its keys as whole vectors.
const JOINED_FIELDS: &[&str] = &["keys", "loop", "spatial", "auto_orient"];

/// The value of a vector key, if the key is an object: the mark of the whole-vector form.
fn key_value(k: &Value) -> Option<&Value> {
    match k {
        Value::Array(a) => a.get(1),
        Value::Object(o) => o.get("v"),
        _ => None,
    }
}

/// Is this object keys of whole vectors with handles (the typed form), not a tween or per-dimension keys?
fn is_joined(o: &Map<String, Value>) -> bool {
    o.contains_key("spatial") || o.contains_key("auto_orient") || o.get("keys").and_then(Value::as_array).is_some_and(|k| k.iter().any(Value::is_object))
}

impl VecProp {
    /// Is this property object a vector (a tween between lists, keys of vectors, separate dimensions) rather than one
    /// scalar property?
    pub fn is_vector_object(v: &Value) -> bool {
        v.get("from").is_some_and(Value::is_array)
            || v.get("separate_dimensions").is_some()
            || v.get("keys").and_then(Value::as_array).and_then(|k| k.first()).and_then(key_value).is_some_and(Value::is_array)
    }

    /// `[x, y]` where each part is any property, or a tween / keys of whole vectors:
    /// `{"from": [0, 0], "to": [100, 50], "at": 1, "dur": 0.5}` and `{"keys": [[0, [0, 0]], [1, [100, 50], "expo.out"]]}`.
    /// Keys written as objects, `{"keys": [{"t": 0, "v": [0, 0], "ease_out": {"speed": 300, "influence": 50}}, ..]}`,
    /// are one track for the whole vector.
    pub fn parse(v: &Value, len: RangeInclusive<usize>) -> Result<VecProp> {
        VecProp::parse_as(v, len, false)
    }

    /// A position (or anchor, or aim point): whole-vector keys follow a motion path, `"spatial": "auto"` unless said
    /// otherwise, and their speeds are along it.
    pub fn parse_position(v: &Value, len: RangeInclusive<usize>) -> Result<VecProp> {
        VecProp::parse_as(v, len, true)
    }

    fn parse_as(v: &Value, len: RangeInclusive<usize>, position: bool) -> Result<VecProp> {
        let what = || format!("{}-{} numbers", len.start(), len.end());
        match v {
            Value::Array(parts) => {
                ensure!(!looks_like_vector_keys(parts), "keys of vectors need the {{\"keys\"}} wrapper. A vector is {VECTOR_FORMS}; found {v}");
                ensure!(len.contains(&parts.len()), "expected {} as {VECTOR_FORMS}; found {} items", what(), parts.len());
                Ok(VecProp::Axes(parts.iter().enumerate().map(|(i, p)| Prop::parse(p).with_context(|| format!("component {i}"))).collect::<Result<_>>()?))
            }
            Value::Object(o) if o.contains_key("separate_dimensions") => {
                ensure!(o.len() == 1, "\"separate_dimensions\" is the whole property: {{\"separate_dimensions\": [x, y, z]}}; it takes no other field");
                VecProp::parse_as(&o["separate_dimensions"], len, position).and_then(|p| match p {
                    VecProp::Axes(_) => Ok(p),
                    VecProp::Joined { .. } => bail!("\"separate_dimensions\" is a list of one property per dimension, like [x, y, z]"),
                })
            }
            Value::Object(o) if is_joined(o) => joined(o, &len, position),
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
                        let mut obj = Map::new();
                        obj.insert("keys".into(), Value::Array(part));
                        if let Some(l) = o.get("loop") {
                            obj.insert("loop".into(), l.clone());
                        }
                        comps.push(Prop::parse(&Value::Object(obj))?);
                    }
                    Ok(VecProp::Axes(comps))
                } else if let (Some(from), Some(to)) = (o.get("from").and_then(Value::as_array), o.get("to").and_then(Value::as_array)) {
                    ensure!(from.len() == to.len() && len.contains(&from.len()), "\"from\" and \"to\" must both have {}", what());
                    let mut comps = vec![];
                    for (f, t) in from.iter().zip(to) {
                        let mut obj = o.clone();
                        obj.insert("from".into(), f.clone());
                        obj.insert("to".into(), t.clone());
                        comps.push(Prop::parse(&Value::Object(obj))?);
                    }
                    Ok(VecProp::Axes(comps))
                } else if o.get("from").is_some_and(Value::is_array) != o.get("to").is_some_and(Value::is_array) {
                    bail!("a vector tween needs \"from\" and \"to\" both as lists of the same length; found from {} and to {} (a scalar tween has both as numbers). A vector is {VECTOR_FORMS}", o.get("from").unwrap_or(&Value::Null), o.get("to").unwrap_or(&Value::Null))
                } else {
                    bail!("a vector is {VECTOR_FORMS}; found {v}")
                }
            }
            other => bail!("expected a vector ({}), found {other}. A vector is {VECTOR_FORMS}", what()),
        }
    }
}

/// `{"keys": [{"t", "v", ..}, ..], "loop", "spatial", "auto_orient"}`: one track for the whole vector.
fn joined(o: &Map<String, Value>, len: &RangeInclusive<usize>, position: bool) -> Result<VecProp> {
    if let Some(bad) = o.keys().find(|k| !JOINED_FIELDS.contains(&k.as_str())) {
        bail!("unknown field {bad:?} on keys of whole vectors; they take {}.{}", JOINED_FIELDS.join(", "), hint(bad, JOINED_FIELDS));
    }
    let keys = o.get("keys").ok_or_else(|| anyhow!("keys of whole vectors need \"keys\""))?;
    let first = keys.as_array().and_then(|l| l.first()).ok_or_else(|| anyhow!("keys must be a non-empty list"))?;
    let n = key_value(first).and_then(Value::as_array).map(Vec::len).ok_or_else(|| anyhow!("every key of a vector holds [x, y] or [x, y, z] as \"v\"; a vector is {VECTOR_FORMS}"))?;
    ensure!(len.contains(&n), "expected {}-{} numbers, found {n}", len.start(), len.end());
    let kind = if n == 2 { Kind::Vec2 } else { Kind::Vec3 };
    let path = match o.get("spatial") {
        None => position.then_some(Tangents::Auto),
        Some(Value::String(s)) => Some(match s.as_str() {
            "auto" => Tangents::Auto,
            "linear" => Tangents::Linear,
            "bezier" => Tangents::Bezier { into: [0.0; 3], out: [0.0; 3] },
            other => bail!("a property's \"spatial\" is \"auto\", \"linear\" or \"bezier\" (the default of its keys), not {other:?}.{}", hint(other, &["auto", "linear", "bezier"])),
        }),
        Some(other) => bail!("\"spatial\" is \"auto\", \"linear\" or \"bezier\", not {other}"),
    };
    let auto_orient = match o.get("auto_orient") {
        None => false,
        Some(Value::String(s)) if s == "path" => true,
        Some(other) => bail!("\"auto_orient\" is \"path\" (turn the layer along its motion path), not {other}"),
    };
    ensure!(!auto_orient || path.is_some(), "\"auto_orient\" follows a motion path, and this property has none; it is for a position");
    let spatial = path.is_some();
    let track = Track::new(kind, Reader { kind, path }.keys(keys)?, Options { looping: loop_of(o)?, spatial })?;
    Ok(VecProp::Joined { track, auto_orient })
}
