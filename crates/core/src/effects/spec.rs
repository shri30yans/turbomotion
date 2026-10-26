//! What a platform effect declares about each of its parameters, once: name, kind, default, range and what it does.
//! The effect's struct, the plan's validation, the descriptor the documentation is written from and `comp fx test`
//! all read this declaration, so none of them can disagree about it.

use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use super::param::Param;
use crate::{color::Rgb, suggest::hint};

/// What a parameter holds, and what it is when the plan leaves it out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamKind {
    /// A number, or `[[seconds, value], ...]` keys it is animated through.
    Float { default: f64 },
    /// `[r, g, b]`, each channel 0 to 255; with no default the colour is simply absent when left out.
    Color { default: Option<Rgb> },
}

#[derive(Debug, Clone, Copy)]
pub struct ParamSpec {
    pub name: &'static str,
    pub kind: ParamKind,
    /// Values outside are limited to the range, as a shader plugin's are.
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub doc: &'static str,
}

/// A number parameter.
pub const fn float(name: &'static str, default: f64, doc: &'static str) -> ParamSpec {
    ParamSpec { name, kind: ParamKind::Float { default }, min: None, max: None, doc }
}

/// A colour parameter.
pub const fn rgb(name: &'static str, default: Rgb, doc: &'static str) -> ParamSpec {
    ParamSpec { name, kind: ParamKind::Color { default: Some(default) }, min: None, max: None, doc }
}

/// A colour that is absent when left out.
pub const fn optional_rgb(name: &'static str, doc: &'static str) -> ParamSpec {
    ParamSpec { name, kind: ParamKind::Color { default: None }, min: None, max: None, doc }
}

impl ParamSpec {
    pub const fn at_least(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    pub const fn at_most(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    pub const fn between(self, min: f64, max: f64) -> Self {
        self.at_least(min).at_most(max)
    }

    /// The value to deserialise: what was written (checked, and limited to the range) or the default; none for an
    /// optional parameter that was left out.
    pub fn resolve(&self, given: Option<&Value>) -> Result<Option<Value>> {
        let Some(v) = given else {
            return Ok(match self.kind {
                ParamKind::Float { default } => Some(json!(default)),
                ParamKind::Color { default } => default.map(|rgb| json!(rgb)),
            });
        };
        Ok(Some(match self.kind {
            ParamKind::Float { .. } => {
                Param::deserialize(v).map_err(|e| anyhow!("{e}"))?;
                self.limited(v)
            }
            ParamKind::Color { .. } => {
                Rgb::deserialize(v).map_err(|_| anyhow!("a colour is [r, g, b] with each channel 0 to 255"))?;
                v.clone()
            }
        }))
    }

    /// A number or the values of `[seconds, value]` keys, held inside the range.
    fn limited(&self, v: &Value) -> Value {
        let limit = |x: &Value| x.as_f64().map_or(x.clone(), |x| json!(x.clamp(self.min.unwrap_or(f64::MIN), self.max.unwrap_or(f64::MAX))));
        match v {
            Value::Array(keys) => Value::Array(keys.iter().map(|k| Value::Array(k.as_array().into_iter().flatten().enumerate().map(|(i, x)| if i == 1 { limit(x) } else { x.clone() }).collect())).collect()),
            number => limit(number),
        }
    }
}

/// Every key written in a use of effect `effect` must be one of `names`; each stray one is named, with the nearest
/// valid name, and the whole list closes the message.
pub fn check_names<'a>(effect: &str, given: impl IntoIterator<Item = &'a String>, names: &[&str]) -> Result<()> {
    let strays: Vec<String> = given.into_iter().filter(|k| !names.contains(&k.as_str())).map(|k| format!("effect {effect:?} has no parameter {k:?}.{}", hint(k, names))).collect();
    if strays.is_empty() {
        return Ok(());
    }
    bail!("{} Its parameters are: {}", strays.join(" "), if names.is_empty() { "none".to_string() } else { names.join(", ") })
}

impl ParamKind {
    /// The forms a value of this kind is written in, for an error message.
    pub fn accepts(&self) -> &'static str {
        match self {
            ParamKind::Float { .. } => "a number, or [[seconds, value], ...] keys that animate it",
            ParamKind::Color { .. } => "[r, g, b] with each channel 0 to 255",
        }
    }
}

