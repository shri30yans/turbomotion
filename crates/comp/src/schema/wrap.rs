//! Serde wrappers that let the animated types (`Prop`, vectors, colours) sit directly in the layer structs, so one
//! `deny_unknown_fields` pass types and validates the whole composition.

use serde::{de::Error, Deserialize, Deserializer};
use serde_json::Value;

use crate::anim::prop::{ColorProp, Prop, VecProp};

/// A scalar animated property.
#[derive(Debug, Clone)]
pub struct P(pub Prop);

/// A 2- or 3-component animated vector (position, anchor).
#[derive(Debug, Clone)]
pub struct Pos(pub VecProp);

/// Scale as a fraction (1 = 100 %): a single property for uniform scale, or `[sx, sy, sz?]`.
#[derive(Debug, Clone)]
pub struct Scale(pub VecProp);

/// A colour that can animate.
#[derive(Debug, Clone)]
pub struct Col(pub ColorProp);

/// X, Y and Z Rotation: one number spins the layer about the z axis (the Z Rotation), three numbers are the X, Y
/// and Z Rotation. Each axis is its own property and may wind past 360 degrees. Three numbers can also be written as
/// one vector tween or one list of vector keys.
#[derive(Debug, Clone)]
pub struct Rot(pub [Prop; 3]);

/// X, Y and Z Orientation: `[x, y, z]`, each any property, or one vector tween / list of vector keys. The keys of an
/// orientation interpolate as a rotation (the short way round), see `eval::pose`.
#[derive(Debug, Clone)]
pub struct Orient(pub [Prop; 3]);

/// Three animated numbers: `[x, y, z]` (each any property), `{"from": [..], "to": [..]}` or `{"keys": [[t, [..]], ..]}`.
fn triple(v: &Value, what: &str) -> Result<[Prop; 3], String> {
    let parts = VecProp::parse(v, 3..=3).map_err(|e| format!("{what}: {e:#}"))?;
    Ok(parts.0.try_into().expect("VecProp::parse(3..=3) yields three"))
}

/// Is this object a tween or keys of whole vectors (as opposed to one scalar property)?
fn is_vector_object(v: &Value) -> bool {
    v.get("from").is_some_and(Value::is_array) || v.get("keys").and_then(Value::as_array).and_then(|k| k.first()).and_then(|k| k.get(1)).is_some_and(Value::is_array)
}

impl<'de> Deserialize<'de> for Rot {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        if v.is_array() || is_vector_object(&v) {
            return triple(&v, "rotation").map(Rot).map_err(D::Error::custom);
        }
        let p = Prop::parse(&v).map_err(|e| D::Error::custom(format!("{e:#}")))?;
        Ok(Rot([Prop::constant(0.0), Prop::constant(0.0), p]))
    }
}

impl<'de> Deserialize<'de> for Orient {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        if !(v.is_array() || is_vector_object(&v)) {
            return Err(D::Error::custom(format!("an orientation is [x, y, z] (or a vector tween); found {v}")));
        }
        triple(&v, "orientation").map(Orient).map_err(D::Error::custom)
    }
}

impl<'de> Deserialize<'de> for P {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        Prop::parse(&v).map(P).map_err(|e| D::Error::custom(format!("{e:#}")))
    }
}

impl<'de> Deserialize<'de> for Pos {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        VecProp::parse(&v, 2..=3).map(Pos).map_err(|e| D::Error::custom(format!("{e:#}")))
    }
}

impl<'de> Deserialize<'de> for Scale {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        // `[x, y]` is per axis, whether each part is a number, an expression or a tween of its own
        let per_axis = v.as_array().is_some_and(|a| (2..=3).contains(&a.len()) && a.iter().all(|x| x.is_number() || x.is_string() || x.is_object()));
        let r = if per_axis || v.get("from").is_some_and(Value::is_array) || v.get("keys").is_some_and(|k| k.as_array().and_then(|k| k.first()).and_then(|k| k.get(1)).is_some_and(Value::is_array)) {
            VecProp::parse(&v, 2..=3)
        } else {
            Prop::parse(&v).map(|p| VecProp(vec![p.clone(), p]))
        };
        r.map(Scale).map_err(|e| D::Error::custom(format!("{e:#}")))
    }
}

impl<'de> Deserialize<'de> for Col {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        ColorProp::parse(&v).map(Col).map_err(|e| D::Error::custom(format!("{e:#}")))
    }
}
