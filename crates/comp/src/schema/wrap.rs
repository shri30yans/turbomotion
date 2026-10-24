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
        let all_numbers = v.as_array().is_some_and(|a| (2..=3).contains(&a.len()) && a.iter().all(|x| x.is_number()));
        let r = if all_numbers || v.get("from").is_some_and(Value::is_array) || v.get("keys").is_some_and(|k| k.as_array().and_then(|k| k.first()).and_then(|k| k.get(1)).is_some_and(Value::is_array)) {
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