/// `given` checked against `specs` of effect `name`, with every parameter present or defaulted, as one JSON object that
/// carries `"effect": name`. Every wrong name and every wrong value is reported together, each with the parameter and
/// the forms it accepts.
pub fn resolve_all(name: &str, specs: &[ParamSpec], given: &Map<String, Value>) -> Result<Map<String, Value>> {
    let mut problems: Vec<String> = Vec::new();
    if let Err(e) = check_names(name, given.keys(), &specs.iter().map(|s| s.name).collect::<Vec<_>>()) {
        problems.push(e.to_string());
    }
    let mut out = Map::new();
    out.insert("effect".to_string(), json!(name));
    for spec in specs {
        match spec.resolve(given.get(spec.name)) {
            Ok(Some(v)) => {
                out.insert(spec.name.to_string(), v);
            }
            Ok(None) => {}
            Err(e) => problems.push(format!("effect {name:?} parameter {:?}: {e:#}; accepted: {}", spec.name, spec.kind.accepts())),
        }
    }
    match problems.as_slice() {
        [] => Ok(out),
        [one] => Err(anyhow!("{one}")),
        many => Err(anyhow!("effect {name:?} has {} problems:\n  - {}", many.len(), many.join("\n  - "))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RADIUS: ParamSpec = float("radius", 8.0, "reach").between(0.0, 100.0);
    const INK: ParamSpec = rgb("ink", [1, 2, 3], "colour");
    const GLOW: ParamSpec = optional_rgb("glow", "an optional colour");

    #[test]
    fn a_left_out_parameter_takes_its_default_and_an_optional_one_stays_absent() {
        assert_eq!(RADIUS.resolve(None).unwrap(), Some(json!(8.0)));
        assert_eq!(INK.resolve(None).unwrap(), Some(json!([1, 2, 3])));
        assert_eq!(GLOW.resolve(None).unwrap(), None);
    }

    #[test]
    fn numbers_and_every_key_value_are_held_inside_the_range() {
        assert_eq!(RADIUS.resolve(Some(&json!(250))).unwrap(), Some(json!(100.0)));
        assert_eq!(RADIUS.resolve(Some(&json!(40))).unwrap(), Some(json!(40.0)));
        assert_eq!(RADIUS.resolve(Some(&json!([[0, -5], [1, 500]]))).unwrap(), Some(json!([[0, 0.0], [1, 100.0]])));
    }

    #[test]
    fn a_value_of_the_wrong_shape_is_an_error() {
        assert!(RADIUS.resolve(Some(&json!("big"))).is_err());
        assert!(INK.resolve(Some(&json!([1, 2]))).unwrap_err().to_string().contains("[r, g, b]"));
        assert!(INK.resolve(Some(&json!([1, 2, 300]))).is_err());
    }

    #[test]
    fn every_wrong_name_and_value_is_reported_in_one_message() {
        let given = json!({"radis": 3, "ink": [1, 2]}).as_object().unwrap().clone();
        let e = resolve_all("blur", &[RADIUS, INK], &given).unwrap_err().to_string();
        assert!(e.contains("2 problems") && e.contains("\"radis\"") && e.contains("parameter \"ink\"") && e.contains("[r, g, b] with each channel"), "{e}");
        let given = json!({"radius": "big", "ink": "red"}).as_object().unwrap().clone();
        let e = resolve_all("blur", &[RADIUS, INK], &given).unwrap_err().to_string();
        assert!(e.contains("parameter \"radius\"") && e.contains("parameter \"ink\"") && e.contains("accepted: a number"), "{e}");
    }

    #[test]
    fn an_unknown_parameter_names_itself_and_lists_the_valid_ones() {
        let given = json!({"radis": 3}).as_object().unwrap().clone();
        let e = resolve_all("blur", &[RADIUS], &given).unwrap_err().to_string();
        assert!(e.contains("\"radis\"") && e.contains("Did you mean \"radius\"") && e.contains("Its parameters are: radius"), "{e}");
    }
}
